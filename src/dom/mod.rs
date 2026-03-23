//! DOM tree representation.
//!
//! Provides two APIs:
//! - **Arena API** (preferred): `Arena`, `NodeId`, `NodeKind`, `DomNode` — flat allocation, O(1) access
//! - **Legacy Node enum** (deprecated): recursive `Node` enum — will be removed

use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::collections::BTreeMap;

// ── Arena types ──────────────────────────────────────────────────

/// Index into the arena (T1 primitive: sequence position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub usize);

/// Data payload of a DOM node — no children, no relationships.
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Document root.
    Document,
    /// Element with tag name and attributes.
    Element {
        tag: String,
        attrs: BTreeMap<String, String>,
    },
    /// Text content.
    Text(String),
    /// Comment (ignored in rendering).
    Comment(String),
}

/// A single node in the arena with parent/child relationships (T2-P).
#[derive(Debug, Clone)]
pub struct DomNode {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

/// Flat arena holding all DOM nodes (T2-C composite).
#[derive(Debug, Clone, Default)]
pub struct Arena {
    nodes: Vec<DomNode>,
}

impl Arena {
    /// Parse HTML string into an arena-based DOM.
    #[must_use]
    pub fn parse(html: &str) -> Self {
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut html.as_bytes())
            .unwrap_or_else(|_| RcDom::default());

        let mut arena = Self::default();
        arena.alloc_from_handle(&dom.document, None);
        arena
    }

    /// Allocate a node with given kind and parent. Returns its `NodeId`.
    pub fn alloc(&mut self, kind: NodeKind, parent: Option<NodeId>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(DomNode {
            kind,
            parent,
            children: Vec::new(),
        });
        if let Some(pid) = parent
            && let Some(p) = self.nodes.get_mut(pid.0)
        {
            p.children.push(id);
        }
        id
    }

    /// Get a node by ID.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&DomNode> {
        self.nodes.get(id.0)
    }

    /// Child IDs of a node.
    #[must_use]
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes.get(id.0).map_or(&[], |n| &n.children)
    }

    /// Parent ID of a node.
    #[must_use]
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id.0).and_then(|n| n.parent)
    }

    /// Tag name shortcut (None for non-elements).
    #[must_use]
    pub fn tag(&self, id: NodeId) -> Option<&str> {
        match self.nodes.get(id.0).map(|n| &n.kind) {
            Some(NodeKind::Element { tag, .. }) => Some(tag),
            _ => None,
        }
    }

    /// Attributes shortcut (None for non-elements).
    #[must_use]
    pub fn attrs(&self, id: NodeId) -> Option<&BTreeMap<String, String>> {
        match self.nodes.get(id.0).map(|n| &n.kind) {
            Some(NodeKind::Element { attrs, .. }) => Some(attrs),
            _ => None,
        }
    }

    /// Text content shortcut (None for non-text nodes).
    #[must_use]
    pub fn text(&self, id: NodeId) -> Option<&str> {
        match self.nodes.get(id.0).map(|n| &n.kind) {
            Some(NodeKind::Text(t)) => Some(t),
            _ => None,
        }
    }

    /// The node kind reference.
    #[must_use]
    pub fn kind(&self, id: NodeId) -> Option<&NodeKind> {
        self.nodes.get(id.0).map(|n| &n.kind)
    }

    /// Iterator walking ancestors from `id` toward root.
    #[must_use]
    pub fn ancestors(&self, id: NodeId) -> AncestorIter<'_> {
        AncestorIter {
            arena: self,
            current: Some(id),
        }
    }

    /// Root node (always `NodeId(0)` after parse).
    #[must_use]
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Number of nodes in the arena.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the arena is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Find first node with given tag name (DFS).
    #[must_use]
    pub fn find_tag(&self, start: NodeId, tag: &str) -> Option<NodeId> {
        if self.tag(start) == Some(tag) {
            return Some(start);
        }
        for &child in self.children(start) {
            if let Some(found) = self.find_tag(child, tag) {
                return Some(found);
            }
        }
        None
    }

    /// Find first non-empty text node (DFS).
    #[must_use]
    pub fn find_text(&self, start: NodeId) -> Option<NodeId> {
        if let Some(t) = self.text(start)
            && !t.trim().is_empty()
        {
            return Some(start);
        }
        for &child in self.children(start) {
            if let Some(found) = self.find_text(child) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively populate arena from html5ever handle.
    fn alloc_from_handle(&mut self, handle: &Handle, parent: Option<NodeId>) -> NodeId {
        let kind = match &handle.data {
            NodeData::Document => NodeKind::Document,
            NodeData::Element { name, attrs, .. } => {
                let tag = name.local.to_string();
                let attrs = attrs
                    .borrow()
                    .iter()
                    .map(|a| (a.name.local.to_string(), a.value.to_string()))
                    .collect();
                NodeKind::Element { tag, attrs }
            }
            NodeData::Text { contents } => NodeKind::Text(contents.borrow().to_string()),
            NodeData::Comment { contents } => NodeKind::Comment(contents.to_string()),
            _ => NodeKind::Text(String::new()),
        };

        let id = self.alloc(kind, parent);

        for child in handle.children.borrow().iter() {
            self.alloc_from_handle(child, Some(id));
        }

        id
    }
}

/// Iterator over ancestor `NodeId`s (excludes starting node).
pub struct AncestorIter<'a> {
    arena: &'a Arena,
    current: Option<NodeId>,
}

impl Iterator for AncestorIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let cur = self.current?;
        let parent = self.arena.parent(cur);
        self.current = parent;
        parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_parse_simple_html() {
        let arena = Arena::parse("<html><body><h1>Hello</h1></body></html>");
        assert!(!arena.is_empty());
        assert!(matches!(arena.kind(arena.root()), Some(NodeKind::Document)));
    }

    #[test]
    fn arena_parent_child_relationships() {
        let arena = Arena::parse("<html><body><p>text</p></body></html>");
        let root = arena.root();
        assert!(arena.parent(root).is_none());
        assert!(!arena.children(root).is_empty());
    }

    #[test]
    fn arena_tag_and_attrs() {
        let arena = Arena::parse(r#"<a href="https://example.com">Link</a>"#);
        let a_id = arena.find_tag(arena.root(), "a");
        assert!(a_id.is_some(), "should find <a> tag");
        let a_id = a_id.unwrap();
        assert_eq!(arena.tag(a_id), Some("a"));
        let attrs = arena.attrs(a_id).unwrap();
        assert_eq!(
            attrs.get("href").map(String::as_str),
            Some("https://example.com")
        );
    }

    #[test]
    fn arena_text_content() {
        let arena = Arena::parse("<p>Hello World</p>");
        let text_id = arena.find_text(arena.root());
        assert!(text_id.is_some(), "should find text node");
        let text = arena.text(text_id.unwrap()).unwrap();
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn arena_ancestors_walk() {
        let arena = Arena::parse("<html><body><p>deep</p></body></html>");
        let p_id = arena.find_tag(arena.root(), "p");
        assert!(p_id.is_some(), "should find <p> tag");
        let ancestor_tags: Vec<Option<&str>> = arena
            .ancestors(p_id.unwrap())
            .map(|id| arena.tag(id))
            .collect();
        assert!(ancestor_tags.contains(&Some("body")));
        assert!(ancestor_tags.contains(&Some("html")));
    }

    #[test]
    fn arena_alloc_manual() {
        let mut arena = Arena::default();
        let root = arena.alloc(NodeKind::Document, None);
        let div = arena.alloc(
            NodeKind::Element {
                tag: "div".to_string(),
                attrs: BTreeMap::new(),
            },
            Some(root),
        );
        let text = arena.alloc(NodeKind::Text("hello".to_string()), Some(div));
        assert_eq!(arena.len(), 3);
        assert_eq!(arena.children(root), &[div]);
        assert_eq!(arena.children(div), &[text]);
        assert_eq!(arena.parent(text), Some(div));
        assert_eq!(arena.text(text), Some("hello"));
    }
}
