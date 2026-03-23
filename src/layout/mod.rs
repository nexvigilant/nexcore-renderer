//! Layout engine using taffy for flexbox/grid.

use crate::dom::{Arena, NodeKind};
use crate::style::{
    AlignItems, ComputedStyle, Display, FlexDirection, FlexWrap, JustifyContent, Overflow,
    StyledNode,
};
use taffy::prelude::*;

/// A rectangle (T3 Domain Type).
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Check if point is inside rectangle.
    #[must_use]
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// A positioned box in the layout tree (T2-P).
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub rect: Rect,
    pub style: ComputedStyle,
    pub content: BoxContent,
    pub children: Vec<LayoutBox>,
}

/// Content type of a layout box.
#[derive(Debug, Clone)]
pub enum BoxContent {
    Block,
    Inline,
    Text(String),
    Image { src: String },
}

/// Layout tree builder.
pub struct LayoutEngine {
    taffy: TaffyTree,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    /// Create a new layout engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    /// Build layout tree from styled nodes, using arena for content extraction.
    pub fn layout(&mut self, root: &StyledNode, viewport: Rect, arena: &Arena) -> LayoutBox {
        self.taffy.clear();
        let root_node = self.build_taffy_tree(root, arena);
        let available = Size {
            width: AvailableSpace::Definite(viewport.width),
            height: AvailableSpace::Definite(viewport.height),
        };
        if let Err(e) = self.taffy.compute_layout(root_node, available) {
            tracing::warn!("Layout computation failed: {e:?}");
        }
        self.extract_layout(root_node, root, 0.0, 0.0, arena)
    }

    fn build_taffy_tree(&mut self, styled: &StyledNode, arena: &Arena) -> NodeId {
        let mut style = self.convert_style(&styled.style);

        // Text nodes need intrinsic size — taffy gives 0×0 to leaves without it
        if let Some(NodeKind::Text(text)) = arena.kind(styled.node_id) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let font_size = styled.style.font_size;
                let line_height = styled.style.line_height;
                // Approximate: char width ≈ font_size * 0.6 (matches paint module)
                let text_width = trimmed.len() as f32 * font_size * 0.6;
                let text_height = font_size * line_height;
                style.min_size = Size {
                    width: Dimension::Length(text_width),
                    height: Dimension::Length(text_height),
                };
            } else {
                // Whitespace-only text nodes: zero size
                style.size = Size {
                    width: Dimension::Length(0.0),
                    height: Dimension::Length(0.0),
                };
            }
        }

        let children: Vec<NodeId> = styled
            .children
            .iter()
            .filter(|c| c.style.display != Display::None)
            .map(|c| self.build_taffy_tree(c, arena))
            .collect();
        self.taffy
            .new_with_children(style, &children)
            .unwrap_or_else(|_| {
                self.taffy
                    .new_leaf(Style::default())
                    .unwrap_or(taffy::NodeId::new(0))
            })
    }

    fn convert_style(&self, computed: &ComputedStyle) -> Style {
        let (display, extra) = match computed.display {
            Display::Block => (taffy::Display::Block, StyleExtra::default()),
            Display::Flex => (taffy::Display::Flex, StyleExtra::default()),
            Display::Inline | Display::None => (taffy::Display::Block, StyleExtra::default()),
            Display::Table => (
                taffy::Display::Block,
                StyleExtra {
                    size_width: Some(Dimension::Percent(1.0)),
                    ..Default::default()
                },
            ),
            Display::TableRow => (taffy::Display::Flex, StyleExtra::default()),
            Display::TableCell => (
                taffy::Display::Block,
                StyleExtra {
                    flex_grow: Some(1.0),
                    flex_basis: Some(Dimension::Length(0.0)),
                    ..Default::default()
                },
            ),
        };

        let width = extra
            .size_width
            .unwrap_or_else(|| self.convert_dimension(computed.width));

        let flex_direction = match computed.flex_direction {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        };

        let flex_wrap = match computed.flex_wrap {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
        };

        let justify_content = match computed.justify_content {
            JustifyContent::FlexStart => Some(taffy::JustifyContent::FlexStart),
            JustifyContent::FlexEnd => Some(taffy::JustifyContent::FlexEnd),
            JustifyContent::Center => Some(taffy::JustifyContent::Center),
            JustifyContent::SpaceBetween => Some(taffy::JustifyContent::SpaceBetween),
            JustifyContent::SpaceAround => Some(taffy::JustifyContent::SpaceAround),
            JustifyContent::SpaceEvenly => Some(taffy::JustifyContent::SpaceEvenly),
        };

        let align_items = match computed.align_items {
            AlignItems::Stretch => Some(taffy::AlignItems::Stretch),
            AlignItems::FlexStart => Some(taffy::AlignItems::FlexStart),
            AlignItems::FlexEnd => Some(taffy::AlignItems::FlexEnd),
            AlignItems::Center => Some(taffy::AlignItems::Center),
            AlignItems::Baseline => Some(taffy::AlignItems::Baseline),
        };

        let overflow_val = match computed.overflow {
            Overflow::Visible => taffy::Overflow::Visible,
            Overflow::Hidden => taffy::Overflow::Hidden,
            Overflow::Scroll => taffy::Overflow::Scroll,
            Overflow::Auto => taffy::Overflow::Scroll,
        };

        let gap_val = LengthPercentage::Length(computed.gap);

        Style {
            display,
            size: Size {
                width,
                height: self.convert_dimension(computed.height),
            },
            min_size: Size {
                width: self.convert_dimension(computed.min_width),
                height: self.convert_dimension(computed.min_height),
            },
            max_size: Size {
                width: self.convert_dimension(computed.max_width),
                height: self.convert_dimension(computed.max_height),
            },
            margin: taffy::Rect {
                top: LengthPercentageAuto::Length(computed.margin.top),
                right: LengthPercentageAuto::Length(computed.margin.right),
                bottom: LengthPercentageAuto::Length(computed.margin.bottom),
                left: LengthPercentageAuto::Length(computed.margin.left),
            },
            padding: taffy::Rect {
                top: LengthPercentage::Length(computed.padding.top),
                right: LengthPercentage::Length(computed.padding.right),
                bottom: LengthPercentage::Length(computed.padding.bottom),
                left: LengthPercentage::Length(computed.padding.left),
            },
            border: taffy::Rect {
                top: LengthPercentage::Length(computed.border.top),
                right: LengthPercentage::Length(computed.border.right),
                bottom: LengthPercentage::Length(computed.border.bottom),
                left: LengthPercentage::Length(computed.border.left),
            },
            flex_direction,
            flex_wrap,
            justify_content,
            align_items,
            gap: Size {
                width: gap_val,
                height: gap_val,
            },
            flex_grow: extra.flex_grow.unwrap_or(computed.flex_grow),
            flex_shrink: computed.flex_shrink,
            flex_basis: extra.flex_basis.unwrap_or(Dimension::Auto),
            overflow: taffy::Point {
                x: overflow_val,
                y: overflow_val,
            },
            ..Default::default()
        }
    }

    fn convert_dimension(&self, len: crate::style::Length) -> Dimension {
        match len {
            crate::style::Length::Auto => Dimension::Auto,
            crate::style::Length::Px(px) => Dimension::Length(px),
            crate::style::Length::Percent(p) => Dimension::Percent(p / 100.0),
            crate::style::Length::Em(em) => Dimension::Length(em * 16.0),
        }
    }

    fn extract_layout(
        &self,
        node: NodeId,
        styled: &StyledNode,
        ox: f32,
        oy: f32,
        arena: &Arena,
    ) -> LayoutBox {
        let layout = self.taffy.layout(node).copied().unwrap_or_default();
        let rect = Rect {
            x: ox + layout.location.x,
            y: oy + layout.location.y,
            width: layout.size.width,
            height: layout.size.height,
        };
        let content = match arena.kind(styled.node_id) {
            Some(NodeKind::Text(t)) => BoxContent::Text(t.clone()),
            Some(NodeKind::Element { tag, attrs }) if tag == "img" => BoxContent::Image {
                src: attrs.get("src").cloned().unwrap_or_default(),
            },
            _ => BoxContent::Block,
        };
        let taffy_children: Vec<NodeId> = self.taffy.children(node).unwrap_or_default();
        let children: Vec<LayoutBox> = taffy_children
            .iter()
            .zip(
                styled
                    .children
                    .iter()
                    .filter(|c| c.style.display != Display::None),
            )
            .map(|(&cn, cs)| self.extract_layout(cn, cs, rect.x, rect.y, arena))
            .collect();
        LayoutBox {
            rect,
            style: styled.style.clone(),
            content,
            children,
        }
    }
}

/// Extra style overrides for table layout mapping.
#[derive(Default)]
struct StyleExtra {
    size_width: Option<Dimension>,
    flex_grow: Option<f32>,
    flex_basis: Option<Dimension>,
}
