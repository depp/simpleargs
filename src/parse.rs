use crate::error::UsageError;
use std::ffi::OsString;

fn is_arg_name(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true,
        _ => false,
    }
}

pub fn parse_arg(arg: OsString) -> Result<ParsedArg, UsageError> {
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

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedArg {
    Positional(OsString),            // A positional argument.
    EndOfFlags,                      // The "--" argument.
    Named(String, Option<OsString>), // A named option -opt or -opt=value.
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ascii::escape_default;
    use std::ffi::OsStr;
    use std::fmt;
    use std::os::unix::ffi::OsStrExt;

    struct Str<'a>(&'a [u8]);

    impl<'a> fmt::Display for Str<'a> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match std::str::from_utf8(self.0) {
                Ok(s) => fmt::Debug::fmt(s, f),
                _ => {
                    let mut s = String::new();
                    s.push_str("b\"");
                    for &b in self.0.iter() {
                        for c in escape_default(b) {
                            s.push(c as char);
                        }
                    }
                    s.push('"');
                    f.write_str(&s)
                }
            }
        }
    }

    fn osstr(s: &[u8]) -> OsString {
        OsString::from(OsStr::from_bytes(s))
    }

    #[test]
    fn parse_success() {
        use ParsedArg::*;
        let cases: &[(&[u8], ParsedArg)] = &[
            (b"abc", Positional(osstr(b"abc"))),
            (b"", Positional(osstr(b""))),
            (b"-", Positional(osstr(b"-"))),
            (b"--", EndOfFlags),
            (b"-a", Named("a".to_owned(), None)),
            (b"--a", Named("a".to_owned(), None)),
            (b"-a=", Named("a".to_owned(), Some(osstr(b"")))),
            (b"--a=", Named("a".to_owned(), Some(osstr(b"")))),
            (b"--arg-name", Named("arg-name".to_owned(), None)),
            (b"--ARG_NAME", Named("ARG_NAME".to_owned(), None)),
            (
                b"--opt=value",
                Named("opt".to_owned(), Some(osstr(b"value"))),
            ),
        ];
        let mut success = true;
        for (input, expected) in cases.iter() {
            match parse_arg(osstr(input)) {
                Ok(arg) => {
                    if &arg != expected {
                        success = false;
                        eprintln!(
                            "parse_arg({}): got {:?}, expect {:?}",
                            Str(input),
                            expected,
                            arg
                        );
                    }
                }
                Err(e) => {
                    success = false;
                    eprintln!("parse_arg({}): got error {:?}", Str(input), e);
                }
            }
        }
        if !success {
            panic!("failed");
        }
    }

    #[test]
    fn parse_failure() {
        let cases: &[(&[u8], UsageError)] = &[
            (b"-=", UsageError::InvalidArgument { arg: osstr(b"-=") }),
            (
                b"--=xyz",
                UsageError::InvalidArgument {
                    arg: osstr(b"--=xyz"),
                },
            ),
        ];
        let mut success = true;
        for (input, expected) in cases.iter() {
            match parse_arg(osstr(input)) {
                Ok(arg) => {
                    success = false;
                    eprintln!(
                        "parse_arg({}): got {:?}, expect error {:?}",
                        Str(input),
                        arg,
                        expected
                    );
                }
                Err(e) => {
                    if &e != expected {
                        success = false;
                        eprintln!(
                            "parse_arg({}): got {:?}, expect {:?}",
                            Str(input),
                            e,
                            expected
                        );
                    }
                }
            }
        }
        if !success {
            panic!("failed");
        }
    }
}
