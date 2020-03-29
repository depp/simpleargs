//! Simple argument parser

#![deny(missing_docs)]

use std::ffi::{OsStr, OsString};

mod error;
mod arg;

pub use error::UsageError;
use arg::{parse_arg, ParsedArg};

/// A stream of arguments.
pub struct Args<T> {
    args: T,
    allow_options: bool,
}

impl<T> Args<T> {
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
    pub fn from_args(args: T) -> Self {
        Args {
            args,
            allow_options: true,
        }
    }
}

impl<T> Args<T>
where
    T: Iterator<Item = OsString>,
{
    /// Get the next argument in the stream.
    ///
    /// This consumes the stream. The remainder of the stream must be taken from the result.
    pub fn next(self) -> Result<Arg<T>, UsageError> {
        let Args {
            mut args,
            allow_options,
        } = self;
        let arg = match args.next() {
            None => return Ok(Arg::End),
            Some(arg) => arg,
        };
        let arg = if allow_options {
            match parse_arg(arg) {
                Ok(x) => x,
                Err(arg) => return Err(UsageError::InvalidArgument { arg }),
            }
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
pub struct NamedArgument<T> {
    option: String,
    option_value: Option<OsString>,
    args: T,
}

impl<T> NamedArgument<T> {
    /// The argument name, without leading dashes.
    ///
    /// For example, in "--out=xyz", the argument name is "out".
    pub fn name(&self) -> &str {
        self.option.as_ref()
    }

    /// Return an error for an unknown argument.
    pub fn unknown(self) -> UsageError {
        UsageError::UnknownOption {
            option: self.option,
        }
    }
}

impl<T> NamedArgument<T>
where
    T: Iterator<Item = OsString>,
{
    /// Consume the argument value as an OsStr.
    pub fn value_osstr(self) -> Result<(String, OsString, Args<T>), UsageError> {
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
    pub fn parse_osstr<U, F: FnOnce(&OsStr) -> Option<U>>(
        self,
        f: F,
    ) -> Result<(String, U, Args<T>), UsageError> {
        let (option, value, rest) = self.value_osstr()?;
        match f(value.as_ref()) {
            None => Err(UsageError::OptionInvalidValue { option, value }),
            Some(x) => Ok((option, x, rest)),
        }
    }

    /// Consume the argument value as a string.
    pub fn value_str(self) -> Result<(String, String, Args<T>), UsageError> {
        self.parse_osstr(|s| s.to_str().map(String::from))
    }

    /// Consume the argument value by parsing a string.
    pub fn parse_str<U, F: FnOnce(&str) -> Option<U>>(
        self,
        f: F,
    ) -> Result<(String, U, Args<T>), UsageError> {
        self.parse_osstr(|s| s.to_str().and_then(|s| f(String::from(s).as_str())))
    }

    /// Consume the argument, returning an error if it has an associated value.
    pub fn no_value(self) -> Result<(String, Args<T>), UsageError> {
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
}

/// A single argument in a stream of arguments.
pub enum Arg<T> {
    /// A positional argument.
    Positional(OsString, Args<T>),
    /// A named argument, possibly with an associated value.
    Named(NamedArgument<T>),
    /// End of the argument stream.
    End,
}
