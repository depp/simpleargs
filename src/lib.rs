//! Simple argument parser

#![deny(missing_docs)]

use std::env;
use std::ffi::{OsStr, OsString};
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

fn is_arg_name(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true,
        _ => false,
    }
}

fn parse_arg(arg: OsString) -> Result<ParsedArg, UsageError> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let bytes = arg.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'-' {
        return Ok(ParsedArg::Positional(arg));
    }
    let body = if bytes[1] != b'-' {
        &bytes[1..]
    } else if bytes.len() == 2 {
        return Ok(ParsedArg::EndOfFlags);
    } else {
        &bytes[2..]
    };
    let (name, value) = match body.iter().position(|&c| c == b'=') {
        None => (body, None),
        Some(idx) => (&body[..idx], Some(&body[idx + 1..])),
    };
    if name.len() == 0
        || name[0] == b'-'
        || name[name.len() - 1] == b'-'
        || !name.iter().all(|&c| is_arg_name(c as char))
    {
        return Err(UsageError::InvalidArgument { arg });
    }
    let name = Vec::from(name);
    let name = unsafe { String::from_utf8_unchecked(name) };
    let value = value.map(|v| OsString::from_vec(Vec::from(v)));
    Ok(ParsedArg::Named(name, value))
}

enum ParsedArg {
    Positional(OsString),            // A positional argument.
    EndOfFlags,                      // The "--" argument.
    Named(String, Option<OsString>), // A named option -opt or -opt=value.
}

/// A stream of arguments.
pub struct Args {
    args: env::ArgsOs,
    allow_options: bool,
}

impl Args {
    /// Create an argument stream from an argument iterator. The program name should not be included
    /// in the argument stream.
    ///
    /// ```
    /// use std::env;
    /// use simpleargs::Args;
    /// fn main() {
    ///     let mut args_os = env::args_os();
    ///     args_os.next(); // Discard program name.
    ///     let args = Args::from_args(args_os);
    /// }
    /// ```
    pub fn from_args(args: env::ArgsOs) -> Self {
        Args {
            args,
            allow_options: true,
        }
    }

    /// Get the next argument in the stream.
    ///
    /// This consumes the stream. The remainder of the stream must be taken from the result.
    pub fn next(self) -> Result<Arg, UsageError> {
        let Args {
            mut args,
            allow_options,
        } = self;
        let arg = match args.next() {
            None => return Ok(Arg::End),
            Some(arg) => arg,
        };
        let arg = if allow_options {
            parse_arg(arg)?
        } else {
            ParsedArg::Positional(arg)
        };
        Ok(match arg {
            ParsedArg::Positional(arg) => Arg::Positional(
                arg,
                Args {
                    args,
                    allow_options,
                },
            ),
            ParsedArg::EndOfFlags => match args.next() {
                None => Arg::End,
                Some(arg) => Arg::Positional(
                    arg,
                    Args {
                        args,
                        allow_options: false,
                    },
                ),
            },
            ParsedArg::Named(name, value) => Arg::Named(NamedArgument {
                option: name,
                option_value: value,
                args,
            }),
        })
    }
}

/// A named command-line argument.
pub struct NamedArgument {
    option: String,
    option_value: Option<OsString>,
    args: env::ArgsOs,
}

impl NamedArgument {
    /// The argument name, without leading dashes.
    ///
    /// For example, in "--out=xyz", the argument name is "out".
    pub fn name(&self) -> &str {
        self.option.as_ref()
    }

    /// Consume the argument value as an OsStr.
    pub fn value_osstr(self) -> Result<(String, OsString, Args), UsageError> {
        let NamedArgument {
            option,
            option_value,
            mut args,
        } = self;
        let value = match option_value {
            None => match args.next() {
                None => return Err(UsageError::OptionMissingParameter { option }),
                Some(value) => value,
            },
            Some(value) => value,
        };
        Ok((
            option,
            value,
            Args {
                args,
                allow_options: true,
            },
        ))
    }

    /// Consume the argument value by parsing an OsStr.
    pub fn parse_osstr<T, F: FnOnce(&OsStr) -> Option<T>>(
        self,
        f: F,
    ) -> Result<(String, T, Args), UsageError> {
        let (option, value, rest) = self.value_osstr()?;
        match f(value.as_ref()) {
            None => Err(UsageError::OptionInvalidValue { option, value }),
            Some(x) => Ok((option, x, rest)),
        }
    }

    /// Consume the argument value as a string.
    pub fn value_str(self) -> Result<(String, String, Args), UsageError> {
        self.parse_osstr(|s| s.to_str().map(String::from))
    }

    /// Consume the argument value by parsing a string.
    pub fn parse_str<T, F: FnOnce(&str) -> Option<T>>(
        self,
        f: F,
    ) -> Result<(String, T, Args), UsageError> {
        self.parse_osstr(|s| s.to_str().and_then(|s| f(String::from(s).as_str())))
    }

    /// Consume the argument, returning an error if it has an associated value.
    pub fn no_value(self) -> Result<(String, Args), UsageError> {
        let NamedArgument {
            option,
            option_value,
            args,
        } = self;
        if option_value.is_some() {
            return Err(UsageError::OptionUnexpectedParameter { option });
        }
        Ok((
            option,
            Args {
                args,
                allow_options: true,
            },
        ))
    }

    /// Return an error for an unknown argument.
    pub fn unknown(self) -> UsageError {
        UsageError::UnknownOption {
            option: self.option,
        }
    }
}

/// A single argument in a stream of arguments.
pub enum Arg {
    /// A positional argument.
    Positional(OsString, Args),
    /// A named argument, possibly with an associated value.
    Named(NamedArgument),
    /// End of the argument stream.
    End,
}
