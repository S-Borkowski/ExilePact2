//! The overlay's command line: `--log PATH`, `--data PATH`, `--from-start`.
//!
//! Each option also takes the `--name=value` form. An unknown argument or an
//! option without its value is refused with one sentence naming it; the shell
//! puts that sentence in the status line and the diagnostic log and starts
//! with no options rather than not at all.

use std::ffi::OsString;
use std::path::PathBuf;

/// What the command line asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The log to follow, ahead of the setting and discovery.
    pub log: Option<PathBuf>,
    /// A quest data file instead of the `data/quests.json` built in.
    pub data: Option<PathBuf>,
    /// Read the whole existing log as catch-up, ignoring any saved position.
    pub from_start: bool,
}

/// One line for `--help`-style messages and refusals.
pub const USAGE: &str = "usage: quest-tracker [--log PATH] [--data PATH] [--from-start]";

/// Parses the arguments after the program name.
pub fn parse<I, S>(args: I) -> Result<Options, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut out = Options::default();
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        let text = arg.to_string_lossy().into_owned();
        let (name, inline) = match text.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_string(), Some(OsString::from(v))),
            _ => (text.clone(), None),
        };
        match name.as_str() {
            "--log" | "--data" => {
                let value = match inline {
                    Some(v) => v,
                    None => args
                        .next()
                        .ok_or_else(|| format!("{name} needs a path ({USAGE})"))?,
                };
                if value.is_empty() {
                    return Err(format!("{name} needs a path ({USAGE})"));
                }
                let path = Some(PathBuf::from(value));
                if name == "--log" {
                    out.log = path;
                } else {
                    out.data = path;
                }
            }
            "--from-start" if inline.is_none() => out.from_start = true,
            _ => return Err(format!("unknown argument {text:?} ({USAGE})")),
        }
    }
    Ok(out)
}
