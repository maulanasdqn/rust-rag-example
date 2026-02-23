//! DateTime tool for getting current date and time information

use async_trait::async_trait;
use chrono::{Datelike, Local, TimeZone, Timelike, Utc};
use serde_json::{json, Value};

use crate::traits::{Tool, ToolError, ToolResult};

/// Tool for getting current date and time information
pub struct DateTimeTool;

impl DateTimeTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DateTimeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DateTimeTool {
    fn name(&self) -> &str {
        "datetime"
    }

    fn description(&self) -> &str {
        "Get the current date and time. Can return the time in UTC or local timezone, \
         and can format the output in various ways."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "enum": ["utc", "local"],
                    "description": "The timezone to use (default: utc)"
                },
                "format": {
                    "type": "string",
                    "enum": ["iso", "human", "date_only", "time_only"],
                    "description": "Output format (default: iso)"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let timezone = args
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("utc");

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("iso");

        let (datetime_str, data) = match timezone {
            "local" => {
                let now = Local::now();
                let formatted = format_datetime(&now, format);
                let data = json!({
                    "year": now.year(),
                    "month": now.month(),
                    "day": now.day(),
                    "hour": now.hour(),
                    "minute": now.minute(),
                    "second": now.second(),
                    "timezone": "local",
                    "iso": now.to_rfc3339()
                });
                (formatted, data)
            }
            _ => {
                let now = Utc::now();
                let formatted = format_datetime(&now, format);
                let data = json!({
                    "year": now.year(),
                    "month": now.month(),
                    "day": now.day(),
                    "hour": now.hour(),
                    "minute": now.minute(),
                    "second": now.second(),
                    "timezone": "UTC",
                    "iso": now.to_rfc3339()
                });
                (formatted, data)
            }
        };

        Ok(ToolResult::success_with_data(datetime_str, data))
    }
}

fn format_datetime<Tz: TimeZone>(dt: &chrono::DateTime<Tz>, format: &str) -> String
where
    Tz::Offset: std::fmt::Display,
{
    match format {
        "human" => dt.format("%A, %B %d, %Y at %H:%M:%S").to_string(),
        "date_only" => dt.format("%Y-%m-%d").to_string(),
        "time_only" => dt.format("%H:%M:%S").to_string(),
        _ => dt.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datetime_utc() {
        let tool = DateTimeTool::new();

        let result = tool
            .execute(json!({"timezone": "utc"}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.data.is_some());

        let data = result.data.unwrap();
        assert_eq!(data["timezone"], "UTC");
    }

    #[tokio::test]
    async fn test_datetime_human_format() {
        let tool = DateTimeTool::new();

        let result = tool
            .execute(json!({"format": "human"}))
            .await
            .unwrap();

        assert!(result.success);
        // Human format should contain day name
        assert!(result.output.contains("day") || result.output.contains(","));
    }

    #[tokio::test]
    async fn test_datetime_date_only() {
        let tool = DateTimeTool::new();

        let result = tool
            .execute(json!({"format": "date_only"}))
            .await
            .unwrap();

        assert!(result.success);
        // Should match YYYY-MM-DD format
        assert!(result.output.contains("-"));
        assert_eq!(result.output.len(), 10);
    }
}
