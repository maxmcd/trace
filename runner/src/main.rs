use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;
use std::io;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!(
        "Starting runner, {:?}",
        std::env::args().nth(2).unwrap_or_default()
    );
    let mut child = Command::new(std::env::args().nth(1).unwrap_or_default())
        .args(std::env::args().skip(2))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let span_map = Arc::new(Mutex::new(HashMap::new()));

    tokio::spawn(handle_output(stdout, "stdout", Arc::clone(&span_map)));
    tokio::spawn(handle_output(stderr, "stderr", Arc::clone(&span_map)));
    let pid = child.id().unwrap() as i32;

    let status = child.wait().await?;

    // Exit this process with the same code as the child process
    std::process::exit(status.code().unwrap_or(0));
}

fn log(v: serde_json::Value) {
    println!("{}", serde_json::to_string(&v).unwrap());
}

async fn handle_output<R: AsyncReadExt + Unpin>(
    reader: R,
    stream_type: &'static str,
    span_map: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
) {
    let buf_reader = BufReader::new(reader);
    let mut lines = buf_reader.lines();
    while let Some(line) = lines.next_line().await.unwrap() {
        // println!("{}", line);
        if let Some(output) = process_line(&line, stream_type, &span_map).await {
            log(output);
        }
    }
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn process_line(
    line: &str,
    stream_type: &'static str,
    span_map: &Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
) -> Option<serde_json::Value> {
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
        if let (Some(event), Some(span)) =
            (json_value["event"].as_str(), json_value["span"].as_object())
        {
            if let Some(span_id) = span["spanID"].as_str() {
                let mut map = span_map.lock().await;
                match event {
                    "start" => {
                        map.insert(span_id.to_string(), now());
                        return None;
                    }
                    "end" => {
                        if let Some(start_time) = map.remove(span_id) {
                            let end_time = now();
                            let duration = end_time.timestamp_nanos_opt().unwrap()
                                - start_time.timestamp_nanos_opt().unwrap();
                            let mut output = serde_json::Map::new();
                            output.insert(
                                "timestamp".to_string(),
                                json!(
                                    start_time.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
                                ),
                            );
                            output.insert("duration".to_string(), json!(duration));
                            if let Some(span) = json_value["span"].as_object() {
                                for (key, value) in span {
                                    output.insert(key.clone(), value.clone());
                                }
                            }
                            output.insert(
                                "serviceName".to_string(),
                                serde_json::Value::String("test-service".to_string()),
                            );
                            return Some(serde_json::Value::Object(output));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // If we reach here, it means the log line didn't match our expected format
    // or wasn't a start/end event, so we fall back to the original logic
    Some(create_json_value(line, stream_type).await)
}

async fn create_json_value(line: &str, stream_type: &'static str) -> serde_json::Value {
    let default_values = json!({
        "timestamp": now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
    });

    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
        if let serde_json::Value::Object(map) = json_value {
            let mut valid_json = serde_json::Map::new();
            for (key, value) in default_values.as_object().unwrap() {
                valid_json.insert(key.clone(), value.clone());
            }
            for (key, value) in map {
                valid_json.insert(key, value);
            }
            if !valid_json.contains_key("level") {
                valid_json.insert(
                    "level".to_string(),
                    serde_json::Value::String(stream_type.to_string()),
                );
            }

            return serde_json::Value::Object(valid_json);
        }
    }
    let mut json_response = default_values;
    json_response["level"] = serde_json::Value::String(stream_type.to_string());
    json_response["message"] = serde_json::Value::String(line.to_string());
    json_response
}
