//! CSS cascade resolver.
//!
//! Collects matching rules, sorts by specificity × origin,
//! and applies declarations to produce `ComputedStyle`.

use super::selector::{Selector, Specificity};
use super::{
    AlignItems, Color, ComputedStyle, Display, FlexDirection, FlexWrap, JustifyContent, Length,
    ListStyleType, Overflow, Position, TextAlign, TextDecoration,
};
use crate::dom::{Arena, NodeId};

// ── Style origin (cascade layer) ───────────────────────────────────

/// Origin of a CSS declaration per W3C cascade order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StyleOrigin {
    /// Browser defaults (lowest priority).
    UserAgent = 0,
    /// Author stylesheets (page CSS).
    Author = 1,
    /// Inline `style` attribute (highest non-!important).
    Inline = 2,
}

// ── Matched declaration ────────────────────────────────────────────

/// A single property:value declaration matched to a node.
#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    /// CSS property name.
    pub property: String,
    /// CSS property value (unparsed).
    pub value: String,
    /// Specificity of the rule that produced this declaration.
    pub specificity: Specificity,
    /// Origin layer.
    pub origin: StyleOrigin,
    /// Source order (later = higher priority at equal specificity).
    pub source_index: usize,
}

// ── CSS Rule ───────────────────────────────────────────────────────

/// A parsed CSS rule: selector + declarations.
#[derive(Debug, Clone)]
pub struct CssRule {
    /// Parsed selector.
    pub selector: Selector,
    /// Property → value declarations.
    pub declarations: Vec<(String, String)>,
    /// Origin of this rule.
    pub origin: StyleOrigin,
}

// ── Cascade resolver ───────────────────────────────────────────────

/// Resolves the CSS cascade for a single node.
///
/// Collects all matching rules, sorts by (origin, specificity, source_index),
/// then applies declarations to produce computed values.
pub struct CascadeResolver<'a> {
    rules: &'a [CssRule],
}

impl<'a> CascadeResolver<'a> {
    /// Create a resolver with the given rule set.
    #[must_use]
    pub fn new(rules: &'a [CssRule]) -> Self {
        Self { rules }
    }

    /// Resolve computed style for a node in the arena.
    #[must_use]
    pub fn resolve_arena(
        &self,
        id: NodeId,
        arena: &Arena,
        parent_style: Option<&ComputedStyle>,
        inline_css: Option<&str>,
    ) -> ComputedStyle {
        let mut style = Self::inherited_or_default(parent_style);
        Self::apply_ua_defaults_arena(id, arena, &mut style);

        let mut matched = self.collect_matches_arena(id, arena);

        if let Some(css) = inline_css {
            let inline_decls = parse_inline_declarations(css);
            for (i, (prop, val)) in inline_decls.into_iter().enumerate() {
                matched.push(MatchedDeclaration {
                    property: prop,
                    value: val,
                    specificity: Specificity::INLINE,
                    origin: StyleOrigin::Inline,
                    source_index: i,
                });
            }
        }

        matched.sort_by(|a, b| {
            a.origin
                .cmp(&b.origin)
                .then(a.specificity.cmp(&b.specificity))
                .then(a.source_index.cmp(&b.source_index))
        });

        for decl in &matched {
            Self::apply_declaration(&decl.property, &decl.value, &mut style);
        }

        style
    }

    /// Collect all rule declarations matching this node in the arena.
    fn collect_matches_arena(&self, id: NodeId, arena: &Arena) -> Vec<MatchedDeclaration> {
        let mut result = Vec::new();
        for (source_index, rule) in self.rules.iter().enumerate() {
            if rule.selector.matches_arena(id, arena) {
                let specificity = rule.selector.specificity();
                for (prop, val) in &rule.declarations {
                    result.push(MatchedDeclaration {
                        property: prop.clone(),
                        value: val.clone(),
                        specificity,
                        origin: rule.origin,
                        source_index,
                    });
                }
            }
        }
        result
    }

    /// Apply user-agent defaults based on element tag (arena version).
    fn apply_ua_defaults_arena(id: NodeId, arena: &Arena, style: &mut ComputedStyle) {
        if let Some(tag) = arena.tag(id) {
            Self::apply_ua_for_tag(tag, style);
        }
    }

    /// Create initial style from parent (inherited properties) or defaults.
    fn inherited_or_default(parent: Option<&ComputedStyle>) -> ComputedStyle {
        match parent {
            Some(p) => ComputedStyle {
                // Inherited properties
                color: p.color,
                font_size: p.font_size,
                font_weight: p.font_weight,
                font_family: p.font_family.clone(),
                text_align: p.text_align,
                line_height: p.line_height,
                list_style_type: p.list_style_type,
                // Non-inherited (reset to defaults)
                display: Display::Block,
                position: Position::Static,
                background_color: Color::TRANSPARENT,
                text_decoration: TextDecoration::None,
                margin: Default::default(),
                padding: Default::default(),
                border: Default::default(),
                border_color: Default::default(),
                border_radius: 0.0,
                width: Length::Auto,
                height: Length::Auto,
                min_width: Length::Auto,
                max_width: Length::Auto,
                min_height: Length::Auto,
                max_height: Length::Auto,
                overflow: Overflow::Visible,
                opacity: 1.0,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::NoWrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                gap: 0.0,
                flex_grow: 0.0,
                flex_shrink: 1.0,
            },
            None => ComputedStyle {
                display: Display::Block,
                position: Position::Static,
                color: Color::BLACK,
                background_color: Color::TRANSPARENT,
                font_size: 16.0,
                font_weight: Default::default(),
                font_family: "sans-serif".to_string(),
                text_align: TextAlign::Left,
                text_decoration: TextDecoration::None,
                line_height: 1.2,
                margin: Default::default(),
                padding: Default::default(),
                border: Default::default(),
                border_color: Default::default(),
                border_radius: 0.0,
                width: Length::Auto,
                height: Length::Auto,
                min_width: Length::Auto,
                max_width: Length::Auto,
                min_height: Length::Auto,
                max_height: Length::Auto,
                overflow: Overflow::Visible,
                opacity: 1.0,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::NoWrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                gap: 0.0,
                flex_grow: 0.0,
                flex_shrink: 1.0,
                list_style_type: ListStyleType::None,
            },
        }
    }

    /// Shared UA defaults by tag name.
    fn apply_ua_for_tag(tag: &str, style: &mut ComputedStyle) {
        match tag {
            "h1" => {
                style.font_size = 32.0;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 21.44;
                style.margin.bottom = 21.44;
            }
            "h2" => {
                style.font_size = 24.0;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 19.92;
                style.margin.bottom = 19.92;
            }
            "h3" => {
                style.font_size = 18.72;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 18.72;
                style.margin.bottom = 18.72;
            }
            "p" => {
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
            }
            "body" => {
                style.background_color = Color::WHITE;
                style.margin.top = 8.0;
                style.margin.right = 8.0;
                style.margin.bottom = 8.0;
                style.margin.left = 8.0;
            }
            "b" | "strong" => {
                style.font_weight = super::FontWeight::Bold;
            }
            "hr" => {
                style.margin.top = 8.0;
                style.margin.bottom = 8.0;
                style.border.top = 1.0;
            }
            "div" | "section" | "article" | "main" | "header" | "footer" | "nav" => {
                style.display = Display::Block;
            }
            "span" | "em" | "i" | "code" | "small" | "sub" | "sup" | "abbr" => {
                style.display = Display::Inline;
            }
            "a" => {
                style.display = Display::Inline;
                style.color = Color {
                    r: 0,
                    g: 102,
                    b: 204,
                    a: 255,
                };
                style.text_decoration = super::TextDecoration::Underline;
            }
            "pre" => {
                style.font_family = "monospace".to_string();
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
                style.padding.top = 8.0;
                style.padding.right = 8.0;
                style.padding.bottom = 8.0;
                style.padding.left = 8.0;
                style.background_color = Color {
                    r: 240,
                    g: 240,
                    b: 240,
                    a: 255,
                };
                style.overflow = super::Overflow::Auto;
            }
            "blockquote" => {
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
                style.margin.left = 40.0;
                style.padding.left = 10.0;
                style.border.left = 3.0;
                style.border_color = Color {
                    r: 180,
                    g: 180,
                    b: 180,
                    a: 255,
                };
            }
            "ul" => {
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
                style.padding.left = 40.0;
                style.list_style_type = super::ListStyleType::Disc;
            }
            "ol" => {
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
                style.padding.left = 40.0;
                style.list_style_type = super::ListStyleType::Decimal;
            }
            "li" => {
                style.display = Display::Block;
            }
            "input" | "textarea" => {
                style.display = Display::Inline;
                style.border.top = 1.0;
                style.border.right = 1.0;
                style.border.bottom = 1.0;
                style.border.left = 1.0;
                style.border_color = Color {
                    r: 169,
                    g: 169,
                    b: 169,
                    a: 255,
                };
                style.padding.top = 2.0;
                style.padding.right = 4.0;
                style.padding.bottom = 2.0;
                style.padding.left = 4.0;
            }
            "button" => {
                style.display = Display::Inline;
                style.padding.top = 4.0;
                style.padding.right = 12.0;
                style.padding.bottom = 4.0;
                style.padding.left = 12.0;
                style.border.top = 1.0;
                style.border.right = 1.0;
                style.border.bottom = 1.0;
                style.border.left = 1.0;
                style.border_color = Color {
                    r: 169,
                    g: 169,
                    b: 169,
                    a: 255,
                };
                style.background_color = Color {
                    r: 240,
                    g: 240,
                    b: 240,
                    a: 255,
                };
                style.text_align = super::TextAlign::Center;
                style.border_radius = 3.0;
            }
            "center" => {
                style.text_align = super::TextAlign::Center;
            }
            "h4" => {
                style.font_size = 16.0;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 21.28;
                style.margin.bottom = 21.28;
            }
            "h5" => {
                style.font_size = 13.28;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 22.18;
                style.margin.bottom = 22.18;
            }
            "h6" => {
                style.font_size = 10.72;
                style.font_weight = super::FontWeight::Bold;
                style.margin.top = 24.97;
                style.margin.bottom = 24.97;
            }
            // Table elements
            "table" => {
                style.display = Display::Table;
                style.border.top = 1.0;
                style.border.right = 1.0;
                style.border.bottom = 1.0;
                style.border.left = 1.0;
                style.border_color = Color::BLACK;
                style.margin.top = 16.0;
                style.margin.bottom = 16.0;
            }
            "thead" | "tbody" | "tfoot" => {
                style.display = Display::Table;
            }
            "tr" => {
                style.display = Display::TableRow;
            }
            "td" => {
                style.display = Display::TableCell;
                style.padding.top = 4.0;
                style.padding.right = 8.0;
                style.padding.bottom = 4.0;
                style.padding.left = 8.0;
                style.border.top = 1.0;
                style.border.right = 1.0;
                style.border.bottom = 1.0;
                style.border.left = 1.0;
                style.border_color = Color {
                    r: 200,
                    g: 200,
                    b: 200,
                    a: 255,
                };
            }
            "th" => {
                style.display = Display::TableCell;
                style.font_weight = super::FontWeight::Bold;
                style.padding.top = 4.0;
                style.padding.right = 8.0;
                style.padding.bottom = 4.0;
                style.padding.left = 8.0;
                style.border.top = 1.0;
                style.border.right = 1.0;
                style.border.bottom = 1.0;
                style.border.left = 1.0;
                style.border_color = Color {
                    r: 200,
                    g: 200,
                    b: 200,
                    a: 255,
                };
            }
            "caption" => {
                style.display = Display::Block;
                style.margin.bottom = 8.0;
            }
            // Non-visual elements — hidden per UA stylesheet
            "head" | "title" | "meta" | "link" | "script" | "style" | "noscript" | "template"
            | "datalist" | "colgroup" | "col" | "param" | "source" | "track" | "area" | "base" => {
                style.display = Display::None;
            }
            "br" => {
                // Line break — display as block with zero height to force line break
                style.display = Display::Block;
            }
            "img" => {
                style.display = Display::Inline;
            }
            _ => {}
        }
    }

    /// Apply a single CSS declaration to the computed style.
    fn apply_declaration(property: &str, value: &str, style: &mut ComputedStyle) {
        match property {
            "color" => {
                if let Some(c) = Color::parse(value) {
                    style.color = c;
                }
            }
            "background-color" | "background" => {
                if let Some(c) = Color::parse(value) {
                    style.background_color = c;
                }
            }
            "font-size" => {
                if let Some(len) = parse_length(value) {
                    style.font_size = len.to_px(0.0, style.font_size);
                }
            }
            "font-family" => {
                style.font_family = value.trim_matches(|c| c == '"' || c == '\'').to_string();
            }
            "display" => {
                style.display = match value.trim() {
                    "block" => Display::Block,
                    "inline" => Display::Inline,
                    "flex" => Display::Flex,
                    "none" => Display::None,
                    _ => style.display,
                };
            }
            "width" => {
                if let Some(len) = parse_length(value) {
                    style.width = len;
                }
            }
            "height" => {
                if let Some(len) = parse_length(value) {
                    style.height = len;
                }
            }
            "margin" => {
                if let Some(len) = parse_length(value) {
                    let px = len.to_px(0.0, style.font_size);
                    style.margin.top = px;
                    style.margin.right = px;
                    style.margin.bottom = px;
                    style.margin.left = px;
                }
            }
            "margin-top" => {
                if let Some(len) = parse_length(value) {
                    style.margin.top = len.to_px(0.0, style.font_size);
                }
            }
            "margin-right" => {
                if let Some(len) = parse_length(value) {
                    style.margin.right = len.to_px(0.0, style.font_size);
                }
            }
            "margin-bottom" => {
                if let Some(len) = parse_length(value) {
                    style.margin.bottom = len.to_px(0.0, style.font_size);
                }
            }
            "margin-left" => {
                if let Some(len) = parse_length(value) {
                    style.margin.left = len.to_px(0.0, style.font_size);
                }
            }
            "padding" => {
                if let Some(len) = parse_length(value) {
                    let px = len.to_px(0.0, style.font_size);
                    style.padding.top = px;
                    style.padding.right = px;
                    style.padding.bottom = px;
                    style.padding.left = px;
                }
            }
            "padding-top" => {
                if let Some(len) = parse_length(value) {
                    style.padding.top = len.to_px(0.0, style.font_size);
                }
            }
            "padding-right" => {
                if let Some(len) = parse_length(value) {
                    style.padding.right = len.to_px(0.0, style.font_size);
                }
            }
            "padding-bottom" => {
                if let Some(len) = parse_length(value) {
                    style.padding.bottom = len.to_px(0.0, style.font_size);
                }
            }
            "padding-left" => {
                if let Some(len) = parse_length(value) {
                    style.padding.left = len.to_px(0.0, style.font_size);
                }
            }
            "font-weight" => {
                style.font_weight = match value.trim() {
                    "bold" | "700" | "800" | "900" => super::FontWeight::Bold,
                    "normal" | "400" => super::FontWeight::Normal,
                    _ => style.font_weight,
                };
            }
            "border-color" => {
                if let Some(c) = Color::parse(value) {
                    style.border_color = c;
                }
            }
            "border" => {
                // Shorthand: "1px solid #333"
                let parts: Vec<&str> = value.split_whitespace().collect();
                for part in &parts {
                    if let Some(len) = parse_length(part) {
                        let px = len.to_px(0.0, style.font_size);
                        style.border.top = px;
                        style.border.right = px;
                        style.border.bottom = px;
                        style.border.left = px;
                    } else if let Some(c) = Color::parse(part) {
                        style.border_color = c;
                    }
                    // "solid", "dashed" etc. — accepted but not differentiated
                }
            }
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                let parts: Vec<&str> = value.split_whitespace().collect();
                let mut width = 0.0_f32;
                let mut color = style.border_color;
                for part in &parts {
                    if let Some(len) = parse_length(part) {
                        width = len.to_px(0.0, style.font_size);
                    } else if let Some(c) = Color::parse(part) {
                        color = c;
                    }
                }
                match property {
                    "border-top" => style.border.top = width,
                    "border-right" => style.border.right = width,
                    "border-bottom" => style.border.bottom = width,
                    "border-left" => style.border.left = width,
                    _ => {}
                }
                style.border_color = color;
            }
            "border-radius" => {
                if let Some(len) = parse_length(value) {
                    style.border_radius = len.to_px(0.0, style.font_size);
                }
            }
            "border-width" => {
                if let Some(len) = parse_length(value) {
                    let px = len.to_px(0.0, style.font_size);
                    style.border.top = px;
                    style.border.right = px;
                    style.border.bottom = px;
                    style.border.left = px;
                }
            }
            "text-align" => {
                style.text_align = match value.trim() {
                    "left" => TextAlign::Left,
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => style.text_align,
                };
            }
            "text-decoration" | "text-decoration-line" => {
                style.text_decoration = match value.trim() {
                    "none" => TextDecoration::None,
                    "underline" => TextDecoration::Underline,
                    "line-through" => TextDecoration::LineThrough,
                    _ => style.text_decoration,
                };
            }
            "line-height" => {
                let v = value.trim();
                if v == "normal" {
                    style.line_height = 1.2;
                } else if let Some(len) = parse_length(v) {
                    let px = len.to_px(0.0, style.font_size);
                    if px > 0.0 {
                        style.line_height = px / style.font_size;
                    }
                } else if let Ok(ratio) = v.parse::<f32>() {
                    if ratio > 0.0 {
                        style.line_height = ratio;
                    }
                }
            }
            "opacity" => {
                if let Ok(o) = value.trim().parse::<f32>() {
                    style.opacity = o.clamp(0.0, 1.0);
                }
            }
            "overflow" => {
                style.overflow = match value.trim() {
                    "visible" => Overflow::Visible,
                    "hidden" => Overflow::Hidden,
                    "scroll" => Overflow::Scroll,
                    "auto" => Overflow::Auto,
                    _ => style.overflow,
                };
            }
            "position" => {
                style.position = match value.trim() {
                    "static" => Position::Static,
                    "relative" => Position::Relative,
                    "absolute" => Position::Absolute,
                    "fixed" => Position::Fixed,
                    _ => style.position,
                };
            }
            "flex-direction" => {
                style.flex_direction = match value.trim() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => style.flex_direction,
                };
            }
            "flex-wrap" => {
                style.flex_wrap = match value.trim() {
                    "nowrap" => FlexWrap::NoWrap,
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => style.flex_wrap,
                };
            }
            "justify-content" => {
                style.justify_content = match value.trim() {
                    "flex-start" | "start" => JustifyContent::FlexStart,
                    "flex-end" | "end" => JustifyContent::FlexEnd,
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    _ => style.justify_content,
                };
            }
            "align-items" => {
                style.align_items = match value.trim() {
                    "stretch" => AlignItems::Stretch,
                    "flex-start" | "start" => AlignItems::FlexStart,
                    "flex-end" | "end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    _ => style.align_items,
                };
            }
            "gap" | "row-gap" | "column-gap" => {
                if let Some(len) = parse_length(value) {
                    style.gap = len.to_px(0.0, style.font_size);
                }
            }
            "flex-grow" => {
                if let Ok(v) = value.trim().parse::<f32>() {
                    style.flex_grow = v;
                }
            }
            "flex-shrink" => {
                if let Ok(v) = value.trim().parse::<f32>() {
                    style.flex_shrink = v;
                }
            }
            "flex" => {
                // Shorthand: "1" or "1 0 auto" or "none"
                let v = value.trim();
                if v == "none" {
                    style.flex_grow = 0.0;
                    style.flex_shrink = 0.0;
                } else if let Ok(g) = v.parse::<f32>() {
                    style.flex_grow = g;
                    style.flex_shrink = 1.0;
                }
            }
            "max-width" => {
                if let Some(len) = parse_length(value) {
                    style.max_width = len;
                }
            }
            "min-width" => {
                if let Some(len) = parse_length(value) {
                    style.min_width = len;
                }
            }
            "max-height" => {
                if let Some(len) = parse_length(value) {
                    style.max_height = len;
                }
            }
            "min-height" => {
                if let Some(len) = parse_length(value) {
                    style.min_height = len;
                }
            }
            "list-style-type" | "list-style" => {
                style.list_style_type = match value.trim() {
                    "none" => ListStyleType::None,
                    "disc" => ListStyleType::Disc,
                    "circle" => ListStyleType::Circle,
                    "square" => ListStyleType::Square,
                    "decimal" => ListStyleType::Decimal,
                    _ => style.list_style_type,
                };
            }
            _ => {}
        }
    }
}

// ── Value parsers ──────────────────────────────────────────────────

/// Parse a CSS length value.
#[must_use]
pub fn parse_length(value: &str) -> Option<Length> {
    let value = value.trim();
    if value == "auto" {
        return Some(Length::Auto);
    }
    if value == "0" {
        return Some(Length::Px(0.0));
    }
    if let Some(px) = value.strip_suffix("px") {
        return px.trim().parse::<f32>().ok().map(Length::Px);
    }
    if let Some(em) = value.strip_suffix("em") {
        return em.trim().parse::<f32>().ok().map(Length::Em);
    }
    if let Some(pct) = value.strip_suffix('%') {
        return pct.trim().parse::<f32>().ok().map(Length::Percent);
    }
    value.parse::<f32>().ok().map(Length::Px)
}

/// Parse inline style declarations into (property, value) pairs.
#[must_use]
pub fn parse_inline_declarations(css: &str) -> Vec<(String, String)> {
    css.split(';')
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

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::{Arena, NodeKind};
    use std::collections::BTreeMap;

    fn make_arena_div(class: &str, id: &str) -> (Arena, NodeId) {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let mut attrs = BTreeMap::new();
        if !class.is_empty() {
            attrs.insert("class".to_string(), class.to_string());
        }
        if !id.is_empty() {
            attrs.insert("id".to_string(), id.to_string());
        }
        let div = arena.alloc(
            NodeKind::Element {
                tag: "div".to_string(),
                attrs,
            },
            Some(root),
        );
        (arena, div)
    }

    #[test]
    fn cascade_empty_rules_arena() {
        let (arena, div) = make_arena_div("", "");
        let resolver = CascadeResolver::new(&[]);
        let style = resolver.resolve_arena(div, &arena, None, None);
        assert_eq!(style.display, Display::Block);
        assert_eq!(style.font_size, 16.0);
    }

    #[test]
    fn cascade_tag_rule_arena() {
        let (arena, div) = make_arena_div("", "");
        let rules = vec![CssRule {
            selector: Selector::parse("div").unwrap(),
            declarations: vec![("color".to_string(), "#ff0000".to_string())],
            origin: StyleOrigin::Author,
        }];
        let resolver = CascadeResolver::new(&rules);
        let style = resolver.resolve_arena(div, &arena, None, None);
        assert_eq!(style.color.r, 255);
        assert_eq!(style.color.g, 0);
    }

    #[test]
    fn cascade_specificity_order_arena() {
        let (arena, div) = make_arena_div("active", "");
        let rules = vec![
            CssRule {
                selector: Selector::parse("div").unwrap(),
                declarations: vec![("color".to_string(), "#ff0000".to_string())],
                origin: StyleOrigin::Author,
            },
            CssRule {
                selector: Selector::parse(".active").unwrap(),
                declarations: vec![("color".to_string(), "#0000ff".to_string())],
                origin: StyleOrigin::Author,
            },
        ];
        let resolver = CascadeResolver::new(&rules);
        let style = resolver.resolve_arena(div, &arena, None, None);
        assert_eq!(style.color.b, 255);
        assert_eq!(style.color.r, 0);
    }

    #[test]
    fn inline_beats_author_arena() {
        let (arena, div) = make_arena_div("", "main");
        let rules = vec![CssRule {
            selector: Selector::parse("#main").unwrap(),
            declarations: vec![("color".to_string(), "#ff0000".to_string())],
            origin: StyleOrigin::Author,
        }];
        let resolver = CascadeResolver::new(&rules);
        let style = resolver.resolve_arena(div, &arena, None, Some("color: #00ff00"));
        assert_eq!(style.color.g, 255);
        assert_eq!(style.color.r, 0);
    }

    #[test]
    fn inheritance_from_parent_arena() {
        let parent = ComputedStyle {
            color: Color {
                r: 100,
                g: 100,
                b: 100,
                a: 255,
            },
            font_size: 20.0,
            font_family: "monospace".to_string(),
            ..Default::default()
        };
        let (arena, div) = make_arena_div("", "");
        let resolver = CascadeResolver::new(&[]);
        let style = resolver.resolve_arena(div, &arena, Some(&parent), None);
        assert_eq!(style.color.r, 100);
        assert_eq!(style.font_size, 20.0);
        assert_eq!(style.font_family, "monospace");
        assert_eq!(style.background_color.a, 0);
    }

    #[test]
    fn ua_defaults_h1_arena() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let h1 = arena.alloc(
            NodeKind::Element {
                tag: "h1".to_string(),
                attrs: BTreeMap::new(),
            },
            Some(root),
        );
        let resolver = CascadeResolver::new(&[]);
        let style = resolver.resolve_arena(h1, &arena, None, None);
        assert_eq!(style.font_size, 32.0);
    }

    #[test]
    fn parse_length_values() {
        assert!(matches!(parse_length("auto"), Some(Length::Auto)));
        assert!(matches!(parse_length("16px"), Some(Length::Px(v)) if (v - 16.0).abs() < 0.01));
        assert!(matches!(parse_length("1.5em"), Some(Length::Em(v)) if (v - 1.5).abs() < 0.01));
        assert!(matches!(parse_length("50%"), Some(Length::Percent(v)) if (v - 50.0).abs() < 0.01));
        assert!(matches!(parse_length("0"), Some(Length::Px(v)) if v.abs() < 0.01));
    }

    #[test]
    fn parse_inline_decl() {
        let decls = parse_inline_declarations("color: red; font-size: 14px;");
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].0, "color");
        assert_eq!(decls[0].1, "red");
        assert_eq!(decls[1].0, "font-size");
        assert_eq!(decls[1].1, "14px");
    }

    #[test]
    fn ua_hides_head_title_style_script() {
        let html = "<html><head><title>Test</title><style>p{}</style></head><body><p>Hello</p></body></html>";
        let arena = Arena::parse(html);
        let styled = super::super::StyledNode::from_arena(&arena, &[]);

        fn find_display(
            node: &super::super::StyledNode,
            arena: &Arena,
            results: &mut Vec<(String, super::super::Display)>,
        ) {
            if let Some(tag) = arena.tag(node.node_id) {
                results.push((tag.to_string(), node.style.display));
            }
            for child in &node.children {
                find_display(child, arena, results);
            }
        }

        let mut results = Vec::new();
        find_display(&styled, &arena, &mut results);

        for (tag, disp) in &results {
            if matches!(tag.as_str(), "head" | "title" | "style") {
                assert_eq!(
                    *disp,
                    super::super::Display::None,
                    "{tag} should be Display::None"
                );
            }
            if matches!(tag.as_str(), "body" | "p" | "html") {
                assert_ne!(
                    *disp,
                    super::super::Display::None,
                    "{tag} should NOT be Display::None"
                );
            }
        }
    }
}
