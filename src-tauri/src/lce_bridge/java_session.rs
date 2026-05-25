use crate::lce_bridge::java_protocol::*;
use crate::lce_bridge::config::RemoteConfig;
use crate::lce_bridge::msa_auth;
use crate::lce_bridge::util::VarInt;
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use aes::Aes128;
use cipher::{KeyInit, BlockEncrypt};

pub struct EncryptionCtx {
    pub key: [u8; 16],
    pub enc_iv: [u8; 16],
    pub dec_iv: [u8; 16],
}

fn aes_cfb8_encrypt(key: &[u8; 16], iv: &mut [u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).unwrap();
    let mut result = data.to_vec();
    for i in 0..result.len() {
        let mut block = *iv;
        cipher.encrypt_block((&mut block).into());
        let keystream = block[0];
        let ct = result[i] ^ keystream;
        result[i] = ct;
        iv.copy_within(1.., 0);
        iv[15] = ct;
    }
    result
}

fn aes_cfb8_decrypt(key: &[u8; 16], iv: &mut [u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).unwrap();
    let mut result = Vec::with_capacity(data.len());
    for &byte in data {
        let mut block = *iv;
        cipher.encrypt_block((&mut block).into());
        let keystream = block[0];
        let pt = byte ^ keystream;
        result.push(pt);
        iv.copy_within(1.., 0);
        iv[15] = byte;
    }
    result
}

fn rsa_encrypt(public_key_der: &[u8], data: &[u8]) -> std::io::Result<Vec<u8>> {
    use rsa::RsaPublicKey;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::Pkcs1v15Encrypt;
    let pubkey = RsaPublicKey::from_public_key_der(public_key_der)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    let mut rng = rand::thread_rng();
    pubkey.encrypt(&mut rng, Pkcs1v15Encrypt, data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

pub struct JavaSession {
    pub tx: mpsc::Sender<JavaPacket>,
    pub rx: mpsc::Receiver<JavaPacket>,
}

impl JavaSession {
    pub async fn connect(config: &RemoteConfig) -> std::io::Result<Self> {
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            TcpStream::connect((config.host.as_str(), config.port)),
        ).await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "connect timeout"))?
            .map_err(|e| e)?;
        let (reader, writer) = stream.into_split();
        let (packet_tx, packet_rx) = mpsc::channel::<JavaPacket>(256);
        let (write_tx, write_rx) = mpsc::channel::<JavaPacket>(256);

        let enc_ctx: Arc<Mutex<Option<EncryptionCtx>>> = Arc::new(Mutex::new(None));

        if config.auth_type == "online" {
            let profile = msa_auth::authenticate().await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            eprintln!("[LCEBridge] Authenticated as {} ({})", profile.username, profile.uuid);
        }

        let enc_w = enc_ctx.clone();
        let write_tx_clone = write_tx.clone();
        let comp_state: Arc<Mutex<i32>> = Arc::new(Mutex::new(-1i32));
        tokio::spawn(write_loop(writer, write_rx, enc_w, comp_state.clone()));

        tokio::spawn(read_loop(reader, packet_tx, write_tx_clone, enc_ctx, comp_state, config.clone()));

        Ok(Self {
            tx: write_tx,
            rx: packet_rx,
        })
    }

    pub async fn send(&mut self, packet: JavaPacket) -> std::io::Result<()> {
        self.tx.send(packet).await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::ConnectionReset, "send failed"))
    }

    pub async fn recv(&mut self) -> Option<JavaPacket> {
        self.rx.recv().await
    }
}

fn encode_frame(packet: &JavaPacket, compression: i32) -> Option<Vec<u8>> {
    let mut payload = Vec::new();
    packet.encode(&mut payload).ok()?;
    let packet_id = packet.id();
    let mut frame = VarInt::write(packet_id);
    frame.extend_from_slice(&payload);
    if compression >= 0 {
        let raw_data_len = frame.len() as i32;
        let mut compressed_frame = Vec::new();
        if raw_data_len > compression {
            use flate2::write::ZlibEncoder;
            use flate2::Compression;
            use std::io::Write;
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&frame).ok()?;
            let compressed = encoder.finish().ok()?;
            VarInt::write_to(raw_data_len, &mut compressed_frame);
            compressed_frame.extend_from_slice(&compressed);
        } else {
            VarInt::write_to(0, &mut compressed_frame);
            compressed_frame.extend_from_slice(&frame);
        }
        frame = compressed_frame;
    }
    let mut len_prefix = VarInt::write(frame.len() as i32);
    len_prefix.extend_from_slice(&frame);
    Some(len_prefix)
}

async fn read_loop(
    mut reader: tokio::net::tcp::OwnedReadHalf,
    tx: mpsc::Sender<JavaPacket>,
    write_tx: mpsc::Sender<JavaPacket>,
    enc_ctx: Arc<Mutex<Option<EncryptionCtx>>>,
    comp_state: Arc<Mutex<i32>>,
    config: RemoteConfig,
) {
    let mut state: u8 = 1;
    let mut compression = -1i32;
    let sent_finish_config = Arc::new(AtomicBool::new(false));
    loop {
        let mut raw = Vec::new();
        let mut dec = Vec::new();
        loop {
            let b = match read_raw_bytes(&mut reader, 1).await {
                Ok(b) => b[0],
                Err(e) => {
                    eprintln!("[LCEBridge] Java read loop error reading byte: {}", e);
                    break;
                }
            };
            raw.push(b);
            let db = {
                let mut ctx = enc_ctx.lock().await;
                if let Some(ref mut ec) = *ctx {
                    let result = aes_cfb8_decrypt(&ec.key, &mut ec.dec_iv, &[b]);
                    result[0]
                } else {
                    b
                }
            };
            dec.push(db);
            if dec.len() == 1 {
                eprintln!("[LCEBridge] Java read loop: first byte 0x{:02x} decrypted 0x{:02x}", b, db);
            }
            if db & 0x80 == 0 || dec.len() >= 5 {
                break;
            }
        }
        eprintln!("[LCEBridge] Java read: VarInt raw={:02x?} dec={:02x?}", raw, dec);
        if raw.is_empty() { break; }

        let len = match VarInt::read(&mut &dec[..]) {
            Ok(l) => {
                eprintln!("[LCEBridge] Java read: decoded packet length={}", l);
                l
            }
            Err(e) => {
                eprintln!("[LCEBridge] Java read: VarInt failed on first byte 0x{:02x}: {}", raw[0], e);
                if let Ok(()) = try_handshake(raw[0], &mut reader, &write_tx, &enc_ctx, &config).await {
                    continue;
                }
                eprintln!("[LCEBridge] Java read: try_handshake failed (offline?), breaking read loop");
                break;
            }
        };

        let mut packet_data = vec![0u8; len as usize];
        if read_exact_decrypt(&mut reader, &mut packet_data, &enc_ctx).await.is_err() { break; }
        if len > 0 {
            let show = &packet_data[..packet_data.len().min(20)];
            eprintln!("[LCEBridge] Java read: packet_data len={} hex={:02x?}", len, show);
        }

        let decompressed = if compression >= 0 {
            let mut data = &packet_data[..];
            let uncompressed_len = match VarInt::read(&mut data) {
                Ok(l) => l,
                Err(_) => break,
            };
            if uncompressed_len > 0 {
                use flate2::read::ZlibDecoder;
                use std::io::Read;
                let mut decoder = ZlibDecoder::new(data);
                let mut buf = Vec::with_capacity(uncompressed_len as usize);
                if decoder.read_to_end(&mut buf).is_err() { break; }
                buf
            } else {
                data.to_vec()
            }
        } else {
            packet_data
        };

        let mut data = &decompressed[..];
        if data.is_empty() { continue; }
        let packet_id = match VarInt::read(&mut data) {
            Ok(id) => id,
            Err(_) => break,
        };

        let proto_state = match state {
            0 => JavaProtocolState::Handshaking,
            1 => JavaProtocolState::Login,
            2 => JavaProtocolState::Config,
            3 => JavaProtocolState::Play,
            _ => JavaProtocolState::Play,
        };

        let packet = match JavaPacket::decode(packet_id, data, &proto_state) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[LCEBridge] Java read: decode failed for id=0x{:02x} state={:?}: {}", packet_id, proto_state, e);
                continue;
            }
        };

        match &packet {
            JavaPacket::SetCompression(p) => {
                eprintln!("[LCEBridge] Java read: SetCompression threshold={}", p.threshold);
                compression = p.threshold;
                *comp_state.lock().await = p.threshold;
                let _ = tx.send(packet).await;
                continue;
            }
            JavaPacket::LoginSuccess(p) => {
                eprintln!("[LCEBridge] Java read: LoginSuccess user={}", p.username);
                let _ = write_tx.send(JavaPacket::LoginAcknowledged).await;
                let _ = tx.send(packet).await;
                state = 2;
                continue;
            }
            JavaPacket::FinishConfig => {
                eprintln!("[LCEBridge] Java read: FinishConfig from server, sending ours");
                let _ = write_tx.send(JavaPacket::FinishConfig).await;
                let _ = tx.send(packet).await;
                state = 3;
                continue;
            }
            JavaPacket::Unknown { id: 0x0E, .. } if state == 2 && config.protocol_version >= 766 => {
                eprintln!("[LCEBridge] Java read: SelectKnownPacks (0x0E), sending empty response");
                let _ = write_tx.send(JavaPacket::Unknown { id: 0x07, data: vec![0] }).await;
                continue;
            }
            JavaPacket::Unknown { id: 0x07, .. } if state == 2 && config.protocol_version < 766 => {
                eprintln!("[LCEBridge] Java read: SelectKnownPacks (0x07), sending empty response");
                let _ = write_tx.send(JavaPacket::Unknown { id: 0x08, data: vec![0] }).await;
                continue;
            }
            JavaPacket::EncryptionRequest(p) => {
                handle_encryption_request(p, &write_tx, &enc_ctx).await;
                continue;
            }
            JavaPacket::Unknown { id: 0x09, data } if state == 2 => {
                eprintln!("[LCEBridge] Java read: ResourcePackPush, auto-accepting");
                let pack_id = if data.len() >= 16 {
                    data[..16].to_vec()
                } else {
                    data.clone()
                };
                let sid: i32 = if config.protocol_version >= 766 { 0x06 } else { 0x0C };
                for status in &[3, 4, 0] {
                    let mut resp = pack_id.clone();
                    VarInt::write_to(*status, &mut resp);
                    let _ = write_tx.send(JavaPacket::Unknown { id: sid, data: resp }).await;
                }
                continue;
            }
            JavaPacket::Unknown { id: 0x04, data } if state == 2 && data.len() == 8 => {
                eprintln!("[LCEBridge] Java read: KeepAlive (Config 0x04), echoing");
                let _ = write_tx.send(JavaPacket::Unknown { id: 0x04, data: data.clone() }).await;
                continue;
            }
            JavaPacket::Unknown { id: 0x04, data } if state == 2 && data.len() == 4 => {
                eprintln!("[LCEBridge] Java read: Ping (Config 0x04), sending Pong");
                let _ = write_tx.send(JavaPacket::Unknown { id: 0x05, data: data.clone() }).await;
                continue;
            }
            _ => {}
        }
        if state == 2 && !sent_finish_config.swap(true, Ordering::Relaxed) {
            eprintln!("[LCEBridge] Java read: first server config, waiting for FinishConfig from server");
        }
        eprintln!("[LCEBridge] Java read: forwarding packet id={}", packet.id());
        if tx.send(packet).await.is_err() { break; }
    }
}

async fn read_raw_bytes(reader: &mut tokio::net::tcp::OwnedReadHalf, n: usize) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn read_exact_decrypt(reader: &mut tokio::net::tcp::OwnedReadHalf, buf: &mut [u8], enc_ctx: &Arc<Mutex<Option<EncryptionCtx>>>) -> std::io::Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        let n = reader.read(&mut buf[offset..]).await?;
        if n == 0 { return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof")); }
        offset += n;
    }
    let mut ctx = enc_ctx.lock().await;
    if let Some(ref mut ec) = *ctx {
        let decrypted = aes_cfb8_decrypt(&ec.key, &mut ec.dec_iv, buf);
        buf.copy_from_slice(&decrypted);
    }
    Ok(())
}

async fn try_handshake(
    first_byte: u8,
    reader: &mut tokio::net::tcp::OwnedReadHalf,
    write_tx: &mpsc::Sender<JavaPacket>,
    enc_ctx: &Arc<Mutex<Option<EncryptionCtx>>>,
    config: &RemoteConfig,
) -> std::io::Result<()> {
    if config.auth_type != "online" {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "not online"));
    }
    let mut more = [0u8; 2];
    reader.read_exact(&mut more).await?;
    let mut full = vec![first_byte];
    full.extend_from_slice(&more);
    let len = VarInt::read(&mut &full[..]).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad varint"))?;
    let mut packet_data = vec![0u8; len as usize];
    reader.read_exact(&mut packet_data).await?;
    let mut rest = &packet_data[..];
    let _sid = read_utf8_string(&mut rest)?;
    let key_len = VarInt::read(&mut rest)? as usize;
    let mut public_key = vec![0u8; key_len];
    std::io::Read::read_exact(&mut rest, &mut public_key)?;
    let token_len = VarInt::read(&mut rest)? as usize;
    let mut verify_token = vec![0u8; token_len];
    std::io::Read::read_exact(&mut rest, &mut verify_token)?;

    let shared_secret: [u8; 16] = rand::random();
    let encrypted_secret = rsa_encrypt(&public_key, &shared_secret)?;
    let encrypted_token = {
        let mut iv = shared_secret;
        aes_cfb8_encrypt(&shared_secret, &mut iv, &verify_token)
    };

    let response = JavaPacket::EncryptionResponse(EncryptionResponsePacket {
        shared_secret: encrypted_secret,
        verify_token: encrypted_token,
    });
    let _ = write_tx.send(response).await;

    *enc_ctx.lock().await = Some(EncryptionCtx {
        key: shared_secret,
        enc_iv: shared_secret,
        dec_iv: shared_secret,
    });
    Ok(())
}

async fn handle_encryption_request(
    p: &EncryptionRequestPacket,
    write_tx: &mpsc::Sender<JavaPacket>,
    enc_ctx: &Arc<Mutex<Option<EncryptionCtx>>>,
) {
    let shared_secret: [u8; 16] = rand::random();
    let encrypted_secret = match rsa_encrypt(&p.public_key, &shared_secret) {
        Ok(s) => s,
        Err(_) => return,
    };
    let encrypted_token = {
        let mut iv = shared_secret;
        aes_cfb8_encrypt(&shared_secret, &mut iv, &p.verify_token)
    };
    let response = JavaPacket::EncryptionResponse(EncryptionResponsePacket {
        shared_secret: encrypted_secret,
        verify_token: encrypted_token,
    });
    let _ = write_tx.send(response).await;
    *enc_ctx.lock().await = Some(EncryptionCtx {
        key: shared_secret,
        enc_iv: shared_secret,
        dec_iv: shared_secret,
    });
}

async fn write_loop(
    mut writer: tokio::net::tcp::OwnedWriteHalf,
    mut rx: mpsc::Receiver<JavaPacket>,
    enc_ctx: Arc<Mutex<Option<EncryptionCtx>>>,
    comp_state: Arc<Mutex<i32>>,
) {
    while let Some(packet) = rx.recv().await {
        let threshold = *comp_state.lock().await;
        if let Some(mut frame) = encode_frame(&packet, threshold) {
            let needs_encryption = !matches!(packet, JavaPacket::EncryptionResponse(_));
            if needs_encryption {
                let mut ctx = enc_ctx.lock().await;
                if let Some(ref mut ec) = *ctx {
                    let encrypted = aes_cfb8_encrypt(&ec.key, &mut ec.enc_iv, &frame);
                    frame = encrypted;
                }
            }
            let hex: String = frame.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
            eprintln!("[LCEBridge] Java write: sending len={} hex=[{}]", frame.len(), hex);
            if writer.write_all(&frame).await.is_err() { break; }
        }
    }
    let _ = writer.shutdown().await;
}

fn read_utf8_string(r: &mut &[u8]) -> std::io::Result<String> {
    let len = VarInt::read(r)? as usize;
    if len == 0 { return Ok(String::new()); }
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(r, &mut buf)?;
    Ok(String::from_utf8(buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
}
