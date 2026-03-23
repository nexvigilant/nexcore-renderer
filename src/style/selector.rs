//! CSS selectors and specificity calculation.
//!
//! Implements W3C selector matching with specificity-ordered cascade.

use crate::dom::{Arena, NodeId};
use std::collections::BTreeMap;

// ── Specificity (T2-P: cross-domain orderable quantity) ────────────

/// CSS specificity as (inline, id, class, type) counts.
///
/// Per W3C: inline styles > #id > .class/:pseudo > element.
/// Implements `Ord` for cascade ordering (Codex Commandment V: COMPARE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Specificity(pub u16, pub u16, pub u16, pub u16);

impl Specificity {
    /// Specificity for inline styles (highest non-!important).
    pub const INLINE: Self = Self(1, 0, 0, 0);
}

impl Ord for Specificity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .cmp(&other.0)
            .then(self.1.cmp(&other.1))
            .then(self.2.cmp(&other.2))
            .then(self.3.cmp(&other.3))
    }
}

impl PartialOrd for Specificity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ── Selector types ─────────────────────────────────────────────────

/// A single CSS selector component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    /// Match by tag name: `div`, `h1`, etc.
    Tag(String),
    /// Match by class: `.foo`
    Class(String),
    /// Match by id: `#bar`
    Id(String),
    /// Match everything: `*`
    Universal,
}

/// A compound selector: sequence of simple selectors all applying to one element.
/// e.g. `div.active#main` → [Tag("div"), Class("active"), Id("main")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundSelector {
    pub parts: Vec<SimpleSelector>,
}

/// A full CSS selector (currently compound only; combinators added later).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Simple compound selector (no combinators).
    Simple(CompoundSelector),
}

// ── Selector parsing ───────────────────────────────────────────────

impl Selector {
    /// Parse a selector string like `div.class#id`.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        let compound = CompoundSelector::parse(input)?;
        Some(Self::Simple(compound))
    }

    /// Compute specificity of this selector.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        match self {
            Self::Simple(compound) => compound.specificity(),
        }
    }

    /// Test if this selector matches a DOM node in the arena.
    #[must_use]
    pub fn matches_arena(&self, id: NodeId, arena: &Arena) -> bool {
        match self {
            Self::Simple(compound) => compound.matches_arena(id, arena),
        }
    }
}

impl CompoundSelector {
    /// Parse compound selector from string.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let mut parts = Vec::new();
        let mut chars = input.chars().peekable();

        while chars.peek().is_some() {
            match chars.peek() {
                Some('#') => {
                    chars.next(); // consume '#'
                    let name = Self::consume_ident(&mut chars);
                    if !name.is_empty() {
                        parts.push(SimpleSelector::Id(name));
                    }
                }
                Some('.') => {
                    chars.next(); // consume '.'
                    let name = Self::consume_ident(&mut chars);
                    if !name.is_empty() {
                        parts.push(SimpleSelector::Class(name));
                    }
                }
                Some('*') => {
                    chars.next();
                    parts.push(SimpleSelector::Universal);
                }
                Some(c) if c.is_alphanumeric() || *c == '-' || *c == '_' => {
                    let name = Self::consume_ident(&mut chars);
                    if !name.is_empty() {
                        parts.push(SimpleSelector::Tag(name));
                    }
                }
                _ => {
                    // Skip unknown characters
                    chars.next();
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(Self { parts })
        }
    }

    /// Consume an identifier (tag name, class name, etc.).
    fn consume_ident(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        name
    }

    /// Compute specificity: count #ids, .classes, tags.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        let mut ids: u16 = 0;
        let mut classes: u16 = 0;
        let mut tags: u16 = 0;

        for part in &self.parts {
            match part {
                SimpleSelector::Id(_) => ids += 1,
                SimpleSelector::Class(_) => classes += 1,
                SimpleSelector::Tag(_) => tags += 1,
                SimpleSelector::Universal => {} // zero specificity
            }
        }

        Specificity(0, ids, classes, tags)
    }

    /// Test if all parts match a node in the arena.
    #[must_use]
    pub fn matches_arena(&self, id: NodeId, arena: &Arena) -> bool {
        self.parts.iter().all(|part| part.matches_arena(id, arena))
    }
}

impl SimpleSelector {
    /// Test if this simple selector matches a node in the arena.
    #[must_use]
    pub fn matches_arena(&self, id: NodeId, arena: &Arena) -> bool {
        match self {
            Self::Universal => arena.tag(id).is_some(),
            Self::Tag(sel_tag) => arena.tag(id).is_some_and(|t| t == sel_tag),
            Self::Class(sel_class) => arena
                .attrs(id)
                .is_some_and(|attrs| has_class(attrs, sel_class)),
            Self::Id(sel_id) => arena
                .attrs(id)
                .is_some_and(|attrs| attrs.get("id").is_some_and(|id| id == sel_id)),
        }
    }
}

/// Check if an element's class attribute contains the given class name.
fn has_class(attrs: &BTreeMap<String, String>, class_name: &str) -> bool {
    attrs
        .get("class")
        .is_some_and(|classes| classes.split_whitespace().any(|c| c == class_name))
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeKind;

    #[test]
    fn specificity_ordering() {
        let inline = Specificity::INLINE;
        let id = Specificity(0, 1, 0, 0);
        let class = Specificity(0, 0, 1, 0);
        let tag = Specificity(0, 0, 0, 1);
        let universal = Specificity(0, 0, 0, 0);

        assert!(inline > id);
        assert!(id > class);
        assert!(class > tag);
        assert!(tag > universal);
    }

    #[test]
    fn specificity_equal() {
        assert_eq!(Specificity(0, 1, 0, 0), Specificity(0, 1, 0, 0));
    }

    #[test]
    fn parse_tag_selector() {
        let sel = Selector::parse("div").unwrap();
        assert_eq!(sel.specificity(), Specificity(0, 0, 0, 1));
    }

    #[test]
    fn parse_class_selector() {
        let sel = Selector::parse(".active").unwrap();
        assert_eq!(sel.specificity(), Specificity(0, 0, 1, 0));
    }

    #[test]
    fn parse_id_selector() {
        let sel = Selector::parse("#main").unwrap();
        assert_eq!(sel.specificity(), Specificity(0, 1, 0, 0));
    }

    #[test]
    fn parse_compound_selector() {
        let sel = Selector::parse("div.active#main").unwrap();
        assert_eq!(sel.specificity(), Specificity(0, 1, 1, 1));
    }

    #[test]
    fn selector_matches_tag_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let div = arena.alloc(
            NodeKind::Element {
                tag: "div".to_string(),
                attrs: BTreeMap::new(),
            },
            Some(root),
        );
        let sel = Selector::parse("div").unwrap();
        assert!(sel.matches_arena(div, &arena));
        let sel2 = Selector::parse("span").unwrap();
        assert!(!sel2.matches_arena(div, &arena));
    }

    #[test]
    fn selector_matches_class_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let mut attrs = BTreeMap::new();
        attrs.insert("class".to_string(), "foo bar".to_string());
        let div = arena.alloc(
            NodeKind::Element {
                tag: "div".to_string(),
                attrs,
            },
            Some(root),
        );
        let sel = Selector::parse(".foo").unwrap();
        assert!(sel.matches_arena(div, &arena));
        let sel2 = Selector::parse(".baz").unwrap();
        assert!(!sel2.matches_arena(div, &arena));
    }

    #[test]
    fn selector_matches_id_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let mut attrs = BTreeMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        let div = arena.alloc(
            NodeKind::Element {
                tag: "div".to_string(),
                attrs,
            },
            Some(root),
        );
        let sel = Selector::parse("#main").unwrap();
        assert!(sel.matches_arena(div, &arena));
    }

    #[test]
    fn universal_matches_any_element_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let span = arena.alloc(
            NodeKind::Element {
                tag: "span".to_string(),
                attrs: BTreeMap::new(),
            },
            Some(root),
        );
        let sel = Selector::parse("*").unwrap();
        assert!(sel.matches_arena(span, &arena));
    }

    #[test]
    fn universal_does_not_match_text_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let text = arena.alloc(NodeKind::Text("hello".to_string()), Some(root));
        let sel = Selector::parse("*").unwrap();
        assert!(!sel.matches_arena(text, &arena));
    }

    #[test]
    fn empty_selector_returns_none() {
        assert!(Selector::parse("").is_none());
        assert!(Selector::parse("   ").is_none());
    }
}
