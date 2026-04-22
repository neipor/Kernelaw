use chrono::Utc;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::hooks::{HookContext, HookRunner, HookStage};
use crate::model_config::ModelConfig;
use crate::module::ModuleRegistry;
use crate::provider::{ProviderAdapter, ProviderError, ProviderMessage, ProviderRequest};
use crate::tool_runtime::{ToolCall, ToolError, ToolRegistry};
use crate::types::{
    Effect, EffectKind, Event, EventFamily, EventSource, Process, ProcessStatus, TapeItem,
    TapeMajor,
};

#[derive(Debug, Clone)]
pub struct RuntimeStepResult {
    pub new_events: Vec<Event>,
    pub new_tape_items: Vec<TapeItem>,
    pub new_effects: Vec<Effect>,
}

pub struct KernelRuntime<P: ProviderAdapter> {
    provider: P,
    modules: ModuleRegistry,
    hooks: HookRunner,
    tools: ToolRegistry,
}

impl<P: ProviderAdapter> KernelRuntime<P> {
    pub fn new(
        provider: P,
        modules: ModuleRegistry,
        hooks: HookRunner,
        tools: ToolRegistry,
    ) -> Self {
        Self {
            provider,
            modules,
            hooks,
            tools,
        }
    }

    pub fn step(
        &self,
        process: &mut Process,
        model: &ModelConfig,
        event_log: &[Event],
        tape: &[TapeItem],
    ) -> Result<RuntimeStepResult, RuntimeError> {
        model.validate().map_err(RuntimeError::ModelConfig)?;

        process.status = ProcessStatus::Running;
        let projection = self.project(event_log, tape);

        let hook_ctx = self.hooks.run(
            HookStage::BeforeReply,
            HookContext {
                process_id: process.id.to_string(),
                payload: json!({"context_items": projection.len()}),
            },
        );

        let request = self.build_provider_request(&projection);
        let provider_response = self.provider.generate(model, &request)?;

        let mut new_effects = vec![Effect {
            id: Uuid::new_v4(),
            process_id: process.id,
            kind: EffectKind::EmitMessage,
            payload: json!({ "channel": "default" }),
            blocking: false,
        }];

        let mut new_tape_items = vec![TapeItem {
            id: Uuid::new_v4(),
            process_id: process.id,
            major: TapeMajor::Assistant,
            subtype: "assistant.final".to_string(),
            content: json!({
                "text": provider_response.assistant_message,
                "hook": hook_ctx.payload
            }),
            refs: Vec::new(),
            ts: Utc::now(),
        }];

        let tool_effects = self.collect_requested_tool_effects(&projection, process.id)?;
        if !tool_effects.is_empty() {
            new_effects.extend(tool_effects);
        }

        let tool_tape_items = self.execute_tool_effects(process.id, &new_effects)?;
        if !tool_tape_items.is_empty() {
            new_tape_items.extend(tool_tape_items);
        }

        let new_events = vec![Event {
            id: Uuid::new_v4(),
            process_id: process.id,
            family: EventFamily::Runtime,
            ty: "step_committed".to_string(),
            source: EventSource::System,
            payload: json!({
                "tape_items": new_tape_items.len(),
                "effects": new_effects.len(),
                "enabled_modules": self.modules.enabled_modules().len()
            }),
            durable: true,
            visible_in_tape: false,
            causation_id: None,
            correlation_id: None,
            ts: Utc::now(),
        }];

        process.status = ProcessStatus::Waiting;

        Ok(RuntimeStepResult {
            new_events,
            new_tape_items,
            new_effects,
        })
    }

    fn build_provider_request(&self, projection: &[TapeItem]) -> ProviderRequest {
        let messages = projection
            .iter()
            .map(|item| ProviderMessage {
                role: match item.major {
                    TapeMajor::User => "user",
                    TapeMajor::Assistant => "assistant",
                    TapeMajor::Tool => "tool",
                    TapeMajor::Injection => "system",
                }
                .to_string(),
                content: item
                    .content
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect();

        ProviderRequest { messages }
    }

    fn project(&self, _event_log: &[Event], tape: &[TapeItem]) -> Vec<TapeItem> {
        tape.iter().rev().take(32).cloned().collect()
    }

    fn collect_requested_tool_effects(
        &self,
        projection: &[TapeItem],
        process_id: uuid::Uuid,
    ) -> Result<Vec<Effect>, RuntimeError> {
        let mut effects = Vec::new();
        for item in projection {
            if item.subtype == "injection.tool_request" {
                if !self.modules.has_capability("tool.call") {
                    return Err(RuntimeError::MissingCapability("tool.call".to_string()));
                }
                let name = item
                    .content
                    .get("tool")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        RuntimeError::Tool(ToolError::InvalidPayload("missing tool".to_string()))
                    })?;
                let args = item
                    .content
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                effects.push(Effect {
                    id: Uuid::new_v4(),
                    process_id,
                    kind: EffectKind::CallTool,
                    payload: json!({"name": name, "args": args}),
                    blocking: true,
                });
            }
        }
        Ok(effects)
    }

    fn execute_tool_effects(
        &self,
        process_id: uuid::Uuid,
        effects: &[Effect],
    ) -> Result<Vec<TapeItem>, RuntimeError> {
        let mut out = Vec::new();
        for effect in effects {
            if effect.kind != EffectKind::CallTool {
                continue;
            }

            let name = effect
                .payload
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    RuntimeError::Tool(ToolError::InvalidPayload("missing name".to_string()))
                })?;
            let args = effect
                .payload
                .get("args")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = self.tools.execute(&ToolCall {
                name: name.to_string(),
                args,
            })?;

            out.push(TapeItem {
                id: Uuid::new_v4(),
                process_id,
                major: TapeMajor::Tool,
                subtype: "tool.result".to_string(),
                content: json!({
                    "tool": name,
                    "result": result.output,
                }),
                refs: Vec::new(),
                ts: Utc::now(),
            });
        }
        Ok(out)
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("model config invalid: {0}")]
    ModelConfig(#[from] crate::model_config::ModelConfigError),
    #[error("provider failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("missing required capability: {0}")]
    MissingCapability(String),
    #[error("tool runtime failed: {0}")]
    Tool(#[from] ToolError),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::hooks::{HookMode, HookOutcome, HookRegistration};
    use crate::module::{ModuleKind, ModuleManifest, ModuleSource};
    use crate::provider::{ProviderAdapter, ProviderResponse};
    use crate::tool_runtime::{EchoTool, ToolRegistry};

    struct MockProvider;

    impl ProviderAdapter for MockProvider {
        fn generate(
            &self,
            _model: &ModelConfig,
            _request: &ProviderRequest,
        ) -> Result<ProviderResponse, ProviderError> {
            Ok(ProviderResponse {
                assistant_message: "ok".to_string(),
            })
        }
    }

    fn test_model() -> ModelConfig {
        ModelConfig {
            provider: crate::ProviderKind::Local,
            model: "tiny-local".into(),
            endpoint: Some("http://127.0.0.1:11434".into()),
            api_key_env: None,
            temperature: 0.2,
            max_output_tokens: Some(256),
            timeout_ms: 5_000,
        }
    }

    #[test]
    fn step_generates_assistant_and_commit_event() {
        let mut process = Process::new();
        let mut hooks = HookRunner::default();
        hooks.register(
            HookRegistration {
                module: "observer".to_string(),
                stage: HookStage::BeforeReply,
                priority: 1,
                mode: HookMode::Observe,
                terminal: false,
            },
            Box::new(|ctx| HookOutcome {
                stop: false,
                context_patch: Some(json!({"seen": ctx.payload})),
            }),
        );

        let runtime = KernelRuntime::new(
            MockProvider,
            ModuleRegistry::default(),
            hooks,
            ToolRegistry::default(),
        );
        let result = runtime.step(&mut process, &test_model(), &[], &[]).unwrap();

        assert_eq!(result.new_tape_items.len(), 1);
        assert_eq!(result.new_tape_items[0].major, TapeMajor::Assistant);
        assert_eq!(result.new_events[0].ty, "step_committed");
        assert_eq!(process.status, ProcessStatus::Waiting);
    }

    #[test]
    fn tool_call_requires_capability_and_executor() {
        let mut process = Process::new();

        let mut registry = ModuleRegistry::default();
        let mut caps = BTreeSet::new();
        caps.insert("tool.call".to_string());
        registry
            .install(ModuleManifest {
                name: "tool-runtime".to_string(),
                version: "0.1.0".to_string(),
                kind: ModuleKind::Tool,
                capabilities: caps,
                source: ModuleSource::Builtin,
                enabled: true,
            })
            .unwrap();

        let mut tools = ToolRegistry::default();
        tools.register("echo", Box::new(EchoTool));

        let runtime = KernelRuntime::new(MockProvider, registry, HookRunner::default(), tools);

        let injection = TapeItem {
            id: Uuid::new_v4(),
            process_id: process.id,
            major: TapeMajor::Injection,
            subtype: "injection.tool_request".to_string(),
            content: json!({"tool":"echo","args":{"q":"hi"}}),
            refs: vec![],
            ts: Utc::now(),
        };

        let result = runtime
            .step(&mut process, &test_model(), &[], &[injection])
            .unwrap();

        assert!(result
            .new_effects
            .iter()
            .any(|e| e.kind == EffectKind::CallTool));
        assert!(result
            .new_tape_items
            .iter()
            .any(|item| item.subtype == "tool.result"));
    }
}
