//! The full catalogue: every crafting item a blueprint or the blocked list
//! may name. The static part (currencies, omens, bones, fluxes, catalysts)
//! is `ch-blueprint`'s; the data-backed part (essences, alloys and liquid
//! emotions) is read here from the modifier data, one id per English name.
//!
//! M5's blocked list offers [`catalogue`]; the settings store its ids.

use std::sync::OnceLock;

use ch_blueprint::catalogue::{
    data_backed, id_from_name, Effect, ItemGroup, Kind, OmenSpec, CATALOGUE,
};
use ch_data::{ClassData, Essence};
use serde::Serialize;

/// One crafting item the blocked list may name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogueItem {
    /// Lower kebab case, apostrophes dropped (`orb-of-annulment`).
    pub id: String,
    /// The English name the id is made from.
    pub name: String,
    pub kind: Kind,
    pub effect: Effect,
    /// Every modifier it adds is at least this level.
    pub min_modifier_level: Option<u32>,
    /// The item groups it applies to; empty for any.
    pub groups: Vec<ItemGroup>,
    /// The highest item level it may be used on.
    pub max_item_level: Option<u32>,
    /// `Some` for an omen.
    pub omen: Option<OmenSpec>,
    /// Data-backed only: the data classes it exists for, in `index.json`'s
    /// order. Empty for a static entry.
    pub classes: Vec<String>,
    /// An essence, alloy or liquid emotion, read from the data.
    pub data_backed: bool,
}

/// An essence or liquid emotion entry of one class's data.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DataEntry {
    pub essence: &'static Essence,
}

/// Every entry of `data` whose name gives `id`. One name may have several
/// entries (one per modifier it adds on different variants).
pub(crate) fn data_entries(data: &'static ClassData, id: &str) -> Vec<DataEntry> {
    data.essences
        .iter()
        .chain(data.liquid.iter())
        .filter(|e| id_from_name(&e.name) == id)
        .map(|essence| DataEntry { essence })
        .collect()
}

/// The static catalogue, then every data-backed id sorted by id. Built
/// once.
pub fn catalogue() -> &'static [CatalogueItem] {
    static ALL: OnceLock<Vec<CatalogueItem>> = OnceLock::new();
    ALL.get_or_init(build)
}

/// The catalogue entry for an id.
pub fn catalogue_item(id: &str) -> Option<&'static CatalogueItem> {
    catalogue().iter().find(|c| c.id == id)
}

fn build() -> Vec<CatalogueItem> {
    let mut out: Vec<CatalogueItem> = CATALOGUE
        .iter()
        .map(|e| CatalogueItem {
            id: e.id.to_string(),
            name: e.name.to_string(),
            kind: e.kind,
            effect: e.effect,
            min_modifier_level: e.min_modifier_level,
            groups: e.groups.to_vec(),
            max_item_level: e.max_item_level,
            omen: e.omen,
            classes: Vec::new(),
            data_backed: false,
        })
        .collect();

    let mut backed: Vec<CatalogueItem> = Vec::new();
    for class in &ch_data::index().classes {
        let Some(data) = ch_data::class_data_by_name(&class.class) else {
            continue;
        };
        let entries = data
            .essences
            .iter()
            .map(|e| (e, false))
            .chain(data.liquid.iter().map(|e| (e, true)));
        for (essence, liquid) in entries {
            let id = id_from_name(&essence.name);
            match backed.iter_mut().find(|c| c.id == id) {
                Some(found) => {
                    if !found.classes.contains(&data.class) {
                        found.classes.push(data.class.clone());
                    }
                }
                None => backed.push(data_item(id, essence, liquid, &data.class)),
            }
        }
    }
    backed.sort_by(|a, b| a.id.cmp(&b.id));
    out.extend(backed);
    out
}

fn data_item(id: String, essence: &Essence, liquid: bool, class: &str) -> CatalogueItem {
    let kind = if liquid {
        Kind::Liquid
    } else if essence.alloy == Some(true) {
        Kind::Alloy
    } else {
        Kind::Essence
    };
    // The validator reads the effect from the id's shape; keep the two
    // the same, and fall back on the data's `removes` for a shape it does
    // not know.
    let effect = data_backed(&id)
        .map(|d| d.effect)
        .unwrap_or(if essence.removes == Some(true) {
            Effect::Swap
        } else {
            Effect::Upgrade
        });
    CatalogueItem {
        id,
        name: essence.name.clone(),
        kind,
        effect,
        min_modifier_level: None,
        groups: Vec::new(),
        max_item_level: None,
        omen: None,
        classes: vec![class.to_string()],
        data_backed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engines_catalogue_ids_are_unique_kebab_case() {
        let all = catalogue();
        let mut ids: Vec<&str> = all.iter().map(|c| c.id.as_str()).collect();
        for id in &ids {
            assert!(
                ch_blueprint::catalogue::is_kebab(id),
                "{id} is not kebab case"
            );
        }
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "a catalogue id appears twice");
        assert!(all.iter().any(|c| c.data_backed && c.kind == Kind::Liquid));
        assert!(all.iter().any(|c| c.data_backed && c.kind == Kind::Alloy));
    }

    #[test]
    fn every_data_backed_id_reads_as_data_backed() {
        for c in catalogue().iter().filter(|c| c.data_backed) {
            assert!(
                data_backed(&c.id).is_some(),
                "{} has no data-backed shape",
                c.id
            );
        }
    }
}
