//! The item's prefix and suffix limits (`item_affix_limits`): the table's
//! rarity and class limits, plus every "Prefix/Suffix Modifier allowed"
//! line on the item, implicit or explicit, with its sign.
//!
//! How the game writes a base's limit implicit in a `Ctrl+Alt+C` copy is not
//! measured (no fixture has one). The items here are constructed: the
//! implicit lines come from the embedded data's base rows, written in the
//! existing `{ Implicit Modifier }` form with each `#` replaced by the low
//! end of its range.

use ch_data::{class_data, item_affix_limits, AffixLimits};
use ch_item::{Item, ItemClass, Rarity};

fn limits(prefixes: usize, suffixes: usize) -> AffixLimits {
    AffixLimits { prefixes, suffixes }
}

/// A constructed item: class, rarity, base type, item level and the given
/// blocks (implicits, then explicits), nothing else.
fn item(class: &str, rarity: &str, base: &str, blocks: &[&str]) -> Item {
    let name = if rarity == "Rare" { "Test Name\n" } else { "" };
    let mut text =
        format!("Item Class: {class}\nRarity: {rarity}\n{name}{base}\n--------\nItem Level: 82\n");
    for block in blocks {
        text.push_str("--------\n");
        text.push_str(block);
    }
    let item = ch_item::parse(&text).unwrap_or_else(|e| panic!("parse: {e}\n{text}"));
    // A magic item's name line carries its affixes, so its base type is not
    // read from the text; the limits need only the class and the lines.
    if rarity != "Magic" {
        assert_eq!(item.base_type.as_deref(), Some(base), "{text}");
    }
    item
}

/// The implicit block of `base` in `class`, from the embedded data: one
/// `{ Implicit Modifier }` header per implicit, each `#` written as the low
/// end of its value range.
fn data_implicits(class: &str, base: &str) -> String {
    let data = class_data(&ItemClass::from_text(class)).unwrap_or_else(|| panic!("no {class}"));
    let row = data
        .bases
        .iter()
        .find(|b| b.name == base)
        .unwrap_or_else(|| panic!("no base {base} in {class}"));
    assert!(
        !row.implicits.is_empty(),
        "{base} has no implicit in the data"
    );
    let mut block = String::new();
    for implicit in &row.implicits {
        block.push_str("{ Implicit Modifier }\n");
        assert_eq!(implicit.lines.len(), implicit.values.len(), "{base}");
        for (line, value) in implicit.lines.iter().zip(&implicit.values) {
            block.push_str(&line.replacen('#', &format!("{}", value[0] as i64), 1));
            block.push('\n');
        }
    }
    block
}

/// A base from the data, rare (or `rarity`), carrying its data implicits.
fn data_base(class: &str, rarity: &str, base: &str) -> Item {
    let implicits = data_implicits(class, base);
    item(class, rarity, base, &[&implicits])
}

#[test]
fn rare_limits_are_three_and_three() {
    let ring = item("Rings", "Rare", "Amethyst Ring", &[]);
    assert_eq!(item_affix_limits(&ring, &Rarity::Rare), Some(limits(3, 3)));
    let jewel = item("Jewels", "Rare", "Sapphire", &[]);
    assert_eq!(item_affix_limits(&jewel, &Rarity::Rare), Some(limits(2, 2)));
    let magic = item("Rings", "Magic", "Amethyst Ring", &[]);
    assert_eq!(
        item_affix_limits(&magic, &Rarity::Magic),
        Some(limits(1, 1))
    );
}

#[test]
fn contempt_adds_a_slot_on_the_other_side() {
    // Contempt as a suffix: "+1 Prefix Modifier allowed".
    let jewel = item(
        "Jewels",
        "Rare",
        "Sapphire",
        &["{ Suffix Modifier }\n+1 Prefix Modifier allowed\n"],
    );
    assert_eq!(item_affix_limits(&jewel, &Rarity::Rare), Some(limits(3, 2)));
}

#[test]
fn an_implicit_raises_or_lowers_a_side() {
    // Dusk: one more prefix, one fewer suffix.
    let dusk = data_base("Rings", "Rare", "Dusk Ring");
    assert_eq!(item_affix_limits(&dusk, &Rarity::Rare), Some(limits(4, 2)));
    // Gloam: the reverse.
    let gloam = data_base("Rings", "Rare", "Gloam Ring");
    assert_eq!(item_affix_limits(&gloam, &Rarity::Rare), Some(limits(2, 4)));
    // Distorted: one fewer suffix only.
    let distorted = data_base("Amulets", "Rare", "Distorted Amulet");
    assert_eq!(
        item_affix_limits(&distorted, &Rarity::Rare),
        Some(limits(3, 2))
    );

    // A side never goes below 0: a normal Distorted Amulet (0 and 0) and a
    // magic one carrying two lowering lines (1 suffix, minus 2).
    let normal = data_base("Amulets", "Normal", "Distorted Amulet");
    assert_eq!(
        item_affix_limits(&normal, &Rarity::Normal),
        Some(limits(0, 0))
    );
    let twice = item(
        "Amulets",
        "Magic",
        "Distorted Amulet",
        &["{ Implicit Modifier }\n-1 Suffix Modifier allowed\n-1 Suffix Modifier allowed\n"],
    );
    assert_eq!(
        item_affix_limits(&twice, &Rarity::Magic),
        Some(limits(1, 0))
    );
}
