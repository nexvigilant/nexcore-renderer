//! CSS stylesheet parsing.
//!
//! Extracts `<style>` blocks from DOM and parses into `CssRule` values.

use super::cascade::{CssRule, StyleOrigin};
use super::selector::Selector;
use crate::dom::{Arena, NodeId, NodeKind};

/// Extract all `<style>` elements from an arena-based DOM and parse their CSS.
#[must_use]
pub fn extract_stylesheets_arena(arena: &Arena) -> Vec<CssRule> {
    let mut rules = Vec::new();
    collect_style_elements_arena(arena, arena.root(), &mut rules);
    rules
}

/// Iteratively find `<style>` elements in the arena and parse their text content.
fn collect_style_elements_arena(arena: &Arena, id: NodeId, rules: &mut Vec<CssRule>) {
    if let Some(NodeKind::Element { tag, .. }) = arena.kind(id)
        && tag == "style"
    {
        let css_text: String = arena
            .children(id)
            .iter()
            .filter_map(|&child_id| arena.text(child_id))
            .collect();
        let parsed = parse_stylesheet(&css_text, StyleOrigin::Author);
        rules.extend(parsed);
        return;
    }
    for &child in arena.children(id) {
        collect_style_elements_arena(arena, child, rules);
    }
}

/// Parse a CSS stylesheet string into rules.
#[must_use]
pub fn parse_stylesheet(css: &str, origin: StyleOrigin) -> Vec<CssRule> {
    let mut rules = Vec::new();
    let mut chars = css.chars().peekable();

    while chars.peek().is_some() {
        skip_whitespace_and_comments(&mut chars);
        if chars.peek().is_none() {
            break;
        }
        let selector_str = consume_until(&mut chars, '{');
        let selector_str = selector_str.trim();
        if selector_str.is_empty() {
            if chars.peek() == Some(&'{') {
                chars.next();
            }
            continue;
        }
        if chars.next() != Some('{') {
            break;
        }
        let decl_block = consume_until(&mut chars, '}');
        chars.next();
        let declarations = parse_declarations(&decl_block);
        for sel_str in selector_str.split(',') {
            let sel_str = sel_str.trim();
            if let Some(selector) = Selector::parse(sel_str) {
                rules.push(CssRule {
                    selector,
                    declarations: declarations.clone(),
                    origin,
                });
            }
        }
    }

    rules
}

/// Extract `href` values from `<link rel="stylesheet" href="...">` elements.
///
/// Walks the DOM arena recursively collecting external stylesheet URLs.
/// Does NOT fetch the CSS — that's the caller's responsibility.
#[must_use]
pub fn extract_link_hrefs_arena(arena: &Arena) -> Vec<String> {
    let mut hrefs = Vec::new();
    collect_link_elements_arena(arena, arena.root(), &mut hrefs);
    hrefs
}

/// Recursively find `<link rel="stylesheet">` elements and collect their `href` attrs.
fn collect_link_elements_arena(arena: &Arena, id: NodeId, hrefs: &mut Vec<String>) {
    if let Some(NodeKind::Element { tag, attrs }) = arena.kind(id)
        && tag == "link"
    {
        let is_stylesheet = attrs
            .get("rel")
            .is_some_and(|rel| rel.eq_ignore_ascii_case("stylesheet"));
        if is_stylesheet {
            if let Some(href) = attrs.get("href") {
                let href = href.trim();
                if !href.is_empty() {
                    hrefs.push(href.to_string());
                }
            }
        }
        return;
    }
    for &child in arena.children(id) {
        collect_link_elements_arena(arena, child, hrefs);
    }
}

/// Parse declaration block into (property, value) pairs.
fn parse_declarations(block: &str) -> Vec<(String, String)> {
    block
        .split(';')
        .filter_map(|decl| {
            let mut parts = decl.splitn(2, ':');
            let prop = parts.next()?.trim();
            let val = parts.next()?.trim();
            if prop.is_empty() || val.is_empty() {
                None
            } else {
                Some((prop.to_string(), val.to_string()))
            }
        })
        .collect()
}

/// Consume characters until delimiter (not consuming delimiter).
fn consume_until(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, delimiter: char) -> String {
    let mut result = String::new();
    while let Some(&c) = chars.peek() {
        if c == delimiter {
            break;
        }
        result.push(c);
        chars.next();
    }
    result
}

/// Skip whitespace and CSS comments (`/* ... */`).
fn skip_whitespace_and_comments(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    loop {
        match chars.peek() {
            Some(c) if c.is_whitespace() => {
                chars.next();
            }
            Some('/') => {
                let mut clone = chars.clone();
                clone.next();
                if clone.peek() == Some(&'*') {
                    chars.next();
                    chars.next();
                    let mut prev = ' ';
                    for c in chars.by_ref() {
                        if prev == '*' && c == '/' {
                            break;
                        }
                        prev = c;
                    }
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_rule() {
        let rules = parse_stylesheet("div { color: red; }", StyleOrigin::Author);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].declarations.len(), 1);
        assert_eq!(rules[0].declarations[0].0, "color");
        assert_eq!(rules[0].declarations[0].1, "red");
    }

    #[test]
    fn parse_multiple_rules() {
        let css = r"
            h1 { font-size: 32px; color: #333; }
            .active { background: blue; }
        ";
        let rules = parse_stylesheet(css, StyleOrigin::Author);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].declarations.len(), 2);
        assert_eq!(rules[1].declarations.len(), 1);
    }

    #[test]
    fn parse_comma_selectors() {
        let rules = parse_stylesheet("h1, h2, h3 { color: red; }", StyleOrigin::Author);
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn parse_with_comments() {
        let css = r"
            /* Header styles */
            h1 { color: red; }
            /* Footer */
        ";
        let rules = parse_stylesheet(css, StyleOrigin::Author);
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn extract_from_arena() {
        let html = "<html><head><style>h1 { color: red; }</style></head><body></body></html>";
        let arena = Arena::parse(html);
        let rules = extract_stylesheets_arena(&arena);
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn empty_stylesheet() {
        let rules = parse_stylesheet("", StyleOrigin::Author);
        assert!(rules.is_empty());
    }

    #[test]
    fn whitespace_only() {
        let rules = parse_stylesheet("   \n\t  ", StyleOrigin::Author);
        assert!(rules.is_empty());
    }

    #[test]
    fn extract_link_stylesheet_href() {
        let html = r#"<html><head>
            <link rel="stylesheet" href="style.css">
        </head><body></body></html>"#;
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert_eq!(hrefs.len(), 1);
        assert_eq!(hrefs[0], "style.css");
    }

    #[test]
    fn extract_link_multiple_stylesheets() {
        let html = r#"<html><head>
            <link rel="stylesheet" href="a.css">
            <link rel="stylesheet" href="b.css">
        </head><body></body></html>"#;
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert_eq!(hrefs.len(), 2);
        assert_eq!(hrefs[0], "a.css");
        assert_eq!(hrefs[1], "b.css");
    }

    #[test]
    fn extract_link_ignores_non_stylesheet() {
        let html = r#"<html><head>
            <link rel="icon" href="favicon.ico">
            <link rel="stylesheet" href="style.css">
            <link rel="preload" href="font.woff">
        </head><body></body></html>"#;
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert_eq!(hrefs.len(), 1);
        assert_eq!(hrefs[0], "style.css");
    }

    #[test]
    fn extract_link_no_stylesheets() {
        let html = "<html><head></head><body></body></html>";
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert!(hrefs.is_empty());
    }

    #[test]
    fn extract_link_case_insensitive_rel() {
        let html = r#"<html><head>
            <link rel="StyleSheet" href="caps.css">
        </head><body></body></html>"#;
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert_eq!(hrefs.len(), 1);
    }

    #[test]
    fn extract_link_empty_href_ignored() {
        let html = r#"<html><head>
            <link rel="stylesheet" href="">
            <link rel="stylesheet" href="  ">
        </head><body></body></html>"#;
        let arena = Arena::parse(html);
        let hrefs = extract_link_hrefs_arena(&arena);
        assert!(hrefs.is_empty());
    }
}
