pub mod hooks;
pub mod model_config;
pub mod module;
pub mod provider;
pub mod runtime;
pub mod tool_runtime;
pub mod types;

pub use hooks::{HookContext, HookMode, HookRegistration, HookRunner, HookStage};
pub use model_config::{ModelConfig, ProviderKind};
pub use module::{ModuleKind, ModuleManifest, ModuleRegistry, ModuleSource};
pub use provider::{
    OllamaProvider, ProviderAdapter, ProviderError, ProviderMessage, ProviderRequest,
    ProviderResponse, StaticProvider,
};
pub use runtime::{KernelRuntime, RuntimeError, RuntimeStepResult};
pub use tool_runtime::{EchoTool, ToolCall, ToolError, ToolExecutor, ToolRegistry, ToolResult};
pub use types::{
    Effect, EffectKind, Event, EventFamily, EventSource, Process, ProcessStatus, TapeItem,
    TapeMajor,
};
