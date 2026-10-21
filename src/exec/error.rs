//! Runtime error

/// Essentially an exception
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Failure {
    ParseBool,
    ParseInt,
    ParseFlt,
    ParseChar,
    TryFromInt,
    Utf8Error,
    OutOfBounds,
    DivByZero,
    UnhandledOrNone,
    UnhandledOrNevr,
}

impl From<std::str::ParseBoolError> for Failure {
    fn from(_: std::str::ParseBoolError) -> Self {
        Self::ParseBool
    }
}
impl From<std::num::ParseIntError> for Failure {
    fn from(_value: std::num::ParseIntError) -> Self {
        Self::ParseInt // TODO: match against more specific errors
    }
}
impl From<std::num::ParseFloatError> for Failure {
    fn from(_value: std::num::ParseFloatError) -> Self {
        Self::ParseFlt // TODO: match against more specific errors
    }
}
impl From<std::char::ParseCharError> for Failure {
    fn from(_value: std::char::ParseCharError) -> Self {
        Self::ParseChar // TODO: match against more specific errors
    }
}
impl From<std::num::TryFromIntError> for Failure {
    fn from(_value: std::num::TryFromIntError) -> Self {
        Self::TryFromInt // TODO: match against more specific errors
    }
}
impl From<std::str::Utf8Error> for Failure {
    fn from(_value: std::str::Utf8Error) -> Self {
        Self::Utf8Error // TODO: match against more specific errors
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseBool => write!(f, "could not parse boolean"),
            Self::ParseInt => write!(f, "could not parse integer"),
            Self::ParseFlt => write!(f, "could not parse float"),
            Self::ParseChar => write!(f, "could not parse chararcter"),
            Self::TryFromInt => write!(f, "could not convert integer"),
            Self::Utf8Error => write!(f, "string contains invalid UTF-8"),
            Self::OutOfBounds => write!(
                f,
                "attempted to access a location outside the bounds of available memory"
            ),
            Self::DivByZero => write!(f, "tried to divide by zero"),
            Self::UnhandledOrNone => write!(f, "tried to perform an operation with none"),
            Self::UnhandledOrNevr => write!(f, "tried to perform an operation with nevr"),
        }
    }
}

impl std::error::Error for Failure {}
