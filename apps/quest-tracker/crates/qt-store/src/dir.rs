//! Where the store lives, and where it refuses to live.
//!
//! `QUEST_TRACKER_DIR` when set and non-empty, else
//! `%LOCALAPPDATA%\ExilePact2\quest-tracker\`. A folder inside the game's
//! install (one holding `PathOfExile.exe` or `PathOfExileSteam.exe`, or any
//! path under a `Grinding Gear Games` folder) or inside a git checkout (a
//! `.git` folder or file in it or above it) is refused: the store must never
//! write next to the game, and never leave a player's character names in a
//! repository.

use std::ffi::OsString;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use crate::StoreError;

/// The environment variable that overrides the store folder.
pub const DIR_ENV: &str = "QUEST_TRACKER_DIR";

/// Executables whose presence marks a folder as the game's install.
const GAME_EXES: [&str; 2] = ["PathOfExile.exe", "PathOfExileSteam.exe"];

/// The publisher folder both the standalone client and its documents use.
const PUBLISHER_DIR: &str = "Grinding Gear Games";

/// Why a folder was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The folder is inside the game install; `marker` is the executable or
    /// the publisher folder that showed it.
    GameInstall { marker: PathBuf },
    /// The folder is inside a git checkout whose `.git` is `marker`.
    GitCheckout { marker: PathBuf },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::GameInstall { marker } => {
                write!(f, "it is inside the game's install ({})", marker.display())
            }
            Refusal::GitCheckout { marker } => {
                write!(f, "it is inside a git checkout ({})", marker.display())
            }
        }
    }
}

/// Resolves the store folder from an override and `%LOCALAPPDATA%`, both
/// injected so tests never touch the process environment. An empty value
/// counts as unset.
pub fn resolve_dir(
    override_dir: Option<OsString>,
    local_app_data: Option<OsString>,
) -> Result<PathBuf, StoreError> {
    if let Some(d) = override_dir.filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(d));
    }
    match local_app_data.filter(|d| !d.is_empty()) {
        Some(base) => Ok(PathBuf::from(base).join("ExilePact2").join("quest-tracker")),
        None => Err(StoreError::NoDirectory),
    }
}

/// [`resolve_dir`] over this process's `QUEST_TRACKER_DIR` and `LOCALAPPDATA`.
pub fn default_dir() -> Result<PathBuf, StoreError> {
    resolve_dir(std::env::var_os(DIR_ENV), std::env::var_os("LOCALAPPDATA"))
}

/// Refuses a folder inside the game install or a git checkout. Reads only
/// metadata; creates and opens nothing. The folder need not exist yet: the
/// nearest existing ancestor is resolved through links, so a junction into
/// the game folder is caught too.
pub fn check_dir(dir: &Path) -> Result<(), StoreError> {
    let refused = |reason| StoreError::Refused {
        dir: dir.to_path_buf(),
        reason,
    };
    let abs = std::path::absolute(dir).map_err(|e| StoreError::io(dir, e))?;

    for candidate in [Some(abs.clone()), resolve_existing(&abs)]
        .into_iter()
        .flatten()
    {
        if let Some(marker) = publisher_component(&candidate) {
            return Err(refused(Refusal::GameInstall { marker }));
        }
        for a in candidate.ancestors() {
            for exe in GAME_EXES {
                let marker = a.join(exe);
                if marker.is_file() {
                    return Err(refused(Refusal::GameInstall { marker }));
                }
            }
            let git = a.join(".git");
            if git.exists() {
                return Err(refused(Refusal::GitCheckout { marker: git }));
            }
        }
    }
    Ok(())
}

/// `abs` with its longest existing prefix canonicalized, or `None` when no
/// prefix resolves.
fn resolve_existing(abs: &Path) -> Option<PathBuf> {
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    let mut cur = abs;
    loop {
        if let Ok(mut real) = cur.canonicalize() {
            for part in rest.iter().rev() {
                real.push(part);
            }
            return Some(real);
        }
        rest.push(cur.file_name()?);
        cur = cur.parent()?;
    }
}

/// The prefix of `p` ending in a `Grinding Gear Games` component, if any.
fn publisher_component(p: &Path) -> Option<PathBuf> {
    let mut acc = PathBuf::new();
    for c in p.components() {
        acc.push(c.as_os_str());
        if let Component::Normal(name) = c {
            if name.to_string_lossy().eq_ignore_ascii_case(PUBLISHER_DIR) {
                return Some(acc);
            }
        }
    }
    None
}
