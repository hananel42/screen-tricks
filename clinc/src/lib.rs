use std::error::Error;
use std::fmt;
use std::str::FromStr;

// ==========================================
// Token Definition
// ==========================================

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    /// A short flag, e.g., "-v"
    Short(&'a str),
    /// A long flag, e.g., "--verbose"
    Long(&'a str),
    /// A positional value, e.g., "config.json"
    Value(&'a str),
}

// ==========================================
// Non-Generic Parse Error
// ==========================================

#[derive(Debug)]
pub enum ParseError {
    /// Occurs when an expected argument value is completely missing.
    MissingValue,
    /// Occurs when a flag is encountered instead of an expected positional value.
    UnexpectedFlag(&'static str),
    /// Occurs when a value is found but fails to parse into the target type.
    InvalidValue { value: String, error: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MissingValue => write!(f, "Missing expected value for argument"),
            ParseError::UnexpectedFlag(flag_type) => {
                write!(f, "Expected a value, but found a {} flag", flag_type)
            }
            ParseError::InvalidValue { value, error } => {
                write!(f, "Invalid value '{}' provided: {}", value, error)
            }
        }
    }
}

impl Error for ParseError {}

// ==========================================
// Argument Parser Struct & Implementation
// ==========================================

pub struct Parser {
    args_ptr: *mut [String],
    cursor: usize,
}

impl Parser {
    /// Initializes the parser by safely gathering arguments directly from the environment.
    pub fn from_env() -> Self {
        let args_vec: Vec<String> = std::env::args().skip(1).collect();
        let boxed_slice = args_vec.into_boxed_slice();
        let args_ptr = Box::into_raw(boxed_slice);

        Parser {
            args_ptr,
            cursor: 0,
        }
    }

    /// Fetches the next CLI token, bound to the lifetime of the parser.
    pub fn next(&mut self) -> Option<Token<'_>> {
        // SAFETY: The raw pointer is guaranteed to be valid for the entire lifecycle
        // of the Parser struct and is exclusively freed when the struct is dropped.
        let args = unsafe { &*self.args_ptr };

        if self.cursor >= args.len() {
            return None;
        }

        let s = args[self.cursor].as_str();
        self.cursor += 1;

        Some(match s {
            s if s.starts_with("--") => Token::Long(&s[2..]),
            s if s.starts_with('-') && s != "-" => Token::Short(&s[1..]),
            _ => Token::Value(s),
        })
    }

    /// Extracts the next token and attempts to parse it into the requested type `T`.
    pub fn parse<T: FromStr>(&mut self) -> Result<T, ParseError>
    where
        T::Err: fmt::Display,
    {
        match self.next() {
            None => Err(ParseError::MissingValue),
            Some(token) => match token {
                Token::Value(a) => T::from_str(a).map_err(|e| ParseError::InvalidValue {
                    value: a.to_string(),
                    error: e.to_string(),
                }),
                Token::Long(_) => Err(ParseError::UnexpectedFlag("Long")),
                Token::Short(_) => Err(ParseError::UnexpectedFlag("Short")),
            },
        }
    }
}

// ==========================================
// Memory Cleanup (RAII)
// ==========================================

impl Drop for Parser {
    fn drop(&mut self) {
        // SAFETY: Reconstructs the owned Box from the raw pointer to ensure
        // that all string vectors are fully deallocated from the heap without leaks.
        unsafe {
            let _boxed = Box::from_raw(self.args_ptr);
        }
    }
}
