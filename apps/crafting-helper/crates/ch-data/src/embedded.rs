//! The data files, embedded at build time and parsed once, lazily: the index
//! on first use, each class file the first time that class is asked for.

use std::sync::OnceLock;

use ch_item::ItemClass;

use crate::model::{ClassData, Index};

macro_rules! class_file {
    ($file:literal) => {
        (
            concat!("classes/", $file),
            include_str!(concat!("../../../data/classes/", $file)),
        )
    };
}

const INDEX_JSON: &str = include_str!("../../../data/index.json");

/// Every embedded class file: its path relative to `data/` and its text, in
/// `index.json`'s order.
pub const EMBEDDED_CLASSES: [(&str, &str); 22] = [
    class_file!("gloves.json"),
    class_file!("boots.json"),
    class_file!("helmets.json"),
    class_file!("body-armours.json"),
    class_file!("shields.json"),
    class_file!("quivers.json"),
    class_file!("bucklers.json"),
    class_file!("foci.json"),
    class_file!("wands.json"),
    class_file!("one-hand-maces.json"),
    class_file!("sceptres.json"),
    class_file!("spears.json"),
    class_file!("bows.json"),
    class_file!("staves.json"),
    class_file!("two-hand-maces.json"),
    class_file!("quarterstaves.json"),
    class_file!("crossbows.json"),
    class_file!("talismans.json"),
    class_file!("belts.json"),
    class_file!("amulets.json"),
    class_file!("rings.json"),
    class_file!("jewels.json"),
];

static INDEX: OnceLock<Index> = OnceLock::new();
static CLASSES: [OnceLock<ClassData>; 22] = [const { OnceLock::new() }; 22];

/// The parsed `data/index.json`.
///
/// Panics if the embedded file does not parse: it is fixed at build time
/// and `embedded_classes_match_index` parses it.
pub fn index() -> &'static Index {
    INDEX.get_or_init(|| {
        serde_json::from_str(INDEX_JSON)
            .unwrap_or_else(|e| panic!("embedded data/index.json does not parse: {e}"))
    })
}

/// The class file at `position` in [`EMBEDDED_CLASSES`], parsed on first
/// use.
pub fn embedded_class(position: usize) -> &'static ClassData {
    CLASSES[position].get_or_init(|| {
        let (file, text) = EMBEDDED_CLASSES[position];
        serde_json::from_str(text)
            .unwrap_or_else(|e| panic!("embedded data/{file} does not parse: {e}"))
    })
}

/// The data of `class`, matched on `ItemClass::as_str()` against the
/// index; `None` for `ItemClass::Other`, which is not in scope.
pub fn class_data(class: &ItemClass) -> Option<&'static ClassData> {
    if matches!(class, ItemClass::Other(_)) {
        return None;
    }
    class_data_by_name(class.as_str())
}

/// The data of the class named as the `Item Class:` line writes it.
pub fn class_data_by_name(name: &str) -> Option<&'static ClassData> {
    let entry = index().classes.iter().find(|c| c.class == name)?;
    let position = EMBEDDED_CLASSES
        .iter()
        .position(|(file, _)| *file == entry.file)?;
    Some(embedded_class(position))
}
