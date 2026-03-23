//! CSS styling and cascade.
//!
//! Applies CSS rules to DOM nodes to create styled nodes.
//! Sub-modules implement W3C-style selector matching and cascade resolution.

pub mod cascade;
pub mod parse;
pub mod selector;

use crate::dom::{Arena, NodeId};
use cascade::{CascadeResolver, CssRule};
use std::collections::BTreeMap;

/// Color value (T3 Domain Type).
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Parse color from CSS string.
    ///
    /// Supports: named colors (all 148 CSS colors), `#hex`, `rgb()`, `rgba()`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "transparent" {
            return Some(Self::TRANSPARENT);
        }
        if s.starts_with('#') {
            return Self::parse_hex(s);
        }
        if s.starts_with("rgb") {
            return Self::parse_rgb_func(s);
        }
        Self::parse_named(s)
    }

    fn parse_hex(s: &str) -> Option<Self> {
        let hex = s.trim_start_matches('#');
        match hex.len() {
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self { r, g, b, a })
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self { r, g, b, a: 255 })
            }
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(Self { r, g, b, a: 255 })
            }
            _ => None,
        }
    }

    /// Parse `rgb(r, g, b)` or `rgba(r, g, b, a)`.
    fn parse_rgb_func(s: &str) -> Option<Self> {
        let inner = s
            .strip_prefix("rgba(")
            .or_else(|| s.strip_prefix("rgb("))?
            .strip_suffix(')')?;
        let parts: Vec<&str> = inner.split(',').collect();
        match parts.len() {
            3 => {
                let r = parts[0].trim().parse::<u8>().ok()?;
                let g = parts[1].trim().parse::<u8>().ok()?;
                let b = parts[2].trim().parse::<u8>().ok()?;
                Some(Self { r, g, b, a: 255 })
            }
            4 => {
                let r = parts[0].trim().parse::<u8>().ok()?;
                let g = parts[1].trim().parse::<u8>().ok()?;
                let b = parts[2].trim().parse::<u8>().ok()?;
                let af: f32 = parts[3].trim().parse().ok()?;
                let a = (af.clamp(0.0, 1.0) * 255.0) as u8;
                Some(Self { r, g, b, a })
            }
            _ => None,
        }
    }

    /// Lookup from the 148 CSS named colors (sorted, binary search).
    fn parse_named(s: &str) -> Option<Self> {
        // Case-insensitive: CSS color names are case-insensitive
        let lower = s.to_ascii_lowercase();
        NAMED_COLORS
            .binary_search_by_key(&lower.as_str(), |&(name, _)| name)
            .ok()
            .map(|i| {
                let (_, [r, g, b]) = NAMED_COLORS[i];
                Self { r, g, b, a: 255 }
            })
    }
}

/// All 148 CSS named colors, sorted alphabetically for binary search.
#[rustfmt::skip]
static NAMED_COLORS: &[(&str, [u8; 3])] = &[
    ("aliceblue",            [240, 248, 255]),
    ("antiquewhite",         [250, 235, 215]),
    ("aqua",                 [  0, 255, 255]),
    ("aquamarine",           [127, 255, 212]),
    ("azure",                [240, 255, 255]),
    ("beige",                [245, 245, 220]),
    ("bisque",               [255, 228, 196]),
    ("black",                [  0,   0,   0]),
    ("blanchedalmond",       [255, 235, 205]),
    ("blue",                 [  0,   0, 255]),
    ("blueviolet",           [138,  43, 226]),
    ("brown",                [165,  42,  42]),
    ("burlywood",            [222, 184, 135]),
    ("cadetblue",            [ 95, 158, 160]),
    ("chartreuse",           [127, 255,   0]),
    ("chocolate",            [210, 105,  30]),
    ("coral",                [255, 127,  80]),
    ("cornflowerblue",       [100, 149, 237]),
    ("cornsilk",             [255, 248, 220]),
    ("crimson",              [220,  20,  60]),
    ("cyan",                 [  0, 255, 255]),
    ("darkblue",             [  0,   0, 139]),
    ("darkcyan",             [  0, 139, 139]),
    ("darkgoldenrod",        [184, 134,  11]),
    ("darkgray",             [169, 169, 169]),
    ("darkgreen",            [  0, 100,   0]),
    ("darkgrey",             [169, 169, 169]),
    ("darkkhaki",            [189, 183, 107]),
    ("darkmagenta",          [139,   0, 139]),
    ("darkolivegreen",       [ 85, 107,  47]),
    ("darkorange",           [255, 140,   0]),
    ("darkorchid",           [153,  50, 204]),
    ("darkred",              [139,   0,   0]),
    ("darksalmon",           [233, 150, 122]),
    ("darkseagreen",         [143, 188, 143]),
    ("darkslateblue",        [ 72,  61, 139]),
    ("darkslategray",        [ 47,  79,  79]),
    ("darkslategrey",        [ 47,  79,  79]),
    ("darkturquoise",        [  0, 206, 209]),
    ("darkviolet",           [148,   0, 211]),
    ("deeppink",             [255,  20, 147]),
    ("deepskyblue",          [  0, 191, 255]),
    ("dimgray",              [105, 105, 105]),
    ("dimgrey",              [105, 105, 105]),
    ("dodgerblue",           [ 30, 144, 255]),
    ("firebrick",            [178,  34,  34]),
    ("floralwhite",          [255, 250, 240]),
    ("forestgreen",          [ 34, 139,  34]),
    ("fuchsia",              [255,   0, 255]),
    ("gainsboro",            [220, 220, 220]),
    ("ghostwhite",           [248, 248, 255]),
    ("gold",                 [255, 215,   0]),
    ("goldenrod",            [218, 165,  32]),
    ("gray",                 [128, 128, 128]),
    ("green",                [  0, 128,   0]),
    ("greenyellow",          [173, 255,  47]),
    ("grey",                 [128, 128, 128]),
    ("honeydew",             [240, 255, 240]),
    ("hotpink",              [255, 105, 180]),
    ("indianred",            [205,  92,  92]),
    ("indigo",               [ 75,   0, 130]),
    ("ivory",                [255, 255, 240]),
    ("khaki",                [240, 230, 140]),
    ("lavender",             [230, 230, 250]),
    ("lavenderblush",        [255, 240, 245]),
    ("lawngreen",            [124, 252,   0]),
    ("lemonchiffon",         [255, 250, 205]),
    ("lightblue",            [173, 216, 230]),
    ("lightcoral",           [240, 128, 128]),
    ("lightcyan",            [224, 255, 255]),
    ("lightgoldenrodyellow", [250, 250, 210]),
    ("lightgray",            [211, 211, 211]),
    ("lightgreen",           [144, 238, 144]),
    ("lightgrey",            [211, 211, 211]),
    ("lightpink",            [255, 182, 193]),
    ("lightsalmon",          [255, 160, 122]),
    ("lightseagreen",        [ 32, 178, 170]),
    ("lightskyblue",         [135, 206, 250]),
    ("lightslategray",       [119, 136, 153]),
    ("lightslategrey",       [119, 136, 153]),
    ("lightsteelblue",       [176, 196, 222]),
    ("lightyellow",          [255, 255, 224]),
    ("lime",                 [  0, 255,   0]),
    ("limegreen",            [ 50, 205,  50]),
    ("linen",                [250, 240, 230]),
    ("magenta",              [255,   0, 255]),
    ("maroon",               [128,   0,   0]),
    ("mediumaquamarine",     [102, 205, 170]),
    ("mediumblue",           [  0,   0, 205]),
    ("mediumorchid",         [186,  85, 211]),
    ("mediumpurple",         [147, 112, 219]),
    ("mediumseagreen",       [ 60, 179, 113]),
    ("mediumslateblue",      [123, 104, 238]),
    ("mediumspringgreen",    [  0, 250, 154]),
    ("mediumturquoise",      [ 72, 209, 204]),
    ("mediumvioletred",      [199,  21, 133]),
    ("midnightblue",         [ 25,  25, 112]),
    ("mintcream",            [245, 255, 250]),
    ("mistyrose",            [255, 228, 225]),
    ("moccasin",             [255, 228, 181]),
    ("navajowhite",          [255, 222, 173]),
    ("navy",                 [  0,   0, 128]),
    ("oldlace",              [253, 245, 230]),
    ("olive",                [128, 128,   0]),
    ("olivedrab",            [107, 142,  35]),
    ("orange",               [255, 165,   0]),
    ("orangered",            [255,  69,   0]),
    ("orchid",               [218, 112, 214]),
    ("palegoldenrod",        [238, 232, 170]),
    ("palegreen",            [152, 251, 152]),
    ("paleturquoise",        [175, 238, 238]),
    ("palevioletred",        [219, 112, 147]),
    ("papayawhip",           [255, 239, 213]),
    ("peachpuff",            [255, 218, 185]),
    ("peru",                 [205, 133,  63]),
    ("pink",                 [255, 192, 203]),
    ("plum",                 [221, 160, 221]),
    ("powderblue",           [176, 224, 230]),
    ("purple",               [128,   0, 128]),
    ("rebeccapurple",        [102,  51, 153]),
    ("red",                  [255,   0,   0]),
    ("rosybrown",            [188, 143, 143]),
    ("royalblue",            [ 65, 105, 225]),
    ("saddlebrown",          [139,  69,  19]),
    ("salmon",               [250, 128, 114]),
    ("sandybrown",           [244, 164,  96]),
    ("seagreen",             [ 46, 139,  87]),
    ("seashell",             [255, 245, 238]),
    ("sienna",               [160,  82,  45]),
    ("silver",               [192, 192, 192]),
    ("skyblue",              [135, 206, 235]),
    ("slateblue",            [106,  90, 205]),
    ("slategray",            [112, 128, 144]),
    ("slategrey",            [112, 128, 144]),
    ("snow",                 [255, 250, 250]),
    ("springgreen",          [  0, 255, 127]),
    ("steelblue",            [ 70, 130, 180]),
    ("tan",                  [210, 180, 140]),
    ("teal",                 [  0, 128, 128]),
    ("thistle",              [216, 191, 216]),
    ("tomato",               [255,  99,  71]),
    ("turquoise",            [ 64, 224, 208]),
    ("violet",               [238, 130, 238]),
    ("wheat",                [245, 222, 179]),
    ("white",                [255, 255, 255]),
    ("whitesmoke",           [245, 245, 245]),
    ("yellow",               [255, 255,   0]),
    ("yellowgreen",          [154, 205,  50]),
];

/// Length unit (T3 Domain Type).
#[derive(Debug, Clone, Copy, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Em(f32),
    Percent(f32),
}

impl Length {
    /// Convert to pixels given a context size.
    #[must_use]
    pub fn to_px(self, parent_px: f32, font_size_px: f32) -> f32 {
        match self {
            Self::Auto => 0.0,
            Self::Px(px) => px,
            Self::Em(em) => em * font_size_px,
            Self::Percent(p) => parent_px * p / 100.0,
        }
    }
}

/// Text alignment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Text decoration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
}

/// Overflow handling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// CSS position property.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
}

/// Flex direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

/// Flex wrap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Justify content (main axis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Align items (cross axis).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

/// List style type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ListStyleType {
    #[default]
    None,
    Disc,
    Circle,
    Square,
    Decimal,
}

/// Computed style values for a node.
#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_family: String,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    pub line_height: f32,
    pub margin: EdgeSizes,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub border_color: Color,
    pub border_radius: f32,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub max_width: Length,
    pub min_height: Length,
    pub max_height: Length,
    pub overflow: Overflow,
    pub opacity: f32,
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub gap: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub list_style_type: ListStyleType,
}

/// Font weight.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

/// Display property.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Block,
    Inline,
    None,
    Flex,
    /// `<table>`, `<thead>`, `<tbody>`, `<tfoot>` — block container for rows.
    Table,
    /// `<tr>` — flex row of cells.
    TableRow,
    /// `<td>`, `<th>` — flex item within a row.
    TableCell,
}

/// Edge sizes for margin/padding/border.
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Styled DOM node (T2-P composite).
///
/// Uses `NodeId` for zero-cost reference into the Arena (no cloning).
#[derive(Debug, Clone)]
pub struct StyledNode {
    /// Arena node ID (Copy — eliminates the old `node.clone()` hot path).
    pub node_id: NodeId,
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
}

impl StyledNode {
    /// Apply styles to an arena-based DOM tree using full cascade resolution.
    ///
    /// Merges `external_rules` (from `<link rel="stylesheet">`) with inline
    /// `<style>` rules extracted from the DOM.  External rules come first so
    /// inline `<style>` rules win at equal specificity (source order).
    #[must_use]
    pub fn from_arena(arena: &Arena, external_rules: &[CssRule]) -> Self {
        let mut css_rules = external_rules.to_vec();
        css_rules.extend(parse::extract_stylesheets_arena(arena));
        let resolver = CascadeResolver::new(&css_rules);
        Self::build_styled_tree(arena.root(), arena, &resolver, None)
    }

    /// Recursively build the styled tree using the arena + cascade resolver.
    fn build_styled_tree(
        id: NodeId,
        arena: &Arena,
        resolver: &CascadeResolver<'_>,
        parent_style: Option<&ComputedStyle>,
    ) -> Self {
        let inline_css = arena
            .attrs(id)
            .and_then(|attrs| attrs.get("style").map(String::as_str));

        let style = resolver.resolve_arena(id, arena, parent_style, inline_css);

        let children = arena
            .children(id)
            .iter()
            .map(|&child_id| Self::build_styled_tree(child_id, arena, resolver, Some(&style)))
            .collect();

        Self {
            node_id: id,
            style,
            children,
        }
    }
}

/// A CSS stylesheet (simplified wrapper for backward compatibility).
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    /// Parsed rules (now delegates to `cascade::CssRule` internally).
    pub rules: Vec<Rule>,
}

/// A CSS rule (legacy type — cascade module uses `CssRule`).
#[derive(Debug, Clone)]
pub struct Rule {
    /// CSS selector string.
    pub selector: String,
    /// Property → value declarations.
    pub declarations: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Color tests ---

    #[test]
    fn color_parse_named_colors() {
        let white = Color::parse("white");
        assert!(white.is_some());
        let w = white.unwrap_or_default();
        assert_eq!((w.r, w.g, w.b, w.a), (255, 255, 255, 255));

        let black = Color::parse("black");
        assert!(black.is_some());
        let b = black.unwrap_or_default();
        assert_eq!((b.r, b.g, b.b, b.a), (0, 0, 0, 255));

        let red = Color::parse("red");
        assert!(red.is_some());
        let r = red.unwrap_or_default();
        assert_eq!((r.r, r.g, r.b), (255, 0, 0));
    }

    #[test]
    fn color_parse_hex_6_digit() {
        let c = Color::parse("#ff8800");
        assert!(c.is_some());
        let c = c.unwrap_or_default();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 136, 0, 255));
    }

    #[test]
    fn color_parse_hex_3_digit() {
        let c = Color::parse("#f00");
        assert!(c.is_some());
        let c = c.unwrap_or_default();
        // #f00 => r=0xf*17=255, g=0, b=0
        assert_eq!((c.r, c.g, c.b), (255, 0, 0));
    }

    #[test]
    fn color_parse_transparent() {
        let c = Color::parse("transparent");
        assert!(c.is_some());
        let c = c.unwrap_or_default();
        assert_eq!(c.a, 0);
    }

    #[test]
    fn color_parse_invalid_returns_none() {
        assert!(Color::parse("not-a-color").is_none());
        assert!(Color::parse("#xyz").is_none());
        assert!(Color::parse("#12345").is_none()); // 5 digits invalid
    }

    #[test]
    fn color_parse_trims_whitespace() {
        let c = Color::parse("  white  ");
        assert!(c.is_some());
    }

    // --- Length tests ---

    #[test]
    fn length_auto_resolves_to_zero() {
        let l = Length::Auto;
        let result = l.to_px(100.0, 16.0);
        assert!((result - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn length_px_passes_through() {
        let l = Length::Px(42.0);
        let result = l.to_px(100.0, 16.0);
        assert!((result - 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn length_em_multiplies_font_size() {
        let l = Length::Em(2.0);
        let result = l.to_px(100.0, 16.0);
        assert!((result - 32.0).abs() < f32::EPSILON);
    }

    #[test]
    fn length_percent_of_parent() {
        let l = Length::Percent(50.0);
        let result = l.to_px(200.0, 16.0);
        assert!((result - 100.0).abs() < f32::EPSILON);
    }

    // --- Display tests ---

    #[test]
    fn display_default_is_block() {
        let d = Display::default();
        assert_eq!(d, Display::Block);
    }

    // --- EdgeSizes tests ---

    #[test]
    fn edge_sizes_default_is_zero() {
        let e = EdgeSizes::default();
        assert!((e.top).abs() < f32::EPSILON);
        assert!((e.right).abs() < f32::EPSILON);
        assert!((e.bottom).abs() < f32::EPSILON);
        assert!((e.left).abs() < f32::EPSILON);
    }

    // --- ComputedStyle tests ---

    #[test]
    fn computed_style_default_has_block_display() {
        let s = ComputedStyle::default();
        assert_eq!(s.display, Display::Block);
    }

    // --- Extended color tests ---

    #[test]
    fn color_parse_all_148_named_colors_exist() {
        // Spot-check across the alphabet
        let cases = [
            ("aliceblue", 240, 248, 255),
            ("coral", 255, 127, 80),
            ("darkslategray", 47, 79, 79),
            ("gold", 255, 215, 0),
            ("indigo", 75, 0, 130),
            ("limegreen", 50, 205, 50),
            ("navy", 0, 0, 128),
            ("orchid", 218, 112, 214),
            ("rebeccapurple", 102, 51, 153),
            ("steelblue", 70, 130, 180),
            ("tomato", 255, 99, 71),
            ("yellowgreen", 154, 205, 50),
        ];
        for (name, r, g, b) in cases {
            let c = Color::parse(name).unwrap_or_default();
            assert_eq!((c.r, c.g, c.b), (r, g, b), "Failed for {name}");
        }
    }

    #[test]
    fn color_parse_case_insensitive() {
        let c1 = Color::parse("DarkCyan").unwrap_or_default();
        let c2 = Color::parse("darkcyan").unwrap_or_default();
        assert_eq!((c1.r, c1.g, c1.b), (c2.r, c2.g, c2.b));
    }

    #[test]
    fn color_parse_grey_alias() {
        let gray = Color::parse("gray").unwrap_or_default();
        let grey = Color::parse("grey").unwrap_or_default();
        assert_eq!((gray.r, gray.g, gray.b), (grey.r, grey.g, grey.b));
    }

    #[test]
    fn color_parse_rgb_function() {
        let c = Color::parse("rgb(100, 200, 50)").unwrap_or_default();
        assert_eq!((c.r, c.g, c.b, c.a), (100, 200, 50, 255));
    }

    #[test]
    fn color_parse_rgba_function() {
        let c = Color::parse("rgba(255, 0, 128, 0.5)").unwrap_or_default();
        assert_eq!((c.r, c.g, c.b), (255, 0, 128));
        // 0.5 * 255 = 127
        assert!(c.a >= 127 && c.a <= 128);
    }

    #[test]
    fn color_parse_rgba_full_opaque() {
        let c = Color::parse("rgba(10, 20, 30, 1.0)").unwrap_or_default();
        assert_eq!((c.r, c.g, c.b, c.a), (10, 20, 30, 255));
    }

    #[test]
    fn color_parse_rgba_transparent() {
        let c = Color::parse("rgba(10, 20, 30, 0)").unwrap_or_default();
        assert_eq!(c.a, 0);
    }

    #[test]
    fn color_parse_hex_8_digit_with_alpha() {
        let c = Color::parse("#ff000080").unwrap_or_default();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 0, 0, 128));
    }

    #[test]
    fn color_parse_rgb_invalid_returns_default() {
        // Wrong number of args
        assert!(Color::parse("rgb(1, 2)").is_none());
        assert!(Color::parse("rgb(1, 2, 3, 4, 5)").is_none());
    }

    #[test]
    fn color_named_table_is_sorted() {
        // Verify binary search precondition
        for window in NAMED_COLORS.windows(2) {
            assert!(
                window[0].0 < window[1].0,
                "NAMED_COLORS not sorted: {:?} >= {:?}",
                window[0].0,
                window[1].0
            );
        }
    }
}
