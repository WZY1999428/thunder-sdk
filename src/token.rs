use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,      // Issuer
    aud: String,      // Audience (strAppID)
    iat: u64,         // Issued At
    exp: u64,         // Expiration Time
    scopes: String,   // 权限范围
}

pub fn build_token(
    str_app_id: &str,
    str_app_secret: &str,
    str_issuer: &str,
    n_expire_seconds: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    // 1. 获取当前 Unix 时间戳 (nSecondsSinceEpoch)
    let start = SystemTime::now();
    let n_seconds_since_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    // 2. 构建 Claims
    let my_claims = Claims {
        iss: str_issuer.to_string(),
        aud: str_app_id.to_string(),
        iat: n_seconds_since_epoch,
        exp: n_seconds_since_epoch + n_expire_seconds,
        scopes: "xtask.p2sp, xindex.query".to_string(),
    };

    // 3. 设置算法和密钥 (对应 HMAC256)
    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret(str_app_secret.as_bytes());

    // 4. 签名并返回
    encode(&header, &my_claims, &key)
}
