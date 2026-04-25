use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookStage {
    BeforeModelResolve,
    BeforePromptBuild,
    BeforeReply,
    BeforeToolCall,
    AfterToolCall,
    BeforeMessageWrite,
    SessionStart,
    SessionEnd,
    SubagentSpawning,
    SubagentSpawned,
    SubagentEnded,
    GatewayStart,
    GatewayStop,
    InstallGuard,
    PolicyGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookMode {
    Observe,
    Modify,
    Claim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRegistration {
    pub module: String,
    pub stage: HookStage,
    pub priority: i32,
    pub mode: HookMode,
    pub terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub process_id: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutcome {
    pub stop: bool,
    pub context_patch: Option<Value>,
}

pub type HookHandler = Box<dyn Fn(&HookContext) -> HookOutcome + Send + Sync>;

#[derive(Default)]
pub struct HookRunner {
    hooks: Vec<(HookRegistration, HookHandler)>,
}

impl HookRunner {
    pub fn register(&mut self, registration: HookRegistration, handler: HookHandler) {
        self.hooks.push((registration, handler));
        self.hooks.sort_by(|a, b| {
            b.0.priority
                .cmp(&a.0.priority)
                .then_with(|| a.0.module.cmp(&b.0.module))
        });
    }

    pub fn run(&self, stage: HookStage, mut ctx: HookContext) -> HookContext {
        for (registration, handler) in self.hooks.iter().filter(|(r, _)| r.stage == stage) {
            let outcome = handler(&ctx);
            if let Some(patch) = outcome.context_patch {
                ctx.payload = patch;
            }
            if outcome.stop || registration.terminal {
                break;
            }
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn priority_order_and_terminal_stop_work() {
        let mut runner = HookRunner::default();
        runner.register(
            HookRegistration {
                module: "policy".to_string(),
                stage: HookStage::BeforeReply,
                priority: 100,
                mode: HookMode::Modify,
                terminal: true,
            },
            Box::new(|_ctx| HookOutcome {
                stop: false,
                context_patch: Some(json!({"text":"blocked"})),
            }),
        );

        runner.register(
            HookRegistration {
                module: "observer".to_string(),
                stage: HookStage::BeforeReply,
                priority: 10,
                mode: HookMode::Observe,
                terminal: false,
            },
            Box::new(|_ctx| HookOutcome {
                stop: false,
                context_patch: Some(json!({"text":"should-not-run"})),
            }),
        );

        let out = runner.run(
            HookStage::BeforeReply,
            HookContext {
                process_id: "p1".to_string(),
                payload: json!({"text":"draft"}),
            },
        );

        assert_eq!(out.payload, json!({"text":"blocked"}));
    }
}
