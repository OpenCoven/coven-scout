use std::sync::Arc;
use serde_json::{json, Value};
use crate::{Config, CovenScoutError};
use crate::protocol::{Tool, ToolResult};
use tracing::instrument;

pub fn tool_descriptor() -> Tool {
    Tool {
        name: "test".to_string(),
        description: "Test/diagnostic tools: echo parameters, generate errors.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["echo", "generate_error"],
                    "description": "Operation to perform"
                },
                "params_to_echo": {
                    "description": "Any value to echo back (for echo operation)"
                },
                "error_code": {
                    "type": "string",
                    "description": "Error code string (for generate_error)"
                }
            },
            "required": ["operation"]
        }),
    }
}

#[instrument(skip(config))]
pub async fn handle(args: Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let operation = args.get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("echo");

    match operation {
        "echo" => {
            let params = args.get("params_to_echo").cloned().unwrap_or(Value::Null);
            let response = json!({
                "echoed": params,
                "server": "coven-scout",
                "version": env!("CARGO_PKG_VERSION"),
            });
            Ok(ToolResult::ok(serde_json::to_string_pretty(&response).unwrap_or_default()))
        }
        "generate_error" => {
            let code = args.get("error_code")
                .and_then(|v| v.as_str())
                .unwrap_or("TEST_ERROR");
            Ok(ToolResult::err(format!("Test error generated with code: {code}")))
        }
        other => Ok(ToolResult::err(format!("Unknown test operation: {other}"))),
    }
}
