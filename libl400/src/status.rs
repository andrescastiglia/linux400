#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStatus {
    pub code: &'static str,
    pub severity: u8,
    pub message: &'static str,
    pub detail: &'static str,
}

pub const CPF_CATALOG: &[CommandStatus] = &[
    CommandStatus {
        code: "CPF0000",
        severity: 0,
        message: "Generic CPF escape",
        detail: "Generic monitorable CPF status.",
    },
    CommandStatus {
        code: "CPF0001",
        severity: 30,
        message: "Command failed",
        detail: "The command handler returned an implementation error.",
    },
    CommandStatus {
        code: "CPF0006",
        severity: 30,
        message: "Invalid parameter",
        detail: "A command parameter is not valid for the command metadata.",
    },
    CommandStatus {
        code: "CPF2204",
        severity: 40,
        message: "Authority insufficient",
        detail: "The current profile does not have the required object authority.",
    },
    CommandStatus {
        code: "CPF9801",
        severity: 30,
        message: "Object not found",
        detail: "The requested Linux/400 object does not exist.",
    },
    CommandStatus {
        code: "CPF9802",
        severity: 30,
        message: "Object type incorrect",
        detail: "The requested object exists but is not valid for this operation.",
    },
    CommandStatus {
        code: "CPF9898",
        severity: 40,
        message: "Backend unavailable",
        detail: "Required storage, runtime or platform backend is unavailable.",
    },
];

pub fn command_status(code: &str) -> CommandStatus {
    let normalized = normalize_cpf(code);
    CPF_CATALOG
        .iter()
        .copied()
        .find(|status| status.code == normalized)
        .unwrap_or(CommandStatus {
            code: "CPF0001",
            severity: 30,
            message: "Command failed",
            detail: "No detailed CPF catalog entry exists for this status.",
        })
}

pub fn normalize_cpf(code: &str) -> String {
    let trimmed = code.trim().to_uppercase();
    if trimmed.starts_with("CPF") {
        trimmed
    } else {
        format!("CPF{trimmed:0>4}")
    }
}
