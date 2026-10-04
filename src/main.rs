use creator_dlq_worker::{decide, DeliveryJob, InfraiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InfraiClient::from_env()?;
    for message in client.consume(10, 30).await? {
        let job: DeliveryJob = serde_json::from_str(&message.payload)?;
        match decide(job, 3) {
            creator_dlq_worker::JobDecision::Process(job) => { println!("processed {} for {}", job.asset_id, job.subscriber_id); client.ack(&message.message_id).await?; }
            creator_dlq_worker::JobDecision::DeadLetter { asset_id, reason } => { let payload = serde_json::json!({"asset_id": asset_id, "reason": reason}).to_string(); client.publish(&payload).await?; client.ack(&message.message_id).await?; println!("dead-lettered {}", asset_id); }
        }
    }
    Ok(())
}
