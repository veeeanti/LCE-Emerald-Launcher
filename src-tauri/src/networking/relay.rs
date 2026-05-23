use futures_util::{SinkExt, StreamExt};
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use crate::state::ProxyGuard;
use crate::util;
async fn run_relay_proxy(
    proxy_state: &ProxyGuard,
    ws_url: &str,
    auth_token: &str,
    cancel: CancellationToken,
) -> Result<u16, String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    eprintln!("[Emerald] Joiner relay: connecting WS...");
    let mut request = ws_url
        .into_client_request()
        .map_err(|e| format!("Failed to build WS request: {}", e))?;
    request.headers_mut().insert(
        http::header::AUTHORIZATION,
        format!("Bearer {}", auth_token)
            .parse()
            .map_err(|_| "Invalid auth header value".to_string())?,
    );
    request.headers_mut().insert(
        http::header::USER_AGENT,
        "MCLCE-LceLive/1.0"
            .parse()
            .map_err(|_| "Invalid UA header value".to_string())?,
    );

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("Relay WS connect failed: {}", e))?;
    eprintln!("[Emerald] Joiner relay: WS connected");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:61000")
        .await
        .map_err(|e| format!("Bind failed: {}", e))?;
    let local_port = listener.local_addr().map_err(|e| e.to_string())?.port();
    eprintln!("[Emerald] Joiner relay: bound on 0.0.0.0:{}", local_port);

    {
        let mut port = proxy_state.local_port.lock().await;
        *port = Some(local_port);
    }

    tokio::spawn(async move {
        eprintln!("[Emerald] Joiner relay: waiting for TCP accept on port {}...", local_port);
        let (tcp_stream, _) = tokio::select! {
            result = listener.accept() => {
                eprintln!("[Emerald] Joiner relay: TCP accepted");
                result.map_err(|e| format!("Accept failed: {}", e)).unwrap()
            },
            _ = cancel.cancelled() => {
                eprintln!("[Emerald] Joiner relay: cancelled before accept");
                return;
            },
        };

        eprintln!("[Emerald] Joiner relay: starting forwarders");
        let (tcp_read, tcp_write) = tcp_stream.into_split();
        let (ws_write, ws_read) = ws_stream.split();
        let cancel_ws = cancel.clone();
        let forward_tcp = tokio::spawn(async move {
            let mut ws_write = ws_write;
            let mut tcp_read = tcp_read;
            let mut buf = [0u8; 65536];
            loop {
                tokio::select! {
                    result = tcp_read.read(&mut buf) => {
                        match result {
                            Ok(0) => { eprintln!("[Emerald] Joiner relay: TCP→WS EOF"); break; },
                            Err(e) => { eprintln!("[Emerald] Joiner relay: TCP→WS read error: {e}"); break; },
                            Ok(n) => {
                                eprintln!("[Emerald] Joiner relay: TCP→WS forwarding {} bytes", n);
                                if ws_write.send(tokio_tungstenite::tungstenite::Message::Binary(buf[..n].to_vec())).await.is_err() {
                                    eprintln!("[Emerald] Joiner relay: TCP→WS send error"); break;
                                }
                            }
                        }
                    }
                    _ = cancel_ws.cancelled() => { eprintln!("[Emerald] Joiner relay: TCP→WS cancelled"); break; },
                }
            }
        });

        let cancel_tcp = cancel.clone();
        let forward_ws = tokio::spawn(async move {
            let ws_read = ws_read;
            let mut tcp_write = tcp_write;
            tokio::pin!(ws_read);
            loop {
                tokio::select! {
                    result = ws_read.next() => {
                        match result {
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                                eprintln!("[Emerald] Joiner relay: WS→TCP forwarding {} bytes", data.len());
                                if tcp_write.write_all(&data).await.is_err() { break; }
                            }
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => { break; }
                            None => { break; }
                            Some(Err(e)) => { eprintln!("[Emerald] Joiner relay: WS→TCP error: {e}"); break; }
                            _ => {}
                        }
                    }
                    _ = cancel_tcp.cancelled() => { break; },
                }
            }
        });

        tokio::select! {
            _ = forward_tcp => eprintln!("[Emerald] Joiner relay: forward_tcp done"),
            _ = forward_ws => eprintln!("[Emerald] Joiner relay: forward_ws done"),
            _ = cancel.cancelled() => eprintln!("[Emerald] Joiner relay: cancelled"),
        }
        eprintln!("[Emerald] Joiner relay: relay task ended");
    });

    Ok(local_port)
}

async fn run_host_relay(
    _proxy_state: &ProxyGuard,
    ws_url: &str,
    auth_token: &str,
    game_port: u16,
    cancel: CancellationToken,
) -> Result<(), String> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    eprintln!("[Emerald] Host relay: connecting WS...");
    let mut request = ws_url
        .into_client_request()
        .map_err(|e| format!("Failed to build WS request: {}", e))?;
    request.headers_mut().insert(
        http::header::AUTHORIZATION,
        format!("Bearer {}", auth_token)
            .parse()
            .map_err(|_| "Invalid auth header value".to_string())?,
    );
    request.headers_mut().insert(
        http::header::USER_AGENT,
        "MCLCE-LceLive/1.0"
            .parse()
            .map_err(|_| "Invalid UA header value".to_string())?,
    );

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("Host relay WS connect failed: {}", e))?;
    eprintln!("[Emerald] Host relay: WS connected");
    eprintln!("[Emerald] Host relay: connecting to game 127.0.0.1:{}...", game_port);
    let game_stream = loop {
        match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", game_port)).await {
            Ok(stream) => { eprintln!("[Emerald] Host relay: connected to game"); break stream; },
            Err(e) => {
                eprintln!("[Emerald] Host relay: game connect failed (retrying): {e}");
                tokio::select! {
                    _ = cancel.cancelled() => return Err("Host relay cancelled".into()),
                    _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                }
            }
        }
    };

    eprintln!("[Emerald] Host relay: starting forwarders");
    let (game_read, game_write) = game_stream.into_split();
    let (ws_write, ws_read) = ws_stream.split();
    let cancel_ws = cancel.clone();
    let forward_game = tokio::spawn(async move {
        let mut ws_write = ws_write;
        let mut game_read = game_read;
        let mut buf = [0u8; 65536];
        loop {
            tokio::select! {
                result = game_read.read(&mut buf) => {
                    match result {
                        Ok(0) => { eprintln!("[Emerald] Host relay: game→WS EOF"); break; },
                        Err(e) => { eprintln!("[Emerald] Host relay: game→WS read error: {e}"); break; },
                        Ok(n) => {
                            eprintln!("[Emerald] Host relay: game→WS forwarding {} bytes", n);
                            if ws_write.send(tokio_tungstenite::tungstenite::Message::Binary(buf[..n].to_vec())).await.is_err() { break; }
                        }
                    }
                }
                _ = cancel_ws.cancelled() => { break; },
            }
        }
    });

    let cancel_ws2 = cancel.clone();
    let forward_ws = tokio::spawn(async move {
        let ws_read = ws_read;
        let mut game_write = game_write;
        tokio::pin!(ws_read);
        loop {
            tokio::select! {
                result = ws_read.next() => {
                    match result {
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                            eprintln!("[Emerald] Host relay: WS→game forwarding {} bytes", data.len());
                            if game_write.write_all(&data).await.is_err() { break; }
                        }
                        Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => { break; }
                        None => { break; }
                        Some(Err(e)) => { eprintln!("[Emerald] Host relay: WS→game error: {e}"); break; }
                        _ => {}
                    }
                }
                _ = cancel_ws2.cancelled() => { break; },
            }
        }
    });

    tokio::select! {
        _ = forward_game => eprintln!("[Emerald] Host relay: forward_game done"),
        _ = forward_ws => eprintln!("[Emerald] Host relay: forward_ws done"),
        _ = cancel.cancelled() => eprintln!("[Emerald] Host relay: cancelled"),
    }

    Ok(())
}

#[tauri::command]
pub async fn start_relay_proxy(
    proxy_state: State<'_, ProxyGuard>,
    api_base_url: String,
    access_token: String,
    session_id: String,
) -> Result<u16, String> {
    let ws_base = util::ws_base_url(&api_base_url);
    let ws_url = format!("{}/api/relay/ws?sessionId={}&role=joiner", ws_base, session_id);
    let cancel = CancellationToken::new();
    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.insert(session_id.clone(), cancel.clone());
    }

    let local_port = run_relay_proxy(&proxy_state, &ws_url, &access_token, cancel).await?;

    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.remove(&session_id);
    }

    Ok(local_port)
}

#[tauri::command]
pub async fn start_host_relay(
    proxy_state: State<'_, ProxyGuard>,
    api_base_url: String,
    access_token: String,
    session_id: String,
    game_port: u16,
) -> Result<(), String> {
    let ws_base = util::ws_base_url(&api_base_url);
    let ws_url = format!("{}/api/relay/ws?sessionId={}&role=host", ws_base, session_id);
    let cancel = CancellationToken::new();
    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.insert(session_id.clone(), cancel.clone());
    }

    let result = run_host_relay(&proxy_state, &ws_url, &access_token, game_port, cancel).await;

    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.remove(&session_id);
    }

    result
}

#[tauri::command]
pub async fn stop_proxy(proxy_state: State<'_, ProxyGuard>, session_id: String) -> Result<(), String> {
    let mut tokens = proxy_state.cancel_tokens.lock().await;
    if let Some(token) = tokens.remove(&session_id) {
        token.cancel();
    }
    let mut port = proxy_state.local_port.lock().await;
    *port = None;
    Ok(())
}

#[tauri::command]
pub async fn stop_all_proxies(proxy_state: State<'_, ProxyGuard>) -> Result<(), String> {
    let mut tokens = proxy_state.cancel_tokens.lock().await;
    for (_, token) in tokens.drain() {
        token.cancel();
    }
    let mut port = proxy_state.local_port.lock().await;
    *port = None;
    Ok(())
}

#[tauri::command]
pub async fn join_game(
    app: tauri::AppHandle,
    game_state: State<'_, crate::state::GameState>,
    _proxy_state: State<'_, ProxyGuard>,
    _api_base_url: String,
    _access_token: String,
    host_ip: String,
    host_port: u16,
    _session_id: String,
    instance_id: String,
) -> Result<(), String> {
    let server = crate::types::McServer {
        name: host_ip.clone(),
        ip: host_ip,
        port: host_port,
    };
    crate::commands::game::launch_game(app, game_state, instance_id, vec![server], vec![]).await
}
