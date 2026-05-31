use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputTraceLine {
    file: String,
    line: Option<u32>,
    column: Option<u32>,
    function: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMessageKind {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OutputMessage {
    kind: OutputMessageKind,
    message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    trace: Vec<OutputTraceLine>,
}

impl OutputMessage {
    pub fn new(kind: OutputMessageKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            trace: Vec::new(),
        }
    }

    pub fn with_trace<I>(mut self, trace: I) -> Self
    where
        I: IntoIterator<Item = OutputTraceLine>,
    {
        self.trace.clear();
        self.trace.extend(trace);
        self
    }
}
