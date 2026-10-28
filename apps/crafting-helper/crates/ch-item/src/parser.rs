//! Sections and their classification: splits the text on `--------`, reads
//! the class/rarity/name block, decides what each later section is by its
//! content, and hands every line to exactly one consumer or to the report.

use crate::line::{self, Note};
use crate::model::{
    Form, Item, ItemClass, ItemLevel, Line, Marker, ModKind, ModLine, Modifier, ParseError,
    Partial, Property, Quality, Rarity, Report, Requirements, Sockets, Unparsed,
};
use crate::vocab::{self, Attribute, ItemFlag};

/// The input lines every index in the model refers to: a leading BOM
/// removed, split on LF with a trailing CR dropped, so CRLF and LF text are
/// numbered alike.
pub fn split_lines(text: &str) -> Vec<&str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect()
}

/// Parses copied item text, either form. Fails only when the text is not
/// item text at all (its first non-blank line is not `Item Class: …`);
/// anything else it does not understand is listed in `Item::report`.
pub fn parse(text: &str) -> Result<Item, ParseError> {
    let lines = split_lines(text);
    let first = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .ok_or(ParseError::NotItemText)?;
    if !lines[first].trim().starts_with(vocab::ITEM_CLASS) {
        return Err(ParseError::NotItemText);
    }

    let mut sections: Vec<Vec<(usize, &str)>> = vec![Vec::new()];
    for (index, raw) in lines.iter().enumerate().skip(first) {
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        if text == vocab::SEPARATOR {
            sections.push(Vec::new());
        } else if let Some(section) = sections.last_mut() {
            section.push((index, text));
        }
    }

    let advanced = sections
        .iter()
        .skip(1)
        .flatten()
        .any(|&(_, t)| line::is_modifier_header(t) || line::has_roll_range(t));

    let mut p = Parser {
        item: Item {
            form: Form::Undetermined,
            class: ItemClass::Other(String::new()),
            class_line: first,
            rarity: Rarity::Other(String::new()),
            rarity_line: None,
            name: None,
            base_type: None,
            name_lines: Vec::new(),
            item_level: None,
            quality: None,
            requirements: None,
            sockets: None,
            properties: Vec::new(),
            description: Vec::new(),
            modifiers: Vec::new(),
            corrupted: false,
            mirrored: false,
            unidentified: false,
            flag_lines: Vec::new(),
            report: Report::default(),
        },
        advanced,
        item_level_section: None,
    };
    p.name_block(&sections[0]);
    for (s, section) in sections.iter().enumerate().skip(1) {
        p.section(s, section);
    }
    p.finish();
    Ok(p.item)
}

struct Parser {
    item: Item,
    /// The text shows a modifier header or a roll range somewhere.
    advanced: bool,
    /// The section holding `Item Level:`; in the short form, plain lines
    /// after it are modifiers.
    item_level_section: Option<usize>,
}

impl Parser {
    fn unparsed(&mut self, line: usize, section: usize, text: &str, reason: &str) {
        self.item.report.unparsed.push(Unparsed {
            line,
            section,
            text: text.to_string(),
            reason: reason.to_string(),
        });
    }

    fn partial(&mut self, line: usize, section: usize, text: &str, note: Note) {
        self.item.report.partial.push(Partial {
            line,
            section,
            text: text.to_string(),
            part: note.part,
            reason: note.reason,
        });
    }

    /// Section 0: `Item Class:`, `Rarity:`, then the name lines (two for a
    /// rare or unique, one otherwise).
    fn name_block(&mut self, lines: &[(usize, &str)]) {
        let Some((&(class_line, class_text), rest)) = lines.split_first() else {
            return;
        };
        self.item.class_line = class_line;
        self.item.class = ItemClass::from_text(class_text[vocab::ITEM_CLASS.len()..].trim());
        for &(i, t) in rest {
            if self.item.rarity_line.is_none() {
                match t.strip_prefix(vocab::RARITY) {
                    Some(r) => {
                        self.item.rarity = Rarity::from_text(r.trim());
                        self.item.rarity_line = Some(i);
                    }
                    None => self.unparsed(i, 0, t, "expected the `Rarity:` line"),
                }
                continue;
            }
            let max = match self.item.rarity {
                Rarity::Rare | Rarity::Unique => 2,
                _ => 1,
            };
            if self.item.name_lines.len() < max {
                self.item.name_lines.push(Line {
                    index: i,
                    text: t.to_string(),
                });
            } else {
                self.unparsed(i, 0, t, "more name lines than the rarity has");
            }
        }
    }

    /// Decides what a section is from its content, not its position.
    fn section(&mut self, s: usize, lines: &[(usize, &str)]) {
        if lines.is_empty() {
            return;
        }
        if lines
            .iter()
            .any(|&(_, t)| line::is_header(t) || line::looks_like_modifier(t))
        {
            self.modifier_section(s, lines);
            return;
        }
        if lines.iter().all(|&(_, t)| t.ends_with('.')) {
            for &(i, t) in lines {
                self.item.description.push(Line {
                    index: i,
                    text: t.to_string(),
                });
            }
            return;
        }
        let single_words = lines.iter().all(|&(_, t)| !t.contains(char::is_whitespace));
        let after_item_level = self.item_level_section.is_some_and(|il| il < s);
        let all_key_value = lines.iter().all(|&(_, t)| split_key_value(t).is_some());
        if !self.advanced && after_item_level && !single_words && !all_key_value {
            self.modifier_section(s, lines);
            return;
        }
        self.key_value_section(s, lines);
    }

    /// Headers and the lines under them; a line with no header is its own
    /// modifier in the short form, and in the advanced form only when a
    /// marker says what it is.
    fn modifier_section(&mut self, s: usize, lines: &[(usize, &str)]) {
        let mut current: Option<Modifier> = None;
        for &(i, t) in lines {
            if line::is_header(t) {
                if let Some(m) = current.take() {
                    self.item.modifiers.push(m);
                }
                let h = line::parse_header(t);
                let mut m = Modifier::bare(h.kind);
                m.affix_name = h.affix_name;
                m.tier = h.tier;
                m.tags = h.tags;
                m.fractured = h.fractured;
                m.desecrated = h.desecrated;
                m.unrevealed = h.unrevealed;
                m.crafted = h.crafted;
                m.header_line = Some(i);
                for note in h.notes {
                    self.partial(i, s, t, note);
                }
                current = Some(m);
                continue;
            }

            let text = line::parse_mod_text(t);
            let names_unrevealed = vocab::names_unrevealed(&text.template);
            let mut notes = text.notes.clone();
            let modifier = match current.as_mut() {
                Some(m) => {
                    for &marker in &text.markers {
                        let agrees = match marker.kind() {
                            Some(kind) => kind == m.kind,
                            None => flag_of(m, marker),
                        };
                        if !agrees {
                            notes.push(Note {
                                part: format!("({})", marker.as_str()),
                                reason: "line marker disagrees with its header".to_string(),
                            });
                        }
                    }
                    m
                }
                None => {
                    let has_marker = !text.markers.is_empty();
                    if self.advanced && !has_marker && !names_unrevealed {
                        self.unparsed(i, s, t, "modifier line with no header");
                        continue;
                    }
                    let kind = text
                        .markers
                        .iter()
                        .find_map(|m| m.kind())
                        .unwrap_or(ModKind::Unknown);
                    self.item.modifiers.push(Modifier::bare(kind));
                    self.item.modifiers.last_mut().expect("just pushed")
                }
            };
            for &marker in &text.markers {
                set_flag(modifier, marker);
            }
            if names_unrevealed {
                modifier.unrevealed = true;
                modifier.desecrated = true;
            }
            modifier.lines.push(ModLine {
                index: i,
                text: t.to_string(),
                template: text.template,
                values: text.values,
                unscalable: text.unscalable,
                markers: text.markers,
            });
            for note in notes {
                self.partial(i, s, t, note);
            }
        }
        if let Some(m) = current {
            self.item.modifiers.push(m);
        }
    }

    /// Properties, item level, quality, requirements, sockets and flag lines.
    fn key_value_section(&mut self, s: usize, lines: &[(usize, &str)]) {
        let mut in_requirements_block = false;
        for &(i, t) in lines {
            if t == vocab::REQUIREMENTS_BLOCK {
                self.requirements().lines.push(i);
                in_requirements_block = true;
                continue;
            }
            if let Some(flag) = vocab::item_flag(t) {
                match flag {
                    ItemFlag::Corrupted => self.item.corrupted = true,
                    ItemFlag::Mirrored => self.item.mirrored = true,
                    ItemFlag::Unidentified => self.item.unidentified = true,
                }
                self.item.flag_lines.push(i);
                continue;
            }
            let Some((key, value)) = split_key_value(t) else {
                if t.contains(char::is_whitespace) {
                    self.unparsed(i, s, t, "line not understood");
                } else {
                    self.unparsed(i, s, t, "single-word line is not a known flag");
                }
                continue;
            };
            if in_requirements_block {
                self.requirement_entry(i, s, t, key, value);
                continue;
            }
            match key {
                vocab::KEY_ITEM_LEVEL => match value.parse::<u32>() {
                    Ok(level) if self.item.item_level.is_none() => {
                        self.item.item_level = Some(ItemLevel {
                            value: level,
                            line: i,
                        });
                        self.item_level_section = Some(s);
                    }
                    Ok(_) => self.unparsed(i, s, t, "second item level line"),
                    Err(_) => self.unparsed(i, s, t, "item level is not a number"),
                },
                vocab::KEY_QUALITY => {
                    let (value, augmented) = strip_augmented(value);
                    let number = value.trim_start_matches('+').trim_end_matches('%');
                    match number.parse::<u32>() {
                        Ok(q) if self.item.quality.is_none() => {
                            self.item.quality = Some(Quality {
                                value: q,
                                augmented,
                                line: i,
                            });
                        }
                        Ok(_) => self.unparsed(i, s, t, "second quality line"),
                        Err(_) => self.unparsed(i, s, t, "quality is not a percentage"),
                    }
                }
                vocab::KEY_REQUIRES => self.requires_line(i, s, t, value),
                vocab::KEY_SOCKETS if self.item.sockets.is_none() => {
                    let count = value
                        .split(|c: char| c.is_whitespace() || c == '-')
                        .filter(|p| !p.is_empty())
                        .count();
                    self.item.sockets = Some(Sockets {
                        text: value.to_string(),
                        count,
                        line: i,
                    });
                }
                vocab::KEY_SOCKETS => self.unparsed(i, s, t, "second sockets line"),
                _ => {
                    let (value, augmented) = strip_augmented(value);
                    self.item.properties.push(Property {
                        name: key.to_string(),
                        value: value.to_string(),
                        augmented,
                        line: i,
                    });
                }
            }
        }
    }

    fn requirements(&mut self) -> &mut Requirements {
        self.item
            .requirements
            .get_or_insert_with(Requirements::default)
    }

    /// `Requires: Level 45, 63 Str, 41 Int`.
    fn requires_line(&mut self, i: usize, s: usize, t: &str, value: &str) {
        self.requirements().lines.push(i);
        for entry in value.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let words: Vec<&str> = entry.split_whitespace().collect();
            let understood = match words.as_slice() {
                [vocab::KEY_LEVEL, n] => match n.parse::<u32>() {
                    Ok(n) => {
                        self.requirements().level = Some(n);
                        true
                    }
                    Err(_) => false,
                },
                [n, attr] => match (n.parse::<u32>(), vocab::attribute(attr)) {
                    (Ok(n), Some(a)) => {
                        self.set_attribute(a, n);
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if !understood {
                self.requirements().other.push(entry.to_string());
                self.partial(
                    i,
                    s,
                    t,
                    Note {
                        part: entry.to_string(),
                        reason: "requirement entry not understood".to_string(),
                    },
                );
            }
        }
    }

    /// One `Key: value` line of a `Requirements:` block.
    fn requirement_entry(&mut self, i: usize, s: usize, t: &str, key: &str, value: &str) {
        self.requirements().lines.push(i);
        let number = value
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u32>().ok());
        let understood = match (key, number) {
            (vocab::KEY_LEVEL, Some(n)) => {
                self.requirements().level = Some(n);
                true
            }
            (_, Some(n)) => match vocab::attribute(key) {
                Some(a) => {
                    self.set_attribute(a, n);
                    true
                }
                None => false,
            },
            _ => false,
        };
        if !understood {
            self.requirements().other.push(t.to_string());
            self.partial(
                i,
                s,
                t,
                Note {
                    part: t.to_string(),
                    reason: "requirement entry not understood".to_string(),
                },
            );
        }
    }

    fn set_attribute(&mut self, attribute: Attribute, value: u32) {
        let r = self.requirements();
        match attribute {
            Attribute::Strength => r.strength = Some(value),
            Attribute::Dexterity => r.dexterity = Some(value),
            Attribute::Intelligence => r.intelligence = Some(value),
        }
    }

    /// The form, and the name and base type now that the modifiers and the
    /// flags are known.
    fn finish(&mut self) {
        self.item.form = if self.advanced {
            Form::Advanced
        } else if self.item.modifiers.is_empty() {
            Form::Undetermined
        } else {
            Form::Short
        };
        let names: Vec<String> = self
            .item
            .name_lines
            .iter()
            .map(|l| l.text.clone())
            .collect();
        let rarity = self.item.rarity.clone();
        match (&rarity, names.as_slice()) {
            (_, []) => {}
            (Rarity::Normal, [base, ..]) => self.item.base_type = Some(base.clone()),
            (Rarity::Magic, [line, ..]) => {
                if self.item.unidentified {
                    self.item.base_type = Some(line.clone());
                } else if self.item.form == Form::Advanced {
                    self.item.base_type = Some(self.strip_magic_affixes(line));
                }
            }
            (Rarity::Rare | Rarity::Unique, [name, base]) => {
                self.item.name = Some(name.clone());
                self.item.base_type = Some(base.clone());
            }
            (Rarity::Rare | Rarity::Unique, [base]) => self.item.base_type = Some(base.clone()),
            _ => {}
        }
    }

    /// A magic item's name line is its base type between its prefix's and
    /// suffix's affix names, which the advanced form's headers give.
    fn strip_magic_affixes(&mut self, line: &str) -> String {
        let affixes = |kind: ModKind| -> Vec<String> {
            self.item
                .modifiers
                .iter()
                .filter(|m| m.kind == kind)
                .filter_map(|m| m.affix_name.clone())
                .collect()
        };
        let prefixes = affixes(ModKind::Prefix);
        let suffixes = affixes(ModKind::Suffix);
        let mut base = line;
        let mut missing = Vec::new();
        match prefixes.iter().find_map(|p| {
            base.strip_prefix(p.as_str())
                .and_then(|b| b.strip_prefix(' '))
        }) {
            Some(rest) => base = rest,
            None if !prefixes.is_empty() => missing.push(prefixes.join(", ")),
            None => {}
        }
        match suffixes.iter().find_map(|x| {
            base.strip_suffix(x.as_str())
                .and_then(|b| b.strip_suffix(' '))
        }) {
            Some(rest) => base = rest,
            None if !suffixes.is_empty() => missing.push(suffixes.join(", ")),
            None => {}
        }
        if let Some(name_line) = self.item.name_lines.first().cloned() {
            for part in missing {
                self.partial(
                    name_line.index,
                    0,
                    &name_line.text,
                    Note {
                        part,
                        reason: "affix name not found in the magic name line".to_string(),
                    },
                );
            }
        }
        base.trim().to_string()
    }
}

/// Whether the modifier already has the flag a flag marker states.
fn flag_of(m: &Modifier, marker: Marker) -> bool {
    match marker {
        Marker::Fractured => m.fractured,
        Marker::Desecrated => m.desecrated,
        Marker::Crafted => m.crafted,
        Marker::Implicit | Marker::Rune | Marker::Enchant => true,
    }
}

fn set_flag(m: &mut Modifier, marker: Marker) {
    match marker {
        Marker::Fractured => m.fractured = true,
        Marker::Desecrated => m.desecrated = true,
        Marker::Crafted => m.crafted = true,
        Marker::Implicit | Marker::Rune | Marker::Enchant => {}
    }
}

/// `Key: value`, where the key starts with a capital and holds only letters,
/// spaces, apostrophes, hyphens and parentheses.
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(": ")?;
    let value = value.trim();
    let first = key.chars().next()?;
    let key_ok = first.is_uppercase()
        && key
            .chars()
            .all(|c| c.is_alphabetic() || matches!(c, ' ' | '\'' | '-' | '(' | ')'));
    (key_ok && !value.is_empty()).then_some((key, value))
}

fn strip_augmented(value: &str) -> (&str, bool) {
    match value.strip_suffix(vocab::AUGMENTED) {
        Some(v) => (v.trim_end(), true),
        None => (value, false),
    }
}
