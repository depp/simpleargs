# SimpleArgs: Simple Command-Line Argument Parsing for Rust

This is a simple, small library for parsing command-line arguments in Rust.

You write your own parser which iterates over the arguments. SimpleArgs interprets the raw arguments and gives you high-quality error messages.

## Example

```
use simpleargs::{Arg, Args, UsageError, OptionError};
use std::ffi::OsString;
use std::str::FromStr;

fn parse_args<T>(mut args: Args<T>) -> Result<(), UsageError<OsString>>
where
    T: Iterator<Item = OsString>,
{
    // The input file
    let mut input: Option<OsString> = None;
    // True if -flag was set
    let mut flag = false;
    // The value of -xvalue, if it was used
    let mut xvalue: Option<i32> = None;
    loop {
        match args.next() {
            Arg::Positional(arg) => if input.is_some() {
                return Err(UsageError::UnexpectedArgument { arg });
            } else {
                input = Some(arg)
            }
            Arg::Named(arg) => arg.parse(|name, value| match name {
                "flag" => {
                    // parse() above will return an error for -flag=value,
                    // because this function does not use 'value'.
                    flag = true;
                    Ok(())
                }
                "xvalue" => {
                    // Call as_str() for a str, or as_osstr() for OsStr.
                    xvalue = Some(i32::from_str(value.as_str()?)?);
                    Ok(())
                }
                _ => Err(OptionError::Unknown),
            })?,
            Arg::End => break,
            Arg::Error(err) => return Err(err),
        }
    }
    let input = match input {
        Some(path) => path,
        None => return Err(UsageError::MissingArgument { name: "input".to_owned() }),
    };
    Ok(())
}
```

## Goals and Non-Goals

- Simple argument parsing and nothing else. This library does not provide usage mesasges for your CLI utility (write your own), it does not validate arguments (do that yourself), and it does not collect arguments or put them into structs for you.

- Handle `OsString` or `String`, user’s choice. This library will correctly preserve invalid Unicode data if you want. You can do pathological things like pass `-flag=$'\xff'` to your command-line tools. However, if you don’t want this, you can just use the string methods instead.

- Decent error messages.

  ```
  $ my-tool -namee=abc
  Error: unknown option -namee
  $ my-tool -name=$'\xff'
  Error: invalid value "\xff" for -name: invalid Unicode string
  $ my-tool -count=1q
  Error: invalid value "0q" for option -count: invalid digit found in string
  ```

## Limitations

Known limitations we intend to fix:

- No Windows support yet! `OsString` is a different beast on Windows. No, we can’t write generic code that works both on Windows and non-Windows systems.

## Opinions

Known limitations that accepted as the library’s design:

- You don’t want to combine short options. You can have three separate options `-a`, `-b`, and `-c`, but you **cannot** combine all three into `-abc`. Combining short flags into one argument is only useful for the most commonly used interactive tools, like `ls`.

- There is no difference between `-option` and `--option`. One or two hyphens are treated identically.

- All options after `--` are treated as positional arguments.

- Options cannot accept multiple parameters. If you want something like `-pos <x> <y>`, you will have to write it as `-pos <x>,<y>`.

## Comparisons

- [Clap](https://github.com/clap-rs/clap) is fancy and has tons of features. It lets you define how each argument is parsed or stored in a few different ways. SimpleArgs is for people who prefer a more minimal, explicit approach.

- [Docopt](https://github.com/docopt/docopt.rs) is for people who want to write the documentation first, and have a library generate a parser from the documentation. SimpleArgs is for people who want to write the parser separately.

- [StructOpt](https://github.com/TeXitoi/structopt) is essentially a flavor of Clap.

- [getopts](https://docs.rs/getopts/0.2.21/getopts/) is somewhat more simple and limited.

- [Seahorse](https://github.com/ksk001100/seahorse) is an entire command-line tool framework.
