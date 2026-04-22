use std::collections::BTreeMap;

use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: Value,
}

pub trait ToolExecutor: Send + Sync {
    fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError>;
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid tool payload: {0}")]
    InvalidPayload(String),
    #[error("tool execution failed: {0}")]
    Execution(String),
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn register(&mut self, name: impl Into<String>, executor: Box<dyn ToolExecutor>) {
        self.tools.insert(name.into(), executor);
    }

    pub fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        self.tools
            .get(&call.name)
            .ok_or_else(|| ToolError::NotFound(call.name.clone()))?
            .execute(call)
    }
}

pub struct EchoTool;

impl ToolExecutor for EchoTool {
    fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            output: json!({
                "tool": call.name,
                "args": call.args,
            }),
        })
    }
}
