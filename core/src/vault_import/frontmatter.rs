//! YAML frontmatter: splitting it off a note and parsing it (design D5).
//!
//! `serde-saphyr` has no generic value type, so frontmatter is read into
//! [`FrontValue`] through its own `Deserialize`. Keys keep their file order.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use std::fmt;

/// One YAML value, as far as frontmatter needs it. YAML dates have no type of
/// their own: they arrive as [`FrontValue::Str`] and are recognised later.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontValue {
    Null,
    Bool(bool),
    /// Wide enough for every integer YAML hands over (`u64` included).
    Int(i128),
    Float(f64),
    Str(String),
    List(Vec<FrontValue>),
    Map(Vec<(String, FrontValue)>),
}

/// Parsed frontmatter: top-level keys in file order.
pub type Frontmatter = Vec<(String, FrontValue)>;

/// Split `text` into its raw frontmatter block (without the `---` fences) and
/// the body that follows the closing fence. A note without a complete block —
/// no opening `---` on the first line, or no closing one — is all body.
pub fn split(text: &str) -> (Option<&str>, &str) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = strip_fence_line(text, "---") else {
        return (None, text);
    };

    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']).trim_end();
        if trimmed == "---" || trimmed == "..." {
            let yaml = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (Some(yaml), body);
        }
        offset += line.len();
    }
    (None, text)
}

/// The text after `fence` when the first line is exactly that fence.
fn strip_fence_line<'a>(text: &'a str, fence: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(fence)?;
    let rest = rest.trim_start_matches([' ', '\t']);
    rest.strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
}

/// Parse a raw frontmatter block. An empty block is an empty map; anything that
/// is not a YAML mapping at the top level is an error, as is invalid YAML.
pub fn parse(raw: &str) -> Result<Frontmatter, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    // YAML 1.2 booleans, as Obsidian's own parser reads them: `yes` stays text.
    let options = serde_saphyr::options!(strict_booleans: true);
    let value: FrontValue =
        serde_saphyr::from_str_with_options(raw, options).map_err(|e| e.to_string())?;
    match value {
        FrontValue::Map(entries) => Ok(entries),
        FrontValue::Null => Ok(Vec::new()),
        _ => Err("frontmatter is not a mapping".to_string()),
    }
}

impl FrontValue {
    /// The value as YAML text, for values kept as one string (nested maps).
    pub fn to_yaml(&self) -> String {
        serde_saphyr::to_string(self)
            .map(|s| s.trim_end().to_string())
            .unwrap_or_default()
    }

    /// A scalar's text; `None` for null, lists and maps.
    pub fn scalar_text(&self) -> Option<String> {
        match self {
            FrontValue::Bool(b) => Some(b.to_string()),
            FrontValue::Int(i) => Some(i.to_string()),
            FrontValue::Float(f) => Some(f.to_string()),
            FrontValue::Str(s) => Some(s.clone()),
            FrontValue::Null | FrontValue::List(_) | FrontValue::Map(_) => None,
        }
    }

    /// `[[x]]` written without quotes is YAML for a list holding a list holding
    /// `x`. Read back as the wikilink the author meant.
    pub fn unquoted_wikilink(&self) -> Option<String> {
        match self {
            FrontValue::List(outer) => match outer.as_slice() {
                [FrontValue::List(inner)] => match inner.as_slice() {
                    [FrontValue::Str(s)] => Some(format!("[[{s}]]")),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for FrontValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(FrontValueVisitor)
    }
}

struct FrontValueVisitor;

impl<'de> Visitor<'de> for FrontValueVisitor {
    type Value = FrontValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any YAML value")
    }

    fn visit_unit<E>(self) -> Result<FrontValue, E> {
        Ok(FrontValue::Null)
    }

    fn visit_none<E>(self) -> Result<FrontValue, E> {
        Ok(FrontValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<FrontValue, D::Error> {
        FrontValue::deserialize(d)
    }

    fn visit_bool<E>(self, v: bool) -> Result<FrontValue, E> {
        Ok(FrontValue::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<FrontValue, E> {
        Ok(FrontValue::Int(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<FrontValue, E> {
        Ok(FrontValue::Int(v.into()))
    }

    fn visit_i128<E>(self, v: i128) -> Result<FrontValue, E> {
        Ok(FrontValue::Int(v))
    }

    fn visit_u128<E>(self, v: u128) -> Result<FrontValue, E> {
        // Beyond i128 a number is kept as its digits rather than rounded.
        Ok(i128::try_from(v).map_or_else(|_| FrontValue::Str(v.to_string()), FrontValue::Int))
    }

    fn visit_f64<E>(self, v: f64) -> Result<FrontValue, E> {
        Ok(FrontValue::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<FrontValue, E> {
        Ok(FrontValue::Str(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<FrontValue, E> {
        Ok(FrontValue::Str(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<FrontValue, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element::<FrontValue>()? {
            items.push(item);
        }
        Ok(FrontValue::List(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<FrontValue, A::Error> {
        let mut entries: Vec<(String, FrontValue)> = Vec::new();
        while let Some((key, value)) = map.next_entry::<FrontValue, FrontValue>()? {
            // A non-string key (`2025: x`) is still a key; it is named by its text.
            let key = key
                .scalar_text()
                .ok_or_else(|| de::Error::custom("unsupported mapping key"))?;
            entries.push((key, value));
        }
        Ok(FrontValue::Map(entries))
    }
}

impl Serialize for FrontValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            FrontValue::Null => serializer.serialize_unit(),
            FrontValue::Bool(b) => serializer.serialize_bool(*b),
            FrontValue::Int(i) => match (i64::try_from(*i), u64::try_from(*i)) {
                (Ok(i), _) => serializer.serialize_i64(i),
                (_, Ok(u)) => serializer.serialize_u64(u),
                _ => serializer.serialize_i128(*i),
            },
            FrontValue::Float(f) => serializer.serialize_f64(*f),
            FrontValue::Str(s) => serializer.serialize_str(s),
            FrontValue::List(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            FrontValue::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (k, v) in entries {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(front: &'a Frontmatter, key: &str) -> &'a FrontValue {
        &front.iter().find(|(k, _)| k == key).unwrap().1
    }

    #[test]
    fn splits_block_from_body() {
        let (yaml, body) = split("---\ntype: Note\n---\n# Devices\n\nbody\n");
        assert_eq!(yaml, Some("type: Note\n"));
        assert_eq!(body, "# Devices\n\nbody\n");
    }

    #[test]
    fn crlf_and_bom_are_accepted() {
        let (yaml, body) = split("\u{feff}---\r\na: 1\r\n---\r\nbody");
        assert_eq!(yaml, Some("a: 1\r\n"));
        assert_eq!(body, "body");
    }

    #[test]
    fn no_frontmatter_is_all_body() {
        assert_eq!(split("# Title\n---\n"), (None, "# Title\n---\n"));
        // An opening fence that is never closed is not frontmatter.
        assert_eq!(split("---\na: 1\nbody"), (None, "---\na: 1\nbody"));
    }

    #[test]
    fn empty_block() {
        let (yaml, body) = split("---\n---\nbody");
        assert_eq!(yaml, Some(""));
        assert_eq!(body, "body");
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn scalars() {
        let front = parse("s: Active\nb: true\ny: yes\ni: 137\nn: -4\nf: 1.5\nz: ~\n").unwrap();
        assert_eq!(get(&front, "s"), &FrontValue::Str("Active".into()));
        assert_eq!(get(&front, "b"), &FrontValue::Bool(true));
        // YAML 1.2: `yes` is a string.
        assert_eq!(get(&front, "y"), &FrontValue::Str("yes".into()));
        assert_eq!(get(&front, "i"), &FrontValue::Int(137));
        assert_eq!(get(&front, "n"), &FrontValue::Int(-4));
        assert_eq!(get(&front, "f"), &FrontValue::Float(1.5));
        assert_eq!(get(&front, "z"), &FrontValue::Null);
        let keys: Vec<&str> = front.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            ["s", "b", "y", "i", "n", "f", "z"],
            "file order is kept"
        );
    }

    #[test]
    fn lists_and_nested_maps() {
        let front = parse("tags:\n  - rust\n  - \"#lang\"\nmeta:\n  a: 1\n  b: [x, y]\n").unwrap();
        assert_eq!(
            get(&front, "tags"),
            &FrontValue::List(vec![
                FrontValue::Str("rust".into()),
                FrontValue::Str("#lang".into())
            ])
        );
        let FrontValue::Map(meta) = get(&front, "meta") else {
            panic!("not a map")
        };
        assert_eq!(meta[0], ("a".to_string(), FrontValue::Int(1)));
        let yaml = get(&front, "meta").to_yaml();
        assert!(yaml.contains("a: 1"), "{yaml}");
    }

    #[test]
    fn integers_beyond_i64_stay_integers() {
        let front = parse("big: 18446744073709551615\nmid: 9223372036854775808\n").unwrap();
        assert_eq!(get(&front, "big"), &FrontValue::Int(u64::MAX.into()));
        assert_eq!(
            get(&front, "mid"),
            &FrontValue::Int(i128::from(i64::MAX) + 1)
        );
        assert_eq!(get(&front, "big").to_yaml(), "18446744073709551615");
    }

    #[test]
    fn dates_stay_text() {
        let front = parse("date: 2025-04-27\nat: 2025-04-27T10:30:00Z\n").unwrap();
        assert_eq!(get(&front, "date"), &FrontValue::Str("2025-04-27".into()));
        assert_eq!(
            get(&front, "at"),
            &FrontValue::Str("2025-04-27T10:30:00Z".into())
        );
    }

    #[test]
    fn quoted_and_unquoted_wikilinks() {
        let front = parse("belongs_to: \"[[3d-printing]]\"\nrelated_to: [[devices]]\n").unwrap();
        assert_eq!(
            get(&front, "belongs_to"),
            &FrontValue::Str("[[3d-printing]]".into())
        );
        assert_eq!(
            get(&front, "related_to").unquoted_wikilink().as_deref(),
            Some("[[devices]]")
        );
    }

    #[test]
    fn broken_yaml_is_an_error() {
        assert!(parse("status: [unclosed\n").is_err());
        assert!(parse("- just\n- a list\n").is_err());
    }
}
