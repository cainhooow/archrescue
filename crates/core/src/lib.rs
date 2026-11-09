//! Shared contracts. No platform access or write operations live here.
use serde::{Deserialize, Serialize};
use std::{
    fmt, io,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorKind {
    Validation,
    Environment,
    Permission,
    Io,
    Incomplete,
    Unsupported,
    Cancelled,
    Integrity,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub kind: ErrorKind,
    pub context: String,
    pub os_code: Option<i32>,
}
impl Error {
    pub fn new(kind: ErrorKind, context: impl Into<String>) -> Self {
        Self {
            kind,
            context: context.into(),
            os_code: None,
        }
    }
    pub fn io(context: impl Into<String>, err: io::Error) -> Self {
        Self {
            kind: if err.kind() == io::ErrorKind::PermissionDenied {
                ErrorKind::Permission
            } else {
                ErrorKind::Io
            },
            context: format!("{}: {}", context.into(), err),
            os_code: err.raw_os_error(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.context)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Pending,
    Validating,
    Running,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub state: State,
    pub transitions: Vec<State>,
    pub errors: Vec<Error>,
}
impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}
impl Operation {
    pub fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            id: format!(
                "ar-{now:x}-{:x}-{:x}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ),
            state: State::Pending,
            transitions: vec![State::Pending],
            errors: vec![],
        }
    }
    pub fn transition(&mut self, next: State) -> Result<()> {
        let valid = matches!(
            (self.state, next),
            (State::Pending, State::Validating)
                | (State::Validating, State::Running)
                | (State::Running, State::Verifying)
                | (State::Verifying, State::Completed)
        ) || (matches!(
            self.state,
            State::Pending | State::Validating | State::Running | State::Verifying
        ) && matches!(next, State::Failed | State::Cancelled | State::Blocked));
        if !valid {
            return Err(Error::new(
                ErrorKind::Validation,
                "Invalid operation state transition",
            ));
        }
        self.state = next;
        self.transitions.push(next);
        Ok(())
    }
    pub fn fail(&mut self, err: Error) {
        let state = match err.kind {
            ErrorKind::Cancelled => State::Cancelled,
            ErrorKind::Environment | ErrorKind::Unsupported => State::Blocked,
            _ => State::Failed,
        };
        if let Err(transition_error) = self.transition(state) {
            self.errors.push(transition_error);
        }
        self.errors.push(err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_cannot_skip_verification() {
        let mut o = Operation::new();
        assert!(o.transition(State::Completed).is_err());
        for s in [
            State::Validating,
            State::Running,
            State::Verifying,
            State::Completed,
        ] {
            o.transition(s).unwrap();
        }
        assert!(o.transition(State::Running).is_err());
    }
    #[test]
    fn cancellation_is_terminal() {
        let mut o = Operation::new();
        o.fail(Error::new(ErrorKind::Cancelled, "test"));
        assert_eq!(o.state, State::Cancelled);
        assert!(o.transition(State::Completed).is_err());
    }
}
