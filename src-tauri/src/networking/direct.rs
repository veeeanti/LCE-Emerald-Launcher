use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use crate::state::ProxyGuard;
async fn run_direct_proxy(
    proxy_state: &ProxyGuard,
    target_ip: &str,
    target_port: u16,
    cancel: CancellationToken,
) -> Result<u16, String> {
    let remote = tokio::net::TcpStream::connect(format!("{}:{}", target_ip, target_port))
        .await
        .map_err(|e| format!("Direct TCP connect failed: {}", e))?;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Bind failed: {}", e))?;
    let local_port = listener.local_addr().map_err(|e| e.to_string())?.port();
    {
        let mut port = proxy_state.local_port.lock().await;
        *port = Some(local_port);
    }

    let (local_stream, _) = tokio::select! {
        result = listener.accept() => result.map_err(|e| format!("Accept failed: {}", e))?,
        _ = cancel.cancelled() => return Err("Proxy cancelled".into()),
    };

    let (mut a_read, mut a_write) = remote.into_split();
    let (mut b_read, mut b_write) = local_stream.into_split();
    let cancel_a = cancel.clone();
    let task_a = tokio::spawn(async move {
        let mut buf = [0u8; 65536];
        loop {
            tokio::select! {
                result = a_read.read(&mut buf) => {
                    match result {
                        Ok(0) | Err(_) => break,
                        Ok(n) => { if b_write.write_all(&buf[..n]).await.is_err() { break; } }
                    }
                }
                _ = cancel_a.cancelled() => break,
            }
        }
    });

    let cancel_b = cancel.clone();
    let task_b = tokio::spawn(async move {
        let mut buf = [0u8; 65536];
        loop {
            tokio::select! {
                result = b_read.read(&mut buf) => {
                    match result {
                        Ok(0) | Err(_) => break,
                        Ok(n) => { if a_write.write_all(&buf[..n]).await.is_err() { break; } }
                    }
                }
                _ = cancel_b.cancelled() => break,
            }
        }
    });

    tokio::select! {
        _ = task_a => {},
        _ = task_b => {},
        _ = cancel.cancelled() => {},
    }

    Ok(local_port)
}

#[tauri::command]
pub async fn start_direct_proxy(
    proxy_state: State<'_, ProxyGuard>,
    target_ip: String,
    target_port: u16,
) -> Result<u16, String> {
    let cancel = CancellationToken::new();
    let session_id = "__direct__".to_string();
    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.insert(session_id.clone(), cancel.clone());
    }

    let local_port = run_direct_proxy(&proxy_state, &target_ip, target_port, cancel).await?;

    {
        let mut tokens = proxy_state.cancel_tokens.lock().await;
        tokens.remove(&session_id);
    }

    Ok(local_port)
}
