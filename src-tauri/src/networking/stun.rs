use crate::types::P2pEndpoint;
async fn stun_discover_impl() -> Result<P2pEndpoint, String> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    let stun_addr = tokio::net::lookup_host("stun.l.google.com:19302").await
        .map_err(|e| format!("STUN DNS lookup failed: {}", e))?
        .next()
        .ok_or_else(|| "STUN DNS returned no addresses".to_string())?;

    let magic_cookie: u32 = 0x2112A442;
    let mut trans_id = [0u8; 12];
    rand::Rng::fill(&mut rand::thread_rng(), &mut trans_id);
    let mut req = Vec::with_capacity(20);
    req.extend_from_slice(&0x0001u16.to_be_bytes());
    req.extend_from_slice(&0x0000u16.to_be_bytes());
    req.extend_from_slice(&magic_cookie.to_be_bytes());
    req.extend_from_slice(&trans_id);
    socket.send_to(&req, stun_addr).await.map_err(|e| format!("STUN send: {}", e))?;
    let mut buf = [0u8; 512];
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        socket.recv_from(&mut buf)
    ).await.map_err(|_| "STUN request timed out (5s)".to_string())?
     .map_err(|e| format!("STUN recv: {}", e))?;
    let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
    let rcvd_cookie = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if msg_type != 0x0101 || rcvd_cookie != magic_cookie {
        return Err(format!("Invalid STUN response (type=0x{:04X}, cookie=0x{:08X})", msg_type, rcvd_cookie));
    }

    let mut pos = 20;
    while pos + 4 <= buf.len() {
        let attr_type = u16::from_be_bytes([buf[pos], buf[pos+1]]);
        let attr_len = u16::from_be_bytes([buf[pos+2], buf[pos+3]]) as usize;
        pos += 4;
        if attr_type == 0x0020 && attr_len >= 8 && pos + 8 <= buf.len() {
            let _family = buf[pos+1];
            let xport = u16::from_be_bytes([buf[pos+2], buf[pos+3]]);
            let port = xport ^ (magic_cookie >> 16) as u16;
            let ip_bytes = [
                buf[pos+4] ^ (magic_cookie >> 24) as u8,
                buf[pos+5] ^ (magic_cookie >> 16) as u8,
                buf[pos+6] ^ (magic_cookie >> 8) as u8,
                buf[pos+7] ^ magic_cookie as u8,
            ];
            return Ok(P2pEndpoint {
                ip: format!("{}.{}.{}.{}", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]),
                port,
            });
        }
        pos += attr_len;
    }

    Err("No XOR-MAPPED-ADDRESS in STUN response".into())
}

#[tauri::command]
pub async fn stun_discover() -> Result<P2pEndpoint, String> {
    stun_discover_impl().await
}
