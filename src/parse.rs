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

pub enum ParsedArg {
    Positional(OsString),            // A positional argument.
    EndOfFlags,                      // The "--" argument.
    Named(String, Option<OsString>), // A named option -opt or -opt=value.
}
