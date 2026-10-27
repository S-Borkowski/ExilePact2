//! Which `Client.txt` the overlay follows: the `--log` option, else the
//! `log_path` setting, else the first of the known install locations whose
//! log exists. The option and the setting are taken as given even when the
//! file is not there yet (the tailer waits for it); discovery only names a
//! file that exists. Existence is a metadata read: nothing is opened.

use std::path::{Path, PathBuf};

/// The standalone client's log (measured, `docs/RUNTIME_DATA_MODELS.md`).
pub const STANDALONE_LOG: &str =
    r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt";

/// The Steam client's log. UNVERIFIED: there is no Steam install on the
/// machine the research ran on.
pub const STEAM_LOG: &str =
    r"C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2\logs\Client.txt";

/// The install locations discovery tries, in order.
pub fn default_candidates() -> Vec<PathBuf> {
    vec![PathBuf::from(STANDALONE_LOG), PathBuf::from(STEAM_LOG)]
}

/// Where the followed log came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Option,
    Setting,
    Discovered,
}

impl LogSource {
    pub fn as_str(self) -> &'static str {
        match self {
            LogSource::Option => "--log",
            LogSource::Setting => "the log path setting",
            LogSource::Discovered => "discovery",
        }
    }
}

/// The log to follow and where the choice came from, or `None` when no
/// option or setting names one and no candidate exists. `exists` is
/// injected so tests never look at the real install folders.
pub fn log_path(
    option: Option<&Path>,
    setting: Option<&str>,
    candidates: &[PathBuf],
    exists: impl Fn(&Path) -> bool,
) -> Option<(PathBuf, LogSource)> {
    if let Some(p) = option.filter(|p| !p.as_os_str().is_empty()) {
        return Some((p.to_path_buf(), LogSource::Option));
    }
    if let Some(s) = setting.map(str::trim).filter(|s| !s.is_empty()) {
        return Some((PathBuf::from(s), LogSource::Setting));
    }
    candidates
        .iter()
        .find(|c| exists(c))
        .map(|c| (c.clone(), LogSource::Discovered))
}

/// [`log_path`] over the real file system: a candidate exists when its
/// metadata says it is a file.
pub fn log_path_on_disk(
    option: Option<&Path>,
    setting: Option<&str>,
    candidates: &[PathBuf],
) -> Option<(PathBuf, LogSource)> {
    log_path(option, setting, candidates, |p| p.is_file())
}
