use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub type ProcessId = Uuid;
pub type EventId = Uuid;
pub type TapeId = Uuid;
pub type EffectId = Uuid;
pub type CheckpointId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessStatus {
    Idle,
    Running,
    Waiting,
    Paused,
    Finished,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub id: ProcessId,
    pub status: ProcessStatus,
    pub checkpoint: Option<CheckpointId>,
    pub capability_set: Vec<String>,
    pub module_set: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventFamily {
    Input,
    Runtime,
    Provider,
    Tool,
    Policy,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSource {
    User,
    Scheduler,
    Provider,
    Tool,
    Gateway,
    Plugin,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub process_id: ProcessId,
    pub family: EventFamily,
    pub ty: String,
    pub source: EventSource,
    pub payload: Value,
    pub durable: bool,
    pub visible_in_tape: bool,
    pub causation_id: Option<EventId>,
    pub correlation_id: Option<String>,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TapeMajor {
    User,
    Assistant,
    Tool,
    Injection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapeItem {
    pub id: TapeId,
    pub process_id: ProcessId,
    pub major: TapeMajor,
    pub subtype: String,
    pub content: Value,
    pub refs: Vec<EventId>,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectKind {
    CallTool,
    WriteMemory,
    DelegateTask,
    RequestApproval,
    EmitMessage,
    Pause,
    Finish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: EffectId,
    pub process_id: ProcessId,
    pub kind: EffectKind,
    pub payload: Value,
    pub blocking: bool,
}

impl Process {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            status: ProcessStatus::Idle,
            checkpoint: None,
            capability_set: Vec::new(),
            module_set: Vec::new(),
        }
    }
}
