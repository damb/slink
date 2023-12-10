use std::borrow;
use std::fmt;

use serde::{Serialize, Serializer};

/// SeedLink `v4` protocol error codes.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ErrorCode {
    /// Generic error.
    Generic,
    /// Command not recognized or not supported.
    UnsupportedCommand,
    /// Command not expected.
    UnexpectedCommand,
    /// Client is not authorized to use the command.
    UnauthorizedCommand,
    /// Limit exceeded.
    LimitExceeded,
    /// Incorrect command arguments.
    IncorrectArguments,
    /// Authentication failed.
    AuthenticationFailed,
    /// Internal error.
    Internal,
}

impl ErrorCode {
    /// Returns the single-word error code representation.
    pub const fn code(&self) -> &'static str {
        match *self {
            Self::Generic => "GENERIC",
            Self::UnsupportedCommand => "UNSUPPORTED",
            Self::UnexpectedCommand => "UNEXPECTED",
            Self::UnauthorizedCommand => "UNAUTHORIZED",
            Self::LimitExceeded => "LIMIT",
            Self::IncorrectArguments => "ARGUMENTS",
            Self::AuthenticationFailed => "AUTH",
            Self::Internal => "INTERNAL",
        }
    }

    /// Returns a human-readable description of the error.
    pub const fn description(&self) -> &'static str {
        match *self {
            Self::Generic => "Generic error",
            Self::UnsupportedCommand => "Command not recognized or not supported",
            Self::UnexpectedCommand => "Command not expected",
            Self::UnauthorizedCommand => "Client is not authorized to use the command",
            Self::LimitExceeded => "Limit exceeded",
            Self::IncorrectArguments => "Incorrect command arguments",
            Self::AuthenticationFailed => "Authentication failed",
            Self::Internal => "Internal error",
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// SeedLink `v4` protocol error.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    /// A short description of the error.
    #[serde(default = "default_message")]
    pub message: Option<borrow::Cow<'static, str>>,
    ///Flag indicating whether the error is related to a info request
    #[serde(skip_serializing)]
    pub info: bool,
}

impl Error {
    /// Creates a new error from the given `ErrorCode`.
    pub const fn new(code: ErrorCode) -> Self {
        Error {
            message: Some(borrow::Cow::Borrowed(code.description())),
            code,
            info: false,
        }
    }

    /// Creates a new generic error
    pub const fn generic() -> Self {
        Self::new(ErrorCode::Generic)
    }

    /// Creates a new unsupported command error (`UNSUPPORTED`).
    pub const fn unsupported_command() -> Self {
        Self::new(ErrorCode::UnsupportedCommand)
    }

    /// Creates a new unexpected command error (`UNEXPECTED`).
    pub const fn unexpected_command() -> Self {
        Self::new(ErrorCode::UnexpectedCommand)
    }

    /// Creates a new unauthorized command error (`UNAUTHORIZED`).
    pub const fn unauthorized_command() -> Self {
        Self::new(ErrorCode::UnauthorizedCommand)
    }

    /// Creates a new limit exceeded error (`LIMIT`).
    pub const fn limit_exceeded() -> Self {
        Self::new(ErrorCode::LimitExceeded)
    }

    /// Creates a new incorrect arguments error (`ARGUMENTS`).
    pub const fn incorrect_arguments() -> Self {
        Self::new(ErrorCode::IncorrectArguments)
    }

    /// Creates a new authentication failed error (`AUTH`).
    pub const fn authentication_failed() -> Self {
        Self::new(ErrorCode::AuthenticationFailed)
    }

    /// Creates a new internal error (`INTERNAL`)
    pub const fn internal() -> Self {
        Self::new(ErrorCode::Internal)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR {}", self.code)?;
        if let Some(ref msg) = self.message {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

fn default_message() -> Option<borrow::Cow<'static, str>> {
    Some(borrow::Cow::Borrowed("unknown"))
}
