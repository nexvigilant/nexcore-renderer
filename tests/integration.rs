//! Integration tests for nexcore-renderer.
//!
//! CTVP Phase 0 (Preclinical) — validates real pipeline behavior
//! without mocks. Covers the 4 highest-priority test categories
//! identified by CTVP analysis.
//!
//! Migrated to Arena-based DOM (no legacy `Node` usage).

use nexcore_renderer::app::Browser;
use nexcore_renderer::dom::Arena;
use nexcore_renderer::layout::{LayoutEngine, Rect};
use nexcore_renderer::paint::{DisplayCommand, build_display_list, build_hit_regions};
use nexcore_renderer::style::{
    Color, ComputedStyle, Length, StyledNode,
    cascade::{CascadeResolver, CssRule, StyleOrigin},
    parse::{extract_stylesheets_arena, parse_stylesheet},
    selector::{Selector, Specificity},
};

// ═══════════════════════════════════════════════════════════════════
// Priority 1: Style pure functions (12 tests)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn color_parse_hex6() {
    let c = Color::parse("#ff8800").expect("valid hex6");
    assert_eq!((c.r, c.g, c.b, c.a), (255, 136, 0, 255));
}

#[test]
fn color_parse_hex3() {
    let c = Color::parse("#f80").expect("valid hex3");
    // #f80 → #ff8800
    assert_eq!((c.r, c.g, c.b), (255, 136, 0));
}

#[test]
fn color_parse_named() {
    assert!(Color::parse("red").is_some());
    assert!(Color::parse("white").is_some());
    assert!(Color::parse("transparent").is_some());
}

#[test]
fn color_parse_invalid() {
    assert!(Color::parse("").is_none());
    assert!(Color::parse("notacolor").is_none());
    assert!(Color::parse("#xyz").is_none());
    assert!(Color::parse("#12345").is_none());
}

#[test]
fn length_px_conversion() {
    let len = Length::Px(24.0);
    assert!((len.to_px(100.0, 16.0) - 24.0).abs() < 0.01);
}

#[test]
fn length_em_conversion() {
    let len = Length::Em(1.5);
    assert!((len.to_px(100.0, 16.0) - 24.0).abs() < 0.01);
}

#[test]
fn length_percent_conversion() {
    let len = Length::Percent(50.0);
    assert!((len.to_px(200.0, 16.0) - 100.0).abs() < 0.01);
}

#[test]
fn length_auto_conversion() {
    let len = Length::Auto;
    assert!((len.to_px(200.0, 16.0)).abs() < 0.01);
}

#[test]
fn specificity_id_beats_class() {
    let id = Specificity(0, 1, 0, 0);
    let class = Specificity(0, 0, 1, 0);
    assert!(id > class);
}

#[test]
fn specificity_inline_beats_all() {
    let inline = Specificity::INLINE;
    let id_class_tag = Specificity(0, 1, 1, 1);
    assert!(inline > id_class_tag);
}

#[test]
fn cascade_inheritance_propagates_color() {
    let parent = ComputedStyle {
        color: Color {
            r: 42,
            g: 42,
            b: 42,
            a: 255,
        },
        font_size: 20.0,
        ..Default::default()
    };
    let arena = Arena::parse("<span></span>");
    // Find the <span> node
    let span_id = arena.find_tag(arena.root(), "span").expect("span exists");
    let resolver = CascadeResolver::new(&[]);
    let style = resolver.resolve_arena(span_id, &arena, Some(&parent), None);
    assert_eq!(style.color.r, 42);
    assert_eq!(style.font_size, 20.0);
}

#[test]
fn cascade_inline_overrides_stylesheet() {
    let rules = vec![CssRule {
        selector: Selector::parse("p").expect("valid"),
        declarations: vec![("color".to_string(), "#ff0000".to_string())],
        origin: StyleOrigin::Author,
    }];
    let resolver = CascadeResolver::new(&rules);
    let arena = Arena::parse("<p></p>");
    let p_id = arena.find_tag(arena.root(), "p").expect("p exists");
    let style = resolver.resolve_arena(p_id, &arena, None, Some("color: #0000ff"));
    assert_eq!(style.color.b, 255); // Inline blue wins
    assert_eq!(style.color.r, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Priority 2: DOM parsing (5 tests) — Arena-based
// ═══════════════════════════════════════════════════════════════════

#[test]
fn dom_parse_returns_document() {
    let arena = Arena::parse("<html><body></body></html>");
    // Root node should be a Document
    assert!(arena.tag(arena.root()).is_none()); // Document has no tag
    assert!(arena.len() > 1);
}

#[test]
fn dom_parse_preserves_text() {
    let arena = Arena::parse("<p>Hello World</p>");
    let text_id = arena.find_text(arena.root());
    assert!(text_id.is_some(), "Should find a text node");
    let content = arena.text(text_id.unwrap()).unwrap();
    assert!(content.contains("Hello World"));
}

#[test]
fn dom_parse_preserves_attributes() {
    let arena = Arena::parse(r#"<a href="https://example.com">Link</a>"#);
    let a_id = arena.find_tag(arena.root(), "a").expect("a tag exists");
    let href = arena.attrs(a_id).and_then(|a| a.get("href").cloned());
    assert_eq!(href, Some("https://example.com".to_string()));
}

#[test]
fn dom_parse_nested_structure() {
    let arena = Arena::parse("<div><p><span>deep</span></p></div>");
    // Find span, walk ancestors: span → p → div → body → html → document
    let span_id = arena.find_tag(arena.root(), "span").expect("span exists");
    let ancestor_count = arena.ancestors(span_id).count();
    assert!(
        ancestor_count >= 3,
        "span should have ≥3 ancestors (p, div, body...)"
    );
}

#[test]
fn dom_children_empty_for_text() {
    let arena = Arena::parse("<p>hello</p>");
    // Find the text node
    let p_id = arena.find_tag(arena.root(), "p").expect("p exists");
    // Text nodes have no children
    for &child in arena.children(p_id) {
        if arena.text(child).is_some() {
            assert!(arena.children(child).is_empty());
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Priority 3: Net module (4 tests) — unchanged
// ═══════════════════════════════════════════════════════════════════

#[test]
fn net_fetch_data_url() {
    let html =
        nexcore_renderer::net::fetch("data:text/html,<h1>Hello</h1>").expect("data URL fetch");
    assert!(html.contains("<h1>Hello</h1>"));
}

#[test]
fn net_fetch_invalid_scheme() {
    let result = nexcore_renderer::net::fetch("ftp://example.com");
    assert!(result.is_err());
}

#[test]
fn net_resolve_relative() {
    let resolved =
        nexcore_renderer::net::resolve("https://example.com/page/index.html", "../about.html");
    assert_eq!(resolved, Some("https://example.com/about.html".to_string()));
}

#[test]
fn net_resolve_absolute() {
    let resolved =
        nexcore_renderer::net::resolve("https://example.com/page/", "https://other.com/path");
    assert_eq!(resolved, Some("https://other.com/path".to_string()));
}

// ═══════════════════════════════════════════════════════════════════
// Priority 4: End-to-end pipeline (6 tests) — Arena-based
// ═══════════════════════════════════════════════════════════════════

#[test]
fn pipeline_dom_to_styled() {
    let html = r#"<html><body><h1 style="color: #ff0000">Red</h1></body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);
    // Find the h1's styled node by walking the tree
    fn find_h1<'a>(s: &'a StyledNode, arena: &Arena) -> Option<&'a StyledNode> {
        if arena.tag(s.node_id) == Some("h1") {
            return Some(s);
        }
        s.children.iter().find_map(|c| find_h1(c, arena))
    }
    let h1 = find_h1(&styled, &arena).expect("h1 in styled tree");
    assert_eq!(h1.style.color.r, 255);
    assert_eq!(h1.style.font_size, 32.0);
}

#[test]
fn pipeline_styled_to_layout() {
    let html = "<html><body><div>Hello</div></body></html>";
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let mut engine = LayoutEngine::new();
    let layout = engine.layout(&styled, viewport, &arena);
    assert!(layout.rect.width > 0.0);
}

#[test]
fn pipeline_layout_to_display_list() {
    let html = r#"<html><body style="background-color: #333"><p>text</p></body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let mut engine = LayoutEngine::new();
    let layout = engine.layout(&styled, viewport, &arena);
    let display_list = build_display_list(&layout);
    assert!(!display_list.is_empty());
    let has_text = display_list
        .iter()
        .any(|cmd| matches!(cmd, DisplayCommand::DrawText { text, .. } if text.contains("text")));
    assert!(has_text, "Display list should contain 'text'");
}

#[test]
fn pipeline_hit_regions_from_display_list() {
    let html = "<html><body><div>Clickable</div></body></html>";
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let mut engine = LayoutEngine::new();
    let layout = engine.layout(&styled, viewport, &arena);
    let display_list = build_display_list(&layout);
    let regions = build_hit_regions(&display_list);
    assert!(
        !regions.is_empty(),
        "Hit regions should exist for rendered content"
    );
}

#[test]
fn pipeline_style_block_extraction() {
    let html = r#"
        <html>
        <head><style>h1 { color: #00ff00; }</style></head>
        <body><h1>Green</h1></body>
        </html>
    "#;
    let arena = Arena::parse(html);
    let rules = extract_stylesheets_arena(&arena);
    assert!(!rules.is_empty(), "Should extract CSS rules from <style>");

    let styled = StyledNode::from_arena(&arena, &[]);
    fn find_h1<'a>(s: &'a StyledNode, arena: &Arena) -> Option<&'a StyledNode> {
        if arena.tag(s.node_id) == Some("h1") {
            return Some(s);
        }
        s.children.iter().find_map(|c| find_h1(c, arena))
    }
    let h1 = find_h1(&styled, &arena).expect("h1 exists");
    assert_eq!(
        h1.style.color.g, 255,
        "h1 should be green from <style> block"
    );
}

#[test]
fn pipeline_browser_navigate_data_url() {
    let mut browser = Browser::new();
    let result = browser.navigate("data:text/html,<h1>Test</h1>");
    assert!(result.is_ok());
    assert_eq!(browser.current_url(), "data:text/html,<h1>Test</h1>");
    assert!(!browser.display_list().is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Priority 5: CSS parse module (4 tests) — unchanged
// ═══════════════════════════════════════════════════════════════════

#[test]
fn css_parse_multiple_declarations() {
    let rules = parse_stylesheet(
        "div { color: red; font-size: 20px; background: #333; }",
        StyleOrigin::Author,
    );
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].declarations.len(), 3);
}

#[test]
fn css_parse_comma_selectors_share_declarations() {
    let rules = parse_stylesheet("h1, h2 { color: blue; }", StyleOrigin::Author);
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].declarations[0].1, "blue");
    assert_eq!(rules[1].declarations[0].1, "blue");
}

#[test]
fn css_parse_ignores_comments() {
    let rules = parse_stylesheet(
        "/* comment */ p { color: red; } /* end */",
        StyleOrigin::Author,
    );
    assert_eq!(rules.len(), 1);
}

#[test]
fn css_specificity_compound_calculates_correctly() {
    let sel = Selector::parse("div.active#main").expect("valid compound");
    assert_eq!(sel.specificity(), Specificity(0, 1, 1, 1));
}

// ═══════════════════════════════════════════════════════════════════
// Priority 6: Rect and hit-testing (3 tests) — unchanged
// ═══════════════════════════════════════════════════════════════════

#[test]
fn rect_contains_interior_point() {
    let r = Rect {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 50.0,
    };
    assert!(r.contains(50.0, 30.0));
}

#[test]
fn rect_excludes_exterior_point() {
    let r = Rect {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 50.0,
    };
    assert!(!r.contains(5.0, 5.0));
    assert!(!r.contains(200.0, 200.0));
}

#[test]
fn rect_boundary_behavior() {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(r.contains(0.0, 0.0)); // top-left corner included
    assert!(!r.contains(100.0, 100.0)); // bottom-right excluded (half-open)
}

// ═══════════════════════════════════════════════════════════════════
// Arena-specific tests (new — validates arena API)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn arena_root_is_zero() {
    let arena = Arena::parse("<p>hi</p>");
    assert_eq!(arena.root().0, 0);
}

#[test]
fn arena_parent_child_consistency() {
    let arena = Arena::parse("<div><p>text</p></div>");
    let div_id = arena.find_tag(arena.root(), "div").expect("div");
    for &child in arena.children(div_id) {
        assert_eq!(arena.parent(child), Some(div_id));
    }
}

#[test]
fn arena_ancestor_walk_reaches_root() {
    let arena = Arena::parse("<div><p><span>deep</span></p></div>");
    let span_id = arena.find_tag(arena.root(), "span").expect("span");
    let ancestors: Vec<_> = arena.ancestors(span_id).collect();
    // Last ancestor should be the root
    assert!(!ancestors.is_empty());
    // Root's parent is None, so root itself isn't in ancestors —
    // but its parent (None) terminates the walk. Check we reach near root.
    let last = *ancestors.last().expect("has ancestors");
    // The last ancestor's parent should be None (it IS root) or its parent is root
    assert!(arena.parent(last).is_none() || arena.parent(last) == Some(arena.root()));
}

#[test]
fn arena_find_tag_returns_first() {
    let arena = Arena::parse("<div><p>first</p><p>second</p></div>");
    let p_id = arena.find_tag(arena.root(), "p").expect("p exists");
    // Should find first <p>
    let text_id = *arena.children(p_id).first().expect("p has child");
    assert_eq!(arena.text(text_id).map(|t| t.trim()), Some("first"));
}

// ═══════════════════════════════════════════════════════════════════
// Table layout tests
// ═══════════════════════════════════════════════════════════════════

use nexcore_renderer::style::{Display, FontWeight};

#[test]
fn table_elements_get_correct_display() {
    let html = r#"<html><body>
        <table><tr><th>Header</th><td>Cell</td></tr></table>
    </body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);

    fn find_display<'a>(s: &'a StyledNode, arena: &Arena, tag: &str) -> Option<Display> {
        if arena.tag(s.node_id) == Some(tag) {
            return Some(s.style.display);
        }
        s.children.iter().find_map(|c| find_display(c, arena, tag))
    }

    assert_eq!(
        find_display(&styled, &arena, "table"),
        Some(Display::Table),
        "table should have Display::Table"
    );
    assert_eq!(
        find_display(&styled, &arena, "tr"),
        Some(Display::TableRow),
        "tr should have Display::TableRow"
    );
    assert_eq!(
        find_display(&styled, &arena, "td"),
        Some(Display::TableCell),
        "td should have Display::TableCell"
    );
    assert_eq!(
        find_display(&styled, &arena, "th"),
        Some(Display::TableCell),
        "th should have Display::TableCell"
    );
}

#[test]
fn table_th_is_bold() {
    let html = "<html><body><table><tr><th>Bold</th><td>Normal</td></tr></table></body></html>";
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);

    fn find_weight<'a>(s: &'a StyledNode, arena: &Arena, tag: &str) -> Option<FontWeight> {
        if arena.tag(s.node_id) == Some(tag) {
            return Some(s.style.font_weight);
        }
        s.children.iter().find_map(|c| find_weight(c, arena, tag))
    }

    assert_eq!(
        find_weight(&styled, &arena, "th"),
        Some(FontWeight::Bold),
        "th should be bold"
    );
    assert_eq!(
        find_weight(&styled, &arena, "td"),
        Some(FontWeight::Normal),
        "td should be normal weight"
    );
}

#[test]
fn table_produces_layout() {
    let html = r#"<html><body>
        <table>
            <tr><th>A</th><th>B</th></tr>
            <tr><td>1</td><td>2</td></tr>
        </table>
    </body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);
    let mut engine = LayoutEngine::new();
    let viewport = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let layout = engine.layout(&styled, viewport, &arena);
    // Table should produce a non-empty layout tree
    assert!(layout.rect.width > 0.0, "Table layout should have width");
    let cmds = build_display_list(&layout);
    assert!(!cmds.is_empty(), "Table should produce display commands");
}

#[test]
fn table_navigate_data_url() {
    let mut browser = Browser::new();
    let result = browser.navigate("data:text/html,<table><tr><td>Cell</td></tr></table>");
    assert!(result.is_ok());
    assert!(!browser.display_list().is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// Extended color tests (integration)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn color_parse_rgb_in_style() {
    let html = r#"<html><body><p style="color: rgb(100, 200, 50);">text</p></body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);

    fn find_p<'a>(s: &'a StyledNode, arena: &Arena) -> Option<&'a StyledNode> {
        if arena.tag(s.node_id) == Some("p") {
            return Some(s);
        }
        s.children.iter().find_map(|c| find_p(c, arena))
    }

    let p = find_p(&styled, &arena).expect("p exists");
    assert_eq!(
        (p.style.color.r, p.style.color.g, p.style.color.b),
        (100, 200, 50)
    );
}

#[test]
fn color_parse_named_in_stylesheet() {
    let html = r#"<html><head><style>p { color: tomato; }</style></head>
        <body><p>text</p></body></html>"#;
    let arena = Arena::parse(html);
    let styled = StyledNode::from_arena(&arena, &[]);

    fn find_p<'a>(s: &'a StyledNode, arena: &Arena) -> Option<&'a StyledNode> {
        if arena.tag(s.node_id) == Some("p") {
            return Some(s);
        }
        s.children.iter().find_map(|c| find_p(c, arena))
    }

    let p = find_p(&styled, &arena).expect("p exists");
    assert_eq!(
        (p.style.color.r, p.style.color.g, p.style.color.b),
        (255, 99, 71)
    );
}
