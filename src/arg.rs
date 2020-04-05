//! Low-level argument parsing.

use std::ffi::{OsStr, OsString};

/// Trait for string types that can be parsed as command-line arguments.
pub trait ArgString: Sized {
    /// Parse the string as a command-line argument.
    ///
    /// On failure, return the input.
    fn parse_arg(self) -> Result<ParsedArg<Self>, Self>;

    /// Convert the argument into a str if it is a valid Unicode string.
    fn to_str(&self) -> Option<&str>;

    /// Convert the argument into an OsStr.
    fn to_osstr(&self) -> &OsStr;
}

fn is_arg_name(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true,
        _ => false,
    }
}

impl ArgString for String {
    fn parse_arg(self) -> Result<ParsedArg<String>, String> {
        let mut chars = self.chars();
        match chars.next() {
            Some('-') => (),
            _ => return Ok(ParsedArg::Positional(self)),
        }
        let cur = chars.clone();
        match chars.next() {
            Some('-') => {
                if chars.as_str().is_empty() {
                    return Ok(ParsedArg::EndOfFlags);
                }
            }
            Some(_) => chars = cur,
            None => return Ok(ParsedArg::Positional(self)),
        }
        let body = chars.as_str();
        let (name, value) = match body.find('=') {
            Some(idx) => (&body[..idx], Some(&body[idx + 1..])),
            None => (body, None),
        };
        if name.is_empty() || !name.chars().all(is_arg_name) {
            return Err(self);
        }
        Ok(ParsedArg::Named(name.to_owned(), value.map(str::to_owned)))
    }

    fn to_str(&self) -> Option<&str> {
        Some(self)
    }

    fn to_osstr(&self) -> &OsStr {
        self.as_ref()
    }
}

impl ArgString for OsString {
    fn parse_arg(self) -> Result<ParsedArg<OsString>, OsString> {
        use os_str_bytes::{OsStrBytes, OsStringBytes};
        let bytes = self.to_bytes();
        if bytes.len() < 2 || bytes[0] != b'-' {
            return Ok(ParsedArg::Positional(self));
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
            return Err(self);
        }
        let name = Vec::from(name);
        let name = unsafe { String::from_utf8_unchecked(name) };
        let value = value.map(|v| unsafe { OsString::from_bytes_unchecked(v) });
        Ok(ParsedArg::Named(name, value))
    }

    fn to_str(&self) -> Option<&str> {
        OsStr::to_str(self)
    }

    fn to_osstr(&self) -> &OsStr {
        self
    }
}

/// A single command-line argument which has been parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedArg<T> {
    /// A positional argument.
    Positional(T),
    /// The "--" argument.
    EndOfFlags,
    /// A named option, such as "-opt" or "-opt=value".
    ///
    /// The leading dashes are removed from the name.
    Named(String, Option<T>),
}

impl<T> ParsedArg<T> {
    /// Map a `ParsedArg<T>` to a `ParsedArg<U>` by applying a function to the inner value.
    pub fn map<U, F>(self, f: F) -> ParsedArg<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ParsedArg::Positional(x) => ParsedArg::Positional(f(x)),
            ParsedArg::EndOfFlags => ParsedArg::EndOfFlags,
            ParsedArg::Named(x, y) => ParsedArg::Named(x, y.map(f)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::OsStr;
    use std::fmt::Debug;
    use std::os::unix::ffi::OsStrExt;

    fn osstr(s: &[u8]) -> OsString {
        OsString::from(OsStr::from_bytes(s))
    }

    struct Case<T>(T, ParsedArg<T>);

    impl<T> Case<T> {
        fn map<F, U>(self, f: F) -> Case<U>
        where
            F: Fn(T) -> U,
        {
            let Case(input, output) = self;
            Case(f(input), output.map(f))
        }
    }

    impl<T: Debug + Clone + ArgString + PartialEq<T>> Case<T> {
        fn test(&self) -> bool {
            let Case(input, expected) = self;
            match input.clone().parse_arg() {
                Ok(arg) => {
                    if &arg != expected {
                        eprintln!(
                            "{:?}.parse_arg(): got {:?}, expect {:?}",
                            input, expected, arg
                        );
                        false
                    } else {
                        true
                    }
                }
                Err(_) => {
                    eprintln!("{:?}.parse_arg(): got error, expect {:?}", input, expected);
                    false
                }
            }
        }
    }

    fn success_cases() -> Vec<Case<String>> {
        let mut cases = vec![
            Case("abc", ParsedArg::Positional("abc")),
            Case("", ParsedArg::Positional("")),
            Case("-", ParsedArg::Positional("-")),
            Case("--", ParsedArg::EndOfFlags),
            Case("-a", ParsedArg::Named("a".to_owned(), None)),
            Case("--a", ParsedArg::Named("a".to_owned(), None)),
            Case("-a=", ParsedArg::Named("a".to_owned(), Some(""))),
            Case("--a=", ParsedArg::Named("a".to_owned(), Some(""))),
            Case("--arg-name", ParsedArg::Named("arg-name".to_owned(), None)),
            Case("--ARG_NAME", ParsedArg::Named("ARG_NAME".to_owned(), None)),
            Case(
                "--opt=value",
                ParsedArg::Named("opt".to_owned(), Some("value")),
            ),
        ];
        cases.drain(..).map(|c| c.map(str::to_owned)).collect()
    }

    struct Fail<T>(T);

    impl<T: Debug + Clone + ArgString + PartialEq<T>> Fail<T> {
        fn test(&self) -> bool {
            let Fail(input) = self;
            match input.clone().parse_arg() {
                Ok(arg) => {
                    eprintln!("{:?}.parse_arg(): got {:?}, expect error", input, arg);
                    false
                }
                Err(e) => {
                    if &e != input {
                        eprintln!(
                            "{:?}.parse_arg(): got error {:?}, expect error {:?}",
                            input, e, input
                        );
                        false
                    } else {
                        true
                    }
                }
            }
        }
    }

    const FAIL_CASES: &'static [&'static str] =
        &["-\0", "--\n", "--\0=", "-=", "--=", "-=value", "--=xyz"];

    #[test]
    fn parse_string_success() {
        let mut success = true;
        for case in success_cases().drain(..) {
            if !case.test() {
                success = false;
            }
        }
        if !success {
            panic!("failed");
        }
    }

    #[test]
    fn parse_osstring_success() {
        let mut success = true;
        let mut cases: Vec<Case<OsString>> = success_cases()
            .drain(..)
            .map(|c| c.map(OsString::from))
            .collect();
        cases.push(Case(
            osstr(b"\x80\xff"),
            ParsedArg::Positional(osstr(b"\x80\xff")),
        ));
        cases.push(Case(
            osstr(b"--opt=\xff"),
            ParsedArg::Named("opt".to_owned(), Some(osstr(b"\xff"))),
        ));
        for case in cases.drain(..) {
            if !case.test() {
                success = false;
            }
        }
        if !success {
            panic!("failed");
        }
    }

    #[test]
    fn parse_string_failure() {
        let mut success = true;
        for &input in FAIL_CASES.iter() {
            if !Fail(input.to_owned()).test() {
                success = false;
            }
        }
        if !success {
            panic!("failed");
        }
    }

    #[test]
    fn parse_osstring_failure() {
        let mut success = true;
        let mut cases: Vec<OsString> = FAIL_CASES
            .iter()
            .map(|&s| OsString::from(s.to_owned()))
            .collect();
        for input in cases.drain(..) {
            if !Fail(input).test() {
                success = false;
            }
        }
        if !success {
            panic!("failed");
        }
    }
}
