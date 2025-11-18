use serde_json::Value;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[allow(dead_code)]
pub async fn save_pretty_json(path: &str, json: &Value) -> Result<(), std::io::Error> {
    let pretty = serde_json::to_string_pretty(json).unwrap_or_else(|_| json.to_string());
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(pretty.as_bytes()).await?;
    file.write_all(b"\n\n").await?;
    file.flush().await?;
    Ok(())
}
