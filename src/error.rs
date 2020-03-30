use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// An error for an invalid named argument.
#[derive(Debug)]
pub enum OptionError {
    /// The named argument is unrecognized.
    ///
    /// For example, the user passed `--xyz` to the program, but there is no option named `"xyz"`.
    Unknown,

    /// The named argument requires a parameter, but no parameter was supplied.
    ///
    /// For example, the program accepts `--output=<file>`, but an argument was passed as `--output`
    /// with no parameter.
    MissingParameter,

    /// The named argument does not accept a parameter, but one was supplied.
    ///
    /// For example, the program accepts `--verbose`, but an argument was passed as `--verbose=3`.
    UnexpectedParameter,

    /// The named argument was passed a value which is not valid unicode.
    InvalidUnicode,

    /// The value for the named argument was invalid.
    ///
    /// For example, the program accepts `--jobs=<N>` with integer N, but the user passed in
    /// `--jobs=xyz`.
    InvalidValue(Box<dyn Error>),
}

impl<T> From<T> for OptionError
where
    T: Error + 'static,
{
    fn from(x: T) -> OptionError {
        OptionError::InvalidValue(Box::new(x))
    }
}

/// A command-line usage error, for when the user has passed incorrect arguments to the program.
#[derive(Debug)]
pub enum UsageError<T> {
    /// Indicates an argument has invalid syntax. Used for arguments which cannot be parsed.
    InvalidArgument {
        /// Full text of the argument.
        arg: T,
    },

    /// Indicates an argument was unexpected. Used for positional arguments.
    UnexpectedArgument {
        /// Full text of the argument.
        arg: T,
    },

    /// Indicates an expected positional argument was missing.
    MissingArgument {
        /// The name of the argument.
        name: String,
    },

    /// Indicates an invalid named argument.
    InvalidOption {
        /// The name of the option without any leading dashes.
        name: String,
        /// The option parameter value, if it exists.
        value: Option<T>,
        /// The inner error from parsing the option.
        err: OptionError,
    },
}

impl<T> Display for UsageError<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UsageError::InvalidArgument { arg } => write!(f, "invalid argument {:?}", arg),
            UsageError::UnexpectedArgument { arg } => write!(f, "unexpected argument {:?}", arg),
            UsageError::MissingArgument { name } => write!(f, "missing argument <{}>", name),
            UsageError::InvalidOption { name, value, err } => match err {
                OptionError::Unknown => write!(f, "unknown option -{}", name),
                OptionError::MissingParameter => write!(f, "option -{} requires a parameter", name),
                OptionError::UnexpectedParameter => {
                    write!(f, "option -{} does not accept a parameter", name)
                }
                OptionError::InvalidUnicode => write!(
                    f,
                    "invalid value {:?} for option -{}: invalid Unicode string",
                    value.as_ref().unwrap(),
                    name
                ),
                OptionError::InvalidValue(err) => write!(
                    f,
                    "invalid value {:?} for option -{}: {}",
                    value.as_ref().unwrap(),
                    name,
                    err
                ),
            },
        }
    }
}

impl<T> Error for UsageError<T> where T: Debug {}
