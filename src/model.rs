use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "trace" | "trc" => Self::Trace,
            "debug" | "dbg" | "verbose" | "verb" => Self::Debug,
            "info" | "information" | "inf" | "notice" => Self::Info,
            "warn" | "warning" | "wrn" => Self::Warn,
            "error" | "err" | "severe" => Self::Error,
            "fatal" | "critical" | "crit" | "panic" | "alert" | "emerg" | "emergency" => {
                Self::Fatal
            }
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
            Self::Unknown => "?????",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFormat {
    Json,
    Logfmt,
    Plain,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    Number(String),
    Bool(bool),
    Null,
    Nested(String),
}

impl FieldValue {
    pub fn display(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => n.clone(),
            Self::Bool(b) => b.to_string(),
            Self::Null => "null".into(),
            Self::Nested(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub key: String,
    pub value: FieldValue,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub line_no: usize,
    pub raw: String,
    pub format: LineFormat,
    pub level: LogLevel,
    pub timestamp: Option<String>,
    pub message: Option<String>,
    pub fields: Vec<Field>,
}

impl LogEntry {
    pub fn summary_message(&self) -> &str {
        self.message.as_deref().unwrap_or(self.raw.as_str())
    }
}
