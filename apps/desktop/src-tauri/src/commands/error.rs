use serde::Serialize;

/// Serializable command error (spec §6): the UI branches on `code` and
/// displays `message`. Domain commands map known failures to specific
/// codes (e.g. "tools-dir-missing"); everything else is "internal".
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub code: String,
    pub message: String,
}

impl CmdError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for CmdError {
    fn from(err: anyhow::Error) -> Self {
        // "{:#}" renders the full context chain: "outer: inner".
        Self::new("internal", format!("{err:#}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use anyhow::Context;

    #[test]
    fn anyhow_chain_flattens_into_code_and_message() {
        let err = anyhow::anyhow!("root cause").context("outer context");
        let cmd: CmdError = err.into();
        assert_eq!(cmd.code, "internal");
        assert_eq!(cmd.message, "outer context: root cause");
        let json = serde_json::to_value(&cmd).unwrap();
        assert_eq!(json["code"], "internal");
        assert_eq!(json["message"], "outer context: root cause");
    }
}
