//! Simple argument parser
//!
//! The goals of this library are to correctly handle OsString and produce high-quality error
//! messages.
//!
//! This library is like the traditional `getopt` with better error reporting. It converts an
//! iterator of [`String`] or [`OsString`] to positional arguments and named arguments.
//!
//! Single and double hyphens are considered equivalent. This means that `-help` and `--help` are
//! equivalent. Opinion: Most new projects should parse each argument as a separate flag. So, `-abc`
//! should be parsed as one argument named `"abc"`, not three arguments named `"a"`, `"b"`, and
//! `"c"`. Combining multiple flags into one argument is confusing, so it should only be used for
//! programs that are called interactively very frequently, like `ls`.
//!
//! Options which take values can take values either as one argument, `-option=value`, or as two
//! arguments, `-option value`.
//!
//! [`String`]: std::string::String
//! [`OsString`]: std::ffi::OsString
//!
//! # Example
//!
//! ```
//! use simpleargs::{Arg, Args, UsageError, OptionError};
//! use std::ffi::OsString;
//! use std::str::FromStr;
//!
//! fn parse_args<T>(mut args: Args<T>) -> Result<(), UsageError<OsString>>
//! where
//!     T: Iterator<Item = OsString>,
//! {
//!     // The input file
//!     let mut input: Option<OsString> = None;
//!     // True if -flag was set
//!     let mut flag = false;
//!     // The value of -xvalue, if it was used
//!     let mut xvalue: Option<i32> = None;
//!     loop {
//!         match args.next() {
//!             Arg::Positional(arg) => if input.is_some() {
//!                 return Err(UsageError::UnexpectedArgument { arg });
//!             } else {
//!                 input = Some(arg)
//!             }
//!             Arg::Named(arg) => arg.parse(|name, value| match name {
//!                 "flag" => {
//!                     // parse() above will return an error for -flag=value,
//!                     // because this function does not use 'value'.
//!                     flag = true;
//!                     Ok(())
//!                 }
//!                 "xvalue" => {
//!                     // Call as_str() for a str, or as_osstr() for OsStr.
//!                     xvalue = Some(i32::from_str(value.as_str()?)?);
//!                     Ok(())
//!                 }
//!                 _ => Err(OptionError::Unknown),
//!             })?,
//!             Arg::End => break,
//!             Arg::Error(err) => return Err(err),
//!         }
//!     }
//!     let input = match input {
//!         Some(path) => path,
//!         None => return Err(UsageError::MissingArgument { name: "input".to_owned() }),
//!     };
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]

pub mod arg;
mod error;

use std::ffi::OsStr;

pub use arg::{ArgString, ParsedArg};
pub use error::{OptionError, UsageError};

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
    ///     let args = Args::from(args_os);
    /// }
    /// ```
    pub fn from(args: T) -> Self {
        Args {
            args,
            allow_options: true,
        }
    }

    /// Get the remaining unparsed arguments in the stream.
    pub fn rest(self) -> T {
        self.args
    }
}

impl<T> Args<T>
where
    T: Iterator,
    <T as Iterator>::Item: ArgString,
{
    /// Get the next argument in the stream.
    pub fn next<'a>(&'a mut self) -> Arg<'a, T> {
        let arg = match self.args.next() {
            None => return Arg::End,
            Some(arg) => arg,
        };
        if !self.allow_options {
            return Arg::Positional(arg);
        }
        let arg = match arg.parse_arg() {
            Err(arg) => return Arg::Error(UsageError::InvalidArgument { arg }),
            Ok(arg) => arg,
        };
        match arg {
            ParsedArg::Positional(arg) => Arg::Positional(arg),
            ParsedArg::EndOfFlags => {
                self.allow_options = false;
                match self.args.next() {
                    None => Arg::End,
                    Some(arg) => Arg::Positional(arg),
                }
            }
            ParsedArg::Named(name, data) => Arg::Named(NamedArgument {
                name,
                data,
                args: self,
            }),
        }
    }
}

/// A single argument in a stream of arguments.
pub enum Arg<'a, T>
where
    T: Iterator,
{
    /// A positional argument.
    Positional(T::Item),
    /// A named argument, possibly with an associated value.
    Named(NamedArgument<'a, T>),
    /// End of the argument stream.
    End,
    /// Invalid argument syntax.
    Error(UsageError<T::Item>),
}

/// A named command-line argument which may or may not have an associated value.
pub struct NamedArgument<'a, T>
where
    T: Iterator,
{
    name: String,
    data: Option<<T as Iterator>::Item>,
    args: &'a mut Args<T>,
}

impl<'a, T> NamedArgument<'a, T>
where
    T: Iterator,
    <T as Iterator>::Item: ArgString,
{
    /// Parse the named command-line option.
    ///
    /// The option name and value are passed to the supplied function. Any errors that the function
    /// returns are annotated with information about the option.
    ///
    /// An error is returned if the user supplied a value, but [`as_str`] or [`as_osstr`] is not
    /// called.
    ///
    /// [`as_str`]: simpleargs::Value::as_str
    /// [`as_osstr`]: simpleargs::Value::as_osstr
    pub fn parse<U, F>(self, f: F) -> Result<U, UsageError<<T as Iterator>::Item>>
    where
        for<'b> F: FnOnce(&'b str, Value<'b, T>) -> Result<U, OptionError>,
    {
        let NamedArgument {
            name,
            mut data,
            args,
        } = self;
        let mut consumed = false;
        let err = match f(
            &name,
            Value {
                data: &mut data,
                args,
                consumed: &mut consumed,
            },
        ) {
            Err(err) => err,
            Ok(r) => {
                if consumed || data.is_none() {
                    return Ok(r);
                } else {
                    OptionError::UnexpectedParameter
                }
            }
        };
        Err(UsageError::InvalidOption {
            name,
            value: data,
            err,
        })
    }
}

/// A handle for getting the value associated with a named flag.UsageError
///
/// This handle can only be used once, and is consumed.
pub struct Value<'a, T>
where
    T: Iterator,
{
    data: &'a mut Option<<T as Iterator>::Item>,
    args: &'a mut Args<T>,
    consumed: &'a mut bool,
}

impl<'a, T> Value<'a, T>
where
    T: Iterator,
{
    /// Get the associated value.
    ///
    /// Returns an error if the user did not supply a value.
    fn value(self) -> Result<&'a T::Item, OptionError> {
        *self.consumed = true;
        match self.data {
            Some(x) => Ok(x),
            None => match self.args.args.next() {
                Some(x) => Ok(self.data.get_or_insert(x)),
                None => Err(OptionError::MissingParameter),
            },
        }
    }
}

impl<'a, T> Value<'a, T>
where
    T: Iterator,
    <T as Iterator>::Item: ArgString,
{
    /// Get the associated value as a string.
    ///
    /// Note that ownership of the string is not passed. Ownership is kept by the NamedArgument so
    /// it can be attached to error messages.
    ///
    /// Returns an error if the user did not supply a value.
    pub fn as_str(self) -> Result<&'a str, OptionError> {
        match self.value()?.to_str() {
            Some(x) => Ok(x),
            None => Err(OptionError::InvalidUnicode),
        }
    }

    /// Get the associated value as an OsStr.
    ///
    /// Note that ownership of the string is not passed. Ownership is kept by the NamedArgument so
    /// it can be attached to error messages.
    ///
    /// Returns an error if the user did not supply a value.
    pub fn as_osstr(self) -> Result<&'a OsStr, OptionError> {
        self.value().map(ArgString::to_osstr)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[derive(Debug, PartialEq, Eq)]
    struct Parsed {
        positional: Vec<String>,
        flag: bool,
        xvalue: Option<i32>,
    }

    fn parse_args(args: &'static [&'static str]) -> Result<Parsed, UsageError<String>> {
        let mut args = Args::from(args.iter().map(|&s| s.to_owned()));
        let mut positional = Vec::new();
        let mut flag = false;
        let mut xvalue = None;
        loop {
            match args.next() {
                Arg::Positional(x) => positional.push(x),
                Arg::Named(arg) => arg.parse(|name, arg| match name {
                    "flag" => {
                        flag = true;
                        Ok(())
                    }
                    "x" => {
                        xvalue = Some(i32::from_str(arg.as_str()?)?);
                        Ok(())
                    }
                    _ => Err(OptionError::Unknown),
                })?,
                Arg::End => break,
                Arg::Error(err) => return Err(err),
            }
        }
        Ok(Parsed {
            positional,
            flag,
            xvalue,
        })
    }

    #[test]
    fn success() {
        match parse_args(&["abc", "--flag", "-x=10", "--", "--", "--arg"]) {
            Err(err) => panic!("err: {:?}", err),
            Ok(r) => assert_eq!(
                r,
                Parsed {
                    positional: vec!["abc".to_owned(), "--".to_owned(), "--arg".to_owned()],
                    flag: true,
                    xvalue: Some(10),
                }
            ),
        }
    }

    #[test]
    fn no_param() {
        let r = parse_args(&["--x"]);
        if let Err(e) = &r {
            if let UsageError::InvalidOption { name, value, err } = e {
                assert_eq!(name, "x");
                assert!(value.is_none());
                if let OptionError::MissingParameter = err {
                    return;
                }
            }
        }
        panic!("incorrect result: {:?}", r);
    }

    #[test]
    fn bad_param() {
        let r = parse_args(&["-x", "0q"]);
        if let Err(e) = &r {
            if let UsageError::InvalidOption { name, value, err } = e {
                assert_eq!(name, "x");
                assert_eq!(value, &Some("0q".to_owned()));
                if let OptionError::InvalidValue(_) = err {
                    return;
                }
            }
        }
        panic!("incorrect result: {:?}", r);
    }
}
