use base64::Engine;
use p256::ecdsa::{SigningKey, Signature, signature::Signer};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::SecretKey;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

const CLIENT_ID: &str = "00000000402b5328";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const CACHE_FILE: &str = "msa_cache.json";

static DEVICE_CODE_TX: OnceLock<broadcast::Sender<String>> = OnceLock::new();

fn device_code_tx() -> broadcast::Sender<String> {
    DEVICE_CODE_TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(8);
        tx
    }).clone()
}

pub fn subscribe_device_code() -> broadcast::Receiver<String> {
    device_code_tx().subscribe()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfile {
    pub uuid: String,
    pub username: String,
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenCache {
    access_token: String,
    refresh_token: Option<String>,
    mc_uuid: Option<String>,
    mc_username: Option<String>,
    mc_access_token: Option<String>,
    expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenError {
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DeviceAuthResponse {
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McLoginResponse {
    access_token: String,
    username: String,
    role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
}

struct AuthKey {
    signing: SigningKey,
    verifying: p256::PublicKey,
}

static AUTH_KEY: OnceLock<AuthKey> = OnceLock::new();

fn auth_key() -> &'static AuthKey {
    AUTH_KEY.get_or_init(|| {
        let secret = SecretKey::random(&mut rand::rngs::OsRng);
        let verifying = secret.public_key();
        let signing = secret.into();
        AuthKey { signing, verifying }
    })
}

fn windows_timestamp() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    ((secs + 11644473600) * 10_000_000) as i64
}

fn build_signature(method: &str, path: &str, body: &[u8]) -> String {
    let key = auth_key();
    let ts = windows_timestamp();

    let mut content = Vec::new();
    content.extend_from_slice(&1i32.to_be_bytes());
    content.push(0);
    content.extend_from_slice(&ts.to_be_bytes());
    content.push(0);
    content.extend_from_slice(method.as_bytes());
    content.push(0);
    content.extend_from_slice(path.as_bytes());
    content.push(0);
    content.push(0);
    content.extend_from_slice(body);
    content.push(0);

    let sig: Signature = key.signing.sign(&content);
    let sig_bytes = sig.to_bytes();

    let mut header = Vec::new();
    header.extend_from_slice(&1i32.to_be_bytes());
    header.extend_from_slice(&ts.to_be_bytes());
    header.extend_from_slice(&sig_bytes[..]);

    base64::engine::general_purpose::STANDARD.encode(&header)
}

fn proof_key_value() -> serde_json::Value {
    let key = auth_key();
    let encoded = key.verifying.to_encoded_point(false);
    serde_json::json!({
        "kty": "EC",
        "alg": "ES256",
        "crv": "P-256",
        "use": "sig",
        "x": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(encoded.x().unwrap()),
        "y": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(encoded.y().unwrap()),
    })
}

fn cache_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_default();
    p.pop();
    p.push(CACHE_FILE);
    p
}

fn load_cache() -> Option<TokenCache> {
    let path = cache_path();
    std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

fn save_cache(cache: &TokenCache) {
    if let Ok(s) = serde_json::to_string(cache) {
        let _ = std::fs::write(cache_path(), &s);
    }
}

async fn request_device_code() -> Result<DeviceCodeResponse, String> {
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("build client: {e}"))?;
    let mut params = vec![
        ("client_id", CLIENT_ID),
        ("scope", SCOPE),
    ];
    params.push(("response_type", "device_code"));
    let resp = client
        .post("https://login.live.com/oauth20_connect.srf")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("device code request failed: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("read device code response: {e}"))?;
    if !status.is_success() {
        return Err(format!("device code HTTP {status}: {text}"));
    }
    let dc: DeviceCodeResponse = serde_json::from_str(&text)
        .map_err(|e| format!("parse device code: {e} (raw: {text})"))?;
    let msg = format!("{}|{}", dc.user_code, dc.verification_uri);
    eprintln!("[MSA] device code: {msg}");
    let _ = device_code_tx().send(msg);
    Ok(dc)
}

async fn poll_token(device_code: &str) -> Result<TokenResponse, String> {
    let client = reqwest::Client::new();
    loop {
        let params = [
            ("client_id", CLIENT_ID),
            ("grant_type", "device_code"),
            ("device_code", device_code),
        ];
        let resp = client
            .post("https://login.live.com/oauth20_token.srf")
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("token poll failed: {e}"))?;
        let text = resp.text().await.map_err(|e| format!("read token response: {e}"))?;
        if let Ok(tok) = serde_json::from_str::<TokenResponse>(&text) {
            return Ok(tok);
        }
        if let Ok(err) = serde_json::from_str::<TokenError>(&text) {
            if err.error == "authorization_pending" || err.error == "slow_down" {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
            return Err(format!("token error: {}", err.error));
        }
        return Err(format!("unexpected token response: {text}"));
    }
}

async fn device_authenticate() -> Result<String, String> {
    let client = reqwest::Client::new();
    let device_id = format!("{{{}}}", uuid::Uuid::new_v4());
    let pk = proof_key_value();
    let body = serde_json::json!({
        "Properties": {
            "DeviceType": "Win32",
            "Id": device_id,
            "AuthMethod": "ProofOfPossession",
            "ProofKey": pk,
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });
    let body_bytes = serde_json::to_vec(&body).map_err(|e| format!("serialize device body: {e}"))?;
    let sig = build_signature("POST", "/device/authenticate", &body_bytes);

    let resp = client
        .post("https://device.auth.xboxlive.com/device/authenticate")
        .header("x-xbl-contract-version", "1")
        .header("Signature", &sig)
        .header("Accept", "application/json")
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("device authenticate: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("device authenticate read: {e}"))?;
    if !status.is_success() {
        return Err(format!("device auth HTTP {status}: {text}"));
    }
    let auth: DeviceAuthResponse = serde_json::from_str(&text)
        .map_err(|e| format!("parse device auth: {e} {text}"))?;
    Ok(auth.token)
}

async fn sisu_authorize(msa_token: &str, device_token: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let pk = proof_key_value();
    let body = serde_json::json!({
        "Sandbox": "RETAIL",
        "UseModernGamertag": true,
        "AppId": CLIENT_ID,
        "AccessToken": format!("t={msa_token}"),
        "DeviceToken": device_token,
        "ProofKey": pk,
        "RelyingParty": "rp://api.minecraftservices.com/",
    });
    let body_bytes = serde_json::to_vec(&body).map_err(|e| format!("serialize sisu body: {e}"))?;
    let sig = build_signature("POST", "/authorize", &body_bytes);

    let resp = client
        .post("https://sisu.xboxlive.com/authorize")
        .header("Signature", &sig)
        .header("Accept", "application/json")
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| format!("sisu auth: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("sisu auth read: {e}"))?;
    if !status.is_success() {
        return Err(format!("sisu auth HTTP {status}: {text}"));
    }

    #[derive(Deserialize)]
    struct SisuResp {
        #[serde(rename = "UserToken")]
        user_token: SisuTok,
        #[serde(rename = "AuthorizationToken")]
        auth_token: SisuTok,
    }
    #[derive(Deserialize)]
    struct SisuTok {
        #[serde(rename = "Token")]
        token: String,
        #[serde(rename = "DisplayClaims")]
        claims: SisuClaims,
    }
    #[derive(Deserialize)]
    struct SisuClaims {
        xui: Vec<SisuXui>,
    }
    #[derive(Deserialize)]
    struct SisuXui {
        uhs: String,
    }

    let sisu: SisuResp = serde_json::from_str(&text)
        .map_err(|e| format!("parse sisu: {e} {text}"))?;
    let xsts_token = sisu.auth_token.token;
    let uhs = sisu.user_token.claims.xui.first()
        .ok_or_else(|| "no uhs in sisu user token".to_string())?
        .uhs.clone();
    Ok((xsts_token, uhs))
}

async fn mc_login(xsts_token: &str, uhs: &str) -> Result<(String, String, String), String> {
    let client = reqwest::Client::new();
    let identity = format!("XBL3.0 x={uhs};{xsts_token}");
    let body = serde_json::json!({ "identityToken": identity });
    let resp = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("mc login: {e}"))?;
    let text = resp.text().await.map_err(|e| format!("mc login read: {e}"))?;
    let login: McLoginResponse = serde_json::from_str(&text)
        .map_err(|e| format!("parse mc login: {e} {text}"))?;

    let profile_resp = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {}", login.access_token))
        .send()
        .await
        .map_err(|e| format!("mc profile: {e}"))?;
    let ptext = profile_resp.text().await.map_err(|e| format!("mc profile read: {e}"))?;
    let profile: McProfileResponse = serde_json::from_str(&ptext)
        .map_err(|e| format!("parse mc profile: {e} {ptext}"))?;

    Ok((profile.id, profile.name, login.access_token))
}

pub async fn authenticate() -> Result<MinecraftProfile, String> {
    if let Some(cached) = load_cache() {
        if let (Some(uuid), Some(uname), Some(token)) =
            (cached.mc_uuid, cached.mc_username, cached.mc_access_token)
        {
            return Ok(MinecraftProfile { uuid, username: uname, access_token: token });
        }
    }

    let device = request_device_code().await?;

    let tok = poll_token(&device.device_code).await?;
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + tok.expires_in;

    let device_token = device_authenticate().await?;
    let (xsts_token, uhs) = sisu_authorize(&tok.access_token, &device_token).await?;
    let (uuid, username, mc_token) = mc_login(&xsts_token, &uhs).await?;

    let cache = TokenCache {
        access_token: tok.access_token,
        refresh_token: tok.refresh_token,
        mc_uuid: Some(uuid.clone()),
        mc_username: Some(username.clone()),
        mc_access_token: Some(mc_token.clone()),
        expires_at: expires,
    };
    save_cache(&cache);

    Ok(MinecraftProfile { uuid, username, access_token: mc_token })
}
