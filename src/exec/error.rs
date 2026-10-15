/// Essentially an exception
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    ParseBool(std::str::ParseBoolError),
    ParseInt(std::num::ParseIntError),
    ParseFlt(std::num::ParseFloatError),
    ParseChar(std::char::ParseCharError),
    TryFromInt(std::num::TryFromIntError),
    Utf8Error(std::str::Utf8Error),
    OutOfBounds,
}

impl From<std::str::ParseBoolError> for Failure {
    fn from(value: std::str::ParseBoolError) -> Self {
        Self::ParseBool(value)
    }
}
impl From<std::num::ParseIntError> for Failure {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}
impl From<std::num::ParseFloatError> for Failure {
    fn from(value: std::num::ParseFloatError) -> Self {
        Self::ParseFlt(value)
    }
}
impl From<std::char::ParseCharError> for Failure {
    fn from(value: std::char::ParseCharError) -> Self {
        Self::ParseChar(value)
    }
}
impl From<std::num::TryFromIntError> for Failure {
    fn from(value: std::num::TryFromIntError) -> Self {
        Self::TryFromInt(value)
    }
}
impl From<std::str::Utf8Error> for Failure {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseBool(_) => write!(f, "could not parse boolean"),
            Self::ParseInt(_) => write!(f, "could not parse integer"),
            Self::ParseFlt(_) => write!(f, "could not parse float"),
            Self::ParseChar(_) => write!(f, "could not parse chararcter"),
            Self::TryFromInt(_) => write!(f, "could not convert integer"),
            Self::Utf8Error(_) => write!(f, "string contains invalid UTF-8"),
            Self::OutOfBounds => write!(
                f,
                "attempted to access a location outside the bounds of available memory"
            ),
        }
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseBool(e) => Some(e),
            Self::ParseInt(e) => Some(e),
            Self::ParseFlt(e) => Some(e),
            Self::ParseChar(e) => Some(e),
            Self::TryFromInt(e) => Some(e),
            Self::Utf8Error(e) => Some(e),

            _ => None,
        }
    }
}
