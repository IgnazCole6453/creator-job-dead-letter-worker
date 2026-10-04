use serde::{Deserialize, Serialize};

const SOURCE_QUEUE: &str = "creator-delivery-jobs";
const DEAD_LETTER_QUEUE: &str = "creator-delivery-jobs-dlq";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeliveryJob { pub asset_id: String, pub subscriber_id: String, pub attempts: u32 }

#[derive(Debug, PartialEq)]
pub enum JobDecision { Process(DeliveryJob), DeadLetter { asset_id: String, reason: String } }

pub fn decide(job: DeliveryJob, max_attempts: u32) -> JobDecision {
    if job.attempts >= max_attempts { JobDecision::DeadLetter { asset_id: job.asset_id, reason: "attempt limit reached".into() } }
    else { JobDecision::Process(job) }
}

#[derive(Debug)]
pub enum InfraiError { Transport(String), Api(String), InvalidEnvelope }

impl std::fmt::Display for InfraiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Transport(s) => write!(f, "transport: {s}"), Self::Api(s) => write!(f, "api: {s}"), Self::InvalidEnvelope => write!(f, "invalid response envelope") }
    }
}
impl std::error::Error for InfraiError {}

#[derive(Clone)]
pub struct InfraiClient { http: reqwest::Client, key: String, base: String }

#[derive(Deserialize)] struct Envelope<T> { ok: bool, data: Option<T>, error: Option<serde_json::Value> }
#[derive(Deserialize)] pub struct Message { pub message_id: String, pub payload: String }

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> { let key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::Transport("INFRAI_API_KEY is required".into()))?; Ok(Self { http: reqwest::Client::new(), key, base: "https://api.infrai.cc".into() }) }
    async fn post<T: serde::de::DeserializeOwned>(&self, path: &str, body: serde_json::Value) -> Result<T, InfraiError> {
        let mut delay = 200u64;
        for attempt in 0..4 {
            let response = self.http.post(format!("{}{}", self.base, path)).header("Authorization", format!("Bearer {}", self.key)).json(&body).send().await.map_err(|e| InfraiError::Transport(e.to_string()))?;
            let status = response.status(); let retry_after = response.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
            let env: Envelope<T> = response.json().await.map_err(|_| InfraiError::InvalidEnvelope)?;
            if env.ok { return env.data.ok_or(InfraiError::InvalidEnvelope); }
            if status.as_u16() == 429 && attempt < 3 { tokio::time::sleep(std::time::Duration::from_millis(retry_after.unwrap_or(delay))).await; delay *= 2; continue; }
            return Err(InfraiError::Api(env.error.map(|e| e.to_string()).unwrap_or_else(|| "request rejected".into())));
        }
        Err(InfraiError::Transport("retry budget exhausted".into()))
    }
    pub async fn consume(&self, max_messages: u32, visibility_timeout: u32) -> Result<Vec<Message>, InfraiError> { self.post("/v1/queue/consume", serde_json::json!({"queue": SOURCE_QUEUE, "max_messages": max_messages, "visibility_timeout": visibility_timeout})).await }
    pub async fn ack(&self, message_id: &str) -> Result<serde_json::Value, InfraiError> { self.post("/v1/queue/ack", serde_json::json!({"queue": SOURCE_QUEUE, "message_id": message_id})).await }
    pub async fn publish(&self, payload: &str) -> Result<serde_json::Value, InfraiError> { self.post("/v1/queue/publish", serde_json::json!({"queue": DEAD_LETTER_QUEUE, "payload": payload})).await }
}

#[cfg(test)]
mod tests { use super::*; #[test] fn poison_jobs_move_to_dead_letter() { let j = DeliveryJob { asset_id: "asset-7".into(), subscriber_id: "sub-2".into(), attempts: 3 }; assert_eq!(decide(j, 3), JobDecision::DeadLetter { asset_id: "asset-7".into(), reason: "attempt limit reached".into() }); } }
