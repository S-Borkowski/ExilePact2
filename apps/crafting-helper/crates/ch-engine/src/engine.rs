//! Loading blueprints: the structural check is `ch-blueprint`'s; this adds
//! the data-backed one (an essence, alloy or liquid emotion id must exist
//! for at least one of the blueprint's classes) and holds the result.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use ch_blueprint::catalogue::{Currency, Kind};
use ch_blueprint::{Blueprint, Checked, Diagnostic};

use crate::catalogue::data_entries;

/// A blueprint that passed the structural and the data-backed checks.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub checked: Checked,
}

impl Resolved {
    pub fn id(&self) -> &str {
        self.checked.blueprint.id.get_ref()
    }

    pub fn blueprint(&self) -> &Blueprint {
        &self.checked.blueprint
    }

    pub fn path(&self) -> &str {
        &self.checked.path
    }
}

/// Checks that every essence, alloy and liquid emotion id the blueprint
/// names exists in the data of at least one of its classes. Every failure
/// names the id's line and column.
pub fn resolve(checked: Checked) -> Result<Resolved, Vec<Diagnostic>> {
    let blueprint = &checked.blueprint;
    let classes: Vec<_> = blueprint
        .classes
        .get_ref()
        .iter()
        .filter_map(|c| ch_data::class_data_by_name(c.get_ref()))
        .collect();
    let class_names: Vec<&str> = blueprint
        .classes
        .get_ref()
        .iter()
        .map(|c| c.get_ref().as_str())
        .collect();

    let mut errors = Vec::new();
    for step in &blueprint.steps {
        let step = step.get_ref();
        for action in &step.actions {
            let currency = &action.get_ref().currency;
            let id = currency.get_ref().as_str();
            let Some(Currency::DataBacked(shape)) = Currency::of(id) else {
                continue;
            };
            if classes
                .iter()
                .any(|data| !data_entries(data, id).is_empty())
            {
                continue;
            }
            let what = match shape.kind {
                Kind::Alloy => "alloy",
                Kind::Liquid => "liquid emotion",
                _ => "essence",
            };
            errors.push(checked.at(
                currency.span(),
                format!(
                    "step `{}`: no {what} `{id}` in the data of {}",
                    step.id.get_ref(),
                    class_names.join(", ")
                ),
            ));
        }
    }
    if errors.is_empty() {
        Ok(Resolved { checked })
    } else {
        Err(errors)
    }
}

/// Reads, checks and resolves one blueprint file.
pub fn check_path(path: &Path) -> Result<Resolved, Vec<Diagnostic>> {
    resolve(ch_blueprint::check_file(path)?)
}

/// The `.toml` files a path names: the file itself, or a folder's
/// `.toml` files (not its subfolders), sorted by name.
pub fn toml_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "toml"))
            .collect();
        files.sort();
        Ok(files)
    } else if path.is_file() {
        Ok(vec![path.to_path_buf()])
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no such file or folder",
        ))
    }
}

/// The blueprints the engine plans with, in load order (the shipped ones
/// sorted by file name), which is the order routes tie-break on.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Engine {
    blueprints: Vec<Resolved>,
}

impl Engine {
    /// The blueprints embedded from `apps/crafting-helper/blueprints/`.
    pub fn shipped() -> Result<Engine, Vec<Diagnostic>> {
        let mut checked = Vec::new();
        let mut errors = Vec::new();
        for result in ch_blueprint::check_shipped() {
            match result {
                Ok(c) => checked.push(c),
                Err(e) => errors.extend(e),
            }
        }
        let engine = Engine::new(checked);
        match engine {
            Ok(engine) if errors.is_empty() => Ok(engine),
            Ok(_) => Err(errors),
            Err(more) => {
                errors.extend(more);
                Err(errors)
            }
        }
    }

    /// Resolves structurally checked blueprints, in the order given.
    pub fn new(checked: Vec<Checked>) -> Result<Engine, Vec<Diagnostic>> {
        let mut resolved = Vec::new();
        let mut errors = Vec::new();
        for c in checked {
            match resolve(c) {
                Ok(r) => resolved.push(r),
                Err(e) => errors.extend(e),
            }
        }
        match Engine::from_resolved(resolved) {
            Ok(engine) if errors.is_empty() => Ok(engine),
            Ok(_) => Err(errors),
            Err(more) => {
                errors.extend(more);
                Err(errors)
            }
        }
    }

    /// Reads, checks and resolves every `.toml` file the paths name (a
    /// folder's files sorted by name).
    pub fn load(paths: &[PathBuf]) -> Result<Engine, Vec<Diagnostic>> {
        let mut resolved = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            let files = match toml_files(path) {
                Ok(files) => files,
                Err(e) => {
                    errors.push(Diagnostic {
                        path: path.display().to_string(),
                        line: 1,
                        col: 1,
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            for file in files {
                match check_path(&file) {
                    Ok(r) => resolved.push(r),
                    Err(e) => errors.extend(e),
                }
            }
        }
        match Engine::from_resolved(resolved) {
            Ok(engine) if errors.is_empty() => Ok(engine),
            Ok(_) => Err(errors),
            Err(more) => {
                errors.extend(more);
                Err(errors)
            }
        }
    }

    /// Holds resolved blueprints; two with one id are refused.
    pub fn from_resolved(blueprints: Vec<Resolved>) -> Result<Engine, Vec<Diagnostic>> {
        let mut seen: HashMap<String, String> = HashMap::new();
        let mut errors = Vec::new();
        for r in &blueprints {
            if let Some(first) = seen.get(r.id()) {
                errors.push(r.checked.at(
                    r.blueprint().id.span(),
                    format!("blueprint id `{}` is also the id of {first}", r.id()),
                ));
            } else {
                seen.insert(r.id().to_string(), r.path().to_string());
            }
        }
        if errors.is_empty() {
            Ok(Engine { blueprints })
        } else {
            Err(errors)
        }
    }

    pub fn blueprints(&self) -> &[Resolved] {
        &self.blueprints
    }

    pub fn blueprint(&self, id: &str) -> Option<&Resolved> {
        self.blueprints.iter().find(|b| b.id() == id)
    }
}
