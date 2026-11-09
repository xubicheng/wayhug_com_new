use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub fn token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

pub fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

pub fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub fn now_utc() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub fn plus_hours(hours: i64) -> String {
    (Utc::now() + chrono::Duration::hours(hours))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn is_expired(ts: &str) -> bool {
    ts < now_utc().as_str()
}

/// 按字符数估算阅读时长：中文约 400 字/分钟，英文约 220 词/分钟。
pub fn reading_minutes(body: &str) -> i64 {
    let chars = body.chars().count() as f64;
    let words = body.split_whitespace().count() as f64;
    let minutes = (chars / 400.0).max(words / 220.0);
    minutes.ceil().max(1.0) as i64
}

pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn ip_hash(headers: &HeaderMap) -> String {
    sha256_hex(&format!("wh-ip:{}", client_ip(headers)))[..16].to_string()
}

pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(&format!("{}=", name)) {
            return Some(rest.to_string());
        }
    }
    None
}

/// 简单内存限流：窗口内同一 key 的请求次数上限。
pub struct RateLimiter {
    inner: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow(&self, key: &str, max: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap();
        if map.len() > 4096 {
            map.retain(|_, v| v.iter().any(|t| now.duration_since(*t) < window));
        }
        let entry = map.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < window);
        if entry.len() >= max {
            return false;
        }
        entry.push(now);
        true
    }
}

pub struct AppError(pub anyhow::Error);

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("request failed: {:#}", self.0);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::response::Html("<h1>500</h1><p>服务器出错了，请稍后再试。</p>"),
        )
            .into_response()
    }
}
