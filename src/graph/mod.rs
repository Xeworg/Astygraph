//! UI-independent graph model boundary.
//!
//! This module defines the minimal types required to accept future graph data
//! without coupling the model to egui, tree-sitter, providers, parsers, or the
//! filesystem. It is exercised headlessly and is the only surface a future
//! rendering or analysis layer may depend on for node/edge/document storage.
//!
//! Scope (intentionally narrow for the desktop-basics slice):
//!
//! - Three opaque id newtypes: [`NodeId`](crate::graph::NodeId),
//!   [`EdgeId`](crate::graph::EdgeId), [`DocumentId`](crate::graph::DocumentId).
//! - Two closed enums for kinds: [`NodeKind`](crate::graph::NodeKind),
//!   [`EdgeKind`](crate::graph::EdgeKind).
//! - Three value types: [`Node`](crate::graph::Node),
//!   [`Edge`](crate::graph::Edge), [`Document`](crate::graph::Document).
//! - One container: [`Graph`](crate::graph::Graph) enforcing the invariants listed below.
//!
//! Invariants enforced by [`Graph`](crate::graph::Graph):
//!
//! - Ids are unique within a graph (assigned monotonically on insert).
//! - An edge's source and target must reference existing nodes.
//! - Self-loops are rejected.
//! - A node attached to a document must reference an existing document.
//! - A document path must be non-empty.
//!
//! Out of scope for this slice (deliberate non-goals): any rendering,
//! persistence, parser, AI/provider, or graph-traversal algorithm.
//! No egui, tree-sitter, or `eframe` symbols may appear in this module.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Unique identifier for a node within a single [`Graph`].
///
/// Two nodes in different graphs may share the same numeric value; ids are
/// only meaningful relative to the graph that minted them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// Unique identifier for an edge within a single [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u64);

/// Unique identifier for a document within a single [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(pub u64);

/// Closed set of node kinds the MVP model can represent.
///
/// New variants may be added later as future analysis requirements become
/// concrete; consumers must handle all variants explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// A source file in the project.
    File,
    /// A named syntactic or semantic unit within a file (function, class, etc.).
    Symbol,
    /// A logical grouping that contains files or symbols (module, namespace,
    /// crate, package — language-dependent).
    Module,
}

/// Closed set of edge kinds the MVP model can represent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Parent contains child (file -> symbol, module -> file).
    Contains,
    /// Source references target (symbol -> symbol, file -> file).
    References,
    /// Source depends on target (file -> file, module -> module).
    DependsOn,
}

/// A node in the graph.
///
/// `document` is `Some` when the node was created via
/// [`Graph::add_node_in_document`]; otherwise the node is unattached.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub label: String,
    pub document: Option<DocumentId>,
}

/// A directed edge between two existing nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub kind: EdgeKind,
}

/// A document represents the source file or logical unit from which nodes
/// were derived. Documents carry the path so future analysis layers can
/// locate the original source without re-discovering it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Document {
    pub id: DocumentId,
    pub path: PathBuf,
}

/// Failure cases for graph construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// The referenced node does not exist in the graph.
    UnknownNode(NodeId),
    /// The referenced document does not exist in the graph.
    UnknownDocument(DocumentId),
    /// An edge was requested with identical source and target ids.
    SelfLoop { node: NodeId },
    /// A document was created with an empty path.
    EmptyDocumentPath,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::UnknownNode(id) => {
                write!(f, "unknown node id {} in graph", id.0)
            }
            GraphError::UnknownDocument(id) => {
                write!(f, "unknown document id {} in graph", id.0)
            }
            GraphError::SelfLoop { node } => {
                write!(f, "self-loop edge rejected on node {}", node.0)
            }
            GraphError::EmptyDocumentPath => {
                write!(f, "document path must not be empty")
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// In-memory container for nodes, edges, and documents with invariant checks.
///
/// The container is intentionally not `Clone`-able as a single value (the
/// internal maps would be expensive to clone), but the value types it owns
/// are `Clone` so callers may copy individual items if needed.
#[derive(Debug, Default)]
pub struct Graph {
    nodes: HashMap<NodeId, Node>,
    edges: HashMap<EdgeId, Edge>,
    documents: HashMap<DocumentId, Document>,
    next_node_id: u64,
    next_edge_id: u64,
    next_document_id: u64,
}

impl Graph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a document and return its id. The path must be non-empty.
    pub fn add_document(&mut self, path: PathBuf) -> Result<DocumentId, GraphError> {
        if path.as_os_str().is_empty() {
            return Err(GraphError::EmptyDocumentPath);
        }
        let id = DocumentId(self.next_document_id);
        self.next_document_id += 1;
        self.documents.insert(id, Document { id, path });
        Ok(id)
    }

    /// Add an unattached node and return its id.
    ///
    /// This method cannot fail: the only invariants for an unattached node
    /// are uniqueness (enforced by the id allocator) and storage, both of
    /// which are guaranteed by construction.
    pub fn add_node(&mut self, kind: NodeKind, label: impl Into<String>) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        let node = Node {
            id,
            kind,
            label: label.into(),
            document: None,
        };
        self.nodes.insert(id, node);
        id
    }

    /// Add a node attached to an existing document and return its id.
    ///
    /// Returns [`GraphError::UnknownDocument`] if `document` was not previously
    /// registered via [`Graph::add_document`].
    pub fn add_node_in_document(
        &mut self,
        kind: NodeKind,
        label: impl Into<String>,
        document: DocumentId,
    ) -> Result<NodeId, GraphError> {
        if !self.documents.contains_key(&document) {
            return Err(GraphError::UnknownDocument(document));
        }
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        let node = Node {
            id,
            kind,
            label: label.into(),
            document: Some(document),
        };
        self.nodes.insert(id, node);
        Ok(id)
    }

    /// Add a directed edge between two existing nodes and return its id.
    ///
    /// Returns:
    /// - [`GraphError::SelfLoop`] if `source == target`.
    /// - [`GraphError::UnknownNode`] if either endpoint does not exist.
    pub fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        kind: EdgeKind,
    ) -> Result<EdgeId, GraphError> {
        if source == target {
            return Err(GraphError::SelfLoop { node: source });
        }
        if !self.nodes.contains_key(&source) {
            return Err(GraphError::UnknownNode(source));
        }
        if !self.nodes.contains_key(&target) {
            return Err(GraphError::UnknownNode(target));
        }
        let id = EdgeId(self.next_edge_id);
        self.next_edge_id += 1;
        let edge = Edge {
            id,
            source,
            target,
            kind,
        };
        self.edges.insert(id, edge);
        Ok(id)
    }

    /// Look up a node by id.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Look up an edge by id.
    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    /// Look up a document by id.
    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    /// Iterate over all nodes in unspecified order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Iterate over all edges in unspecified order.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values()
    }

    /// Iterate over all documents in unspecified order.
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    /// Iterate over edges whose `source` equals `id`.
    pub fn outgoing(&self, id: NodeId) -> impl Iterator<Item = &Edge> {
        self.edges.values().filter(move |e| e.source == id)
    }

    /// Iterate over edges whose `target` equals `id`.
    pub fn incoming(&self, id: NodeId) -> impl Iterator<Item = &Edge> {
        self.edges.values().filter(move |e| e.target == id)
    }

    /// Number of nodes currently stored.
    pub fn len_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges currently stored.
    pub fn len_edges(&self) -> usize {
        self.edges.len()
    }

    /// Number of documents currently stored.
    pub fn len_documents(&self) -> usize {
        self.documents.len()
    }

    /// True when the graph contains no nodes, edges, or documents.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty() && self.documents.is_empty()
    }
}
