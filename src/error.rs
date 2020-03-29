use std::ffi::OsString;
use std::fmt;

/// A command-line usage error, for when the user has passed incorrect arguments to the program.
#[derive(Debug, Clone)]
pub enum UsageError {
    /// Indicates an argument has invalid syntax. Used for arguments which cannot be parsed.
    InvalidArgument {
        /// Full text of the argument.
        arg: OsString,
    },
    /// Indicates an argument was unexpected. Used for positional arguments.
    UnexpectedArgument {
        /// Full text of the argument.
        arg: OsString,
    },
    /// Indicates an expected positional argument was missing.
    MissingArgument {
        /// The name of the argument.
        name: String,
    },
    /// Indicates that a named option was unrecognized.
    UnknownOption {
        /// The name of the option, without leading dashes.
        option: String,
    },
    /// Indicates that a named option requires a parameter, but no parameter was supplied.
    OptionMissingParameter {
        /// The name of the option, without leading dashes.
        option: String,
    },
    /// Indicates that a named option does not take a parameter, but one was supplied anyway.
    OptionUnexpectedParameter {
        /// The name of the option, without leading dashes.
        option: String,
    },
    /// Indicates that the parameter for a named option could not be parsed or was invalid.
    OptionInvalidValue {
        /// The name of the option, without leading dashes.
        option: String,
        /// The parameter value which could not be parsed.
        value: OsString,
    },
    /// A free-form usage error string.
    Custom {
        /// The error text.
        text: String,
    },
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use UsageError::*;
        match self {
            InvalidArgument { arg } => write!(f, "invalid argument {:?}", arg),
            UnexpectedArgument { arg } => write!(f, "unexpected argument {:?}", arg),
            MissingArgument { name } => write!(f, "missing argument <{}>", name),
            UnknownOption { option } => write!(f, "unknown option -{}", option),
            OptionMissingParameter { option } => write!(f, "option -{} requires parameter", option),
            OptionUnexpectedParameter { option } => write!(f, "unknown option {:?}", option),
            OptionInvalidValue { option, value } => {
                write!(f, "invalid value for -{}: {:?}", option, value)
            }
            Custom { text } => f.write_str(text),
        }
    }
}
