use std::fmt;

#[derive(Debug)]
pub enum Error {
    ParseError(String),
}

impl Error {
    #[inline]
    pub fn parse_error(input: String) -> Error {
        Error::ParseError(input)
    }

    pub fn get_parse_error(&self) -> Option<String> {
        match self {
            Error::ParseError(s) => Some(s.to_string()),
        }
    }

    pub fn is_parse_error(&self) -> bool {
        match self {
            Error::ParseError(_) => true,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseError(input) =>
                write!(f, "Failed to parse version: {}", input),
        }
    }
}
