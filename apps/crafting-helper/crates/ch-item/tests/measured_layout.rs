//! Baseline: the item-text layout measured from real copied waystones
//! (waystone roller Live 1, 2026-10-28). The waystone roller's fixtures carry
//! invented mods in that measured layout, with an independent oracle in
//! `items.json`; every one must parse with nothing left over and agree with
//! the oracle.

mod common;

use ch_item::{parse, Form, ItemClass, ModKind, Rarity};
use common::{read, txt_files, waystone_dir};

/// One `items.json` entry: what the item is, written independently of its
/// text.
struct Oracle {
    file: String,
    rarity: String,
    tier: u32,
    corrupted: bool,
    mods: usize,
}

fn oracle() -> Vec<Oracle> {
    let json = Json::parse(&read(&waystone_dir().join("items.json")));
    let items = json.get("items").and_then(Json::as_array).expect("items");
    items
        .iter()
        .map(|i| Oracle {
            file: i.get("file").and_then(Json::as_str).expect("file").into(),
            rarity: i
                .get("rarity")
                .and_then(Json::as_str)
                .expect("rarity")
                .into(),
            tier: i.get("tier").and_then(Json::as_number).expect("tier") as u32,
            corrupted: i
                .get("corrupted")
                .and_then(Json::as_bool)
                .expect("corrupted"),
            mods: i.get("mods").and_then(Json::as_array).expect("mods").len(),
        })
        .collect()
}

#[test]
fn every_waystone_text_matches_the_oracle() {
    let entries = oracle();
    let files = txt_files(&waystone_dir());
    assert!(!files.is_empty(), "no waystone fixtures found");
    assert_eq!(files.len(), entries.len(), "every text has an oracle entry");
    for e in &entries {
        let item = parse(&read(&waystone_dir().join(&e.file))).expect("item text");
        assert!(
            item.report.unparsed.is_empty() && item.report.partial.is_empty(),
            "{}: {:?}",
            e.file,
            item.report
        );
        assert_eq!(
            item.class,
            ItemClass::Other("Waystones".into()),
            "{}",
            e.file
        );
        let rarity = match e.rarity.as_str() {
            "normal" => Rarity::Normal,
            "magic" => Rarity::Magic,
            "rare" => Rarity::Rare,
            other => panic!("oracle rarity {other}"),
        };
        assert_eq!(item.rarity, rarity, "{}", e.file);
        assert_eq!(item.corrupted, e.corrupted, "{}", e.file);
        let base = item.base_type.as_deref().expect("base type");
        assert!(
            base.contains(&format!("(Tier {})", e.tier)),
            "{}: base {base:?}",
            e.file
        );
        let explicit = item
            .modifiers
            .iter()
            .filter(|m| matches!(m.kind, ModKind::Prefix | ModKind::Suffix))
            .count();
        assert_eq!(explicit, e.mods, "{}", e.file);
        assert_eq!(
            item.modifiers.len(),
            e.mods,
            "{}: only explicit mods",
            e.file
        );
        for m in &item.modifiers {
            assert!(m.tier.is_some(), "{}: {m:?}", e.file);
            assert!(m.affix_name.is_some(), "{}: {m:?}", e.file);
        }
        let form = if e.mods == 0 {
            Form::Undetermined
        } else {
            Form::Advanced
        };
        assert_eq!(item.form, form, "{}", e.file);
    }
}

#[test]
fn magic_waystone_base_type_has_its_affix_names_stripped() {
    for (file, base) in [
        ("magic-t12.txt", "Waystone (Tier 12)"),
        ("magic-t3.txt", "Waystone (Tier 3)"),
        ("magic-t9-corrupted.txt", "Waystone (Tier 9)"),
    ] {
        let item = parse(&read(&waystone_dir().join(file))).expect("item text");
        assert_eq!(item.base_type.as_deref(), Some(base), "{file}");
        assert_eq!(item.name, None, "{file}");
    }
}

#[test]
fn measured_glimmer_line_parses() {
    let item = parse(&read(&waystone_dir().join("rare-t15-corrupted.txt"))).expect("item text");
    let m = &item.modifiers[0];
    assert_eq!(m.kind, ModKind::Prefix);
    assert_eq!(m.affix_name.as_deref(), Some("Glinting"));
    assert_eq!(m.tier, Some(1));
    let line = &m.lines[0];
    assert_eq!(
        line.text,
        "Monsters deal 12(10-14)% of Damage as Extra Glimmer"
    );
    assert_eq!(line.template, "Monsters deal #% of Damage as Extra Glimmer");
    assert_eq!(line.values.len(), 1);
    assert_eq!(line.values[0].value, 12.0);
    let range = line.values[0].range.expect("roll range");
    assert_eq!((range.min, range.max), (10.0, 14.0));
}

// A minimal JSON reader: the crate takes no dependency, not even for tests,
// and `items.json` needs only objects, arrays, strings, numbers and booleans.

enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn parse(text: &str) -> Json {
        let mut p = JsonParser {
            chars: text.chars().collect(),
            at: 0,
        };
        let v = p.value();
        p.skip_ws();
        assert_eq!(p.at, p.chars.len(), "trailing JSON");
        v
    }

    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            Json::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

struct JsonParser {
    chars: Vec<char>,
    at: usize,
}

impl JsonParser {
    fn skip_ws(&mut self) {
        while self.at < self.chars.len() && self.chars[self.at].is_whitespace() {
            self.at += 1;
        }
    }

    fn peek(&self) -> char {
        *self.chars.get(self.at).expect("unexpected end of JSON")
    }

    fn expect(&mut self, c: char) {
        self.skip_ws();
        assert_eq!(self.peek(), c, "JSON at {}", self.at);
        self.at += 1;
    }

    fn literal(&mut self, word: &str) {
        for c in word.chars() {
            assert_eq!(self.peek(), c, "JSON literal {word}");
            self.at += 1;
        }
    }

    fn value(&mut self) -> Json {
        self.skip_ws();
        match self.peek() {
            '{' => {
                self.at += 1;
                let mut fields = Vec::new();
                self.skip_ws();
                if self.peek() == '}' {
                    self.at += 1;
                    return Json::Object(fields);
                }
                loop {
                    self.skip_ws();
                    let key = self.string();
                    self.expect(':');
                    fields.push((key, self.value()));
                    self.skip_ws();
                    let c = self.peek();
                    self.at += 1;
                    match c {
                        ',' => continue,
                        '}' => return Json::Object(fields),
                        other => panic!("JSON object: {other:?}"),
                    }
                }
            }
            '[' => {
                self.at += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == ']' {
                    self.at += 1;
                    return Json::Array(items);
                }
                loop {
                    items.push(self.value());
                    self.skip_ws();
                    let c = self.peek();
                    self.at += 1;
                    match c {
                        ',' => continue,
                        ']' => return Json::Array(items),
                        other => panic!("JSON array: {other:?}"),
                    }
                }
            }
            '"' => Json::String(self.string()),
            't' => {
                self.literal("true");
                Json::Bool(true)
            }
            'f' => {
                self.literal("false");
                Json::Bool(false)
            }
            'n' => {
                self.literal("null");
                Json::Null
            }
            _ => {
                let start = self.at;
                while self.at < self.chars.len()
                    && matches!(self.chars[self.at], '-' | '+' | '.' | 'e' | 'E' | '0'..='9')
                {
                    self.at += 1;
                }
                let s: String = self.chars[start..self.at].iter().collect();
                Json::Number(s.parse().unwrap_or_else(|_| panic!("JSON number {s:?}")))
            }
        }
    }

    fn string(&mut self) -> String {
        assert_eq!(self.peek(), '"', "JSON string at {}", self.at);
        self.at += 1;
        let mut s = String::new();
        loop {
            let c = self.peek();
            self.at += 1;
            match c {
                '"' => return s,
                '\\' => {
                    let e = self.peek();
                    self.at += 1;
                    match e {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        'u' => {
                            let hex: String = self.chars[self.at..self.at + 4].iter().collect();
                            self.at += 4;
                            let code = u32::from_str_radix(&hex, 16).expect("JSON \\u escape");
                            s.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        other => s.push(other),
                    }
                }
                other => s.push(other),
            }
        }
    }
}
