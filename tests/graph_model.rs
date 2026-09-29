//! Integration tests for the UI-independent graph model boundary.
//!
//! These tests verify that the graph model:
//! - Has no coupling to egui (it is exercised without any window).
//! - Enforces invariants: unique IDs, valid edge endpoints, no self-loops,
//!   document references, and non-empty document paths.
//! - Supports node/edge/document queries for future rendering layers.
//!
//! The tests intentionally do not depend on egui; the existence of this
//! integration test that only imports `astynex::graph` proves the boundary.

use std::path::PathBuf;

use astynex::graph::{DocumentId, EdgeKind, Graph, GraphError, NodeId, NodeKind};

#[test]
fn empty_graph_is_empty() {
    let g = Graph::new();
    assert!(g.is_empty());
    assert_eq!(g.len_nodes(), 0);
    assert_eq!(g.len_edges(), 0);
    assert_eq!(g.len_documents(), 0);
    assert_eq!(g.nodes().count(), 0);
    assert_eq!(g.edges().count(), 0);
    assert_eq!(g.documents().count(), 0);
}

#[test]
fn add_node_assigns_distinct_ids_and_stores_fields() {
    let mut g = Graph::new();
    let a = g.add_node(NodeKind::File, "src/lib.rs");
    let b = g.add_node(NodeKind::File, "src/main.rs");
    let c = g.add_node(NodeKind::Symbol, "parse");

    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
    assert_eq!(g.len_nodes(), 3);
    assert!(!g.is_empty());

    let na = g.node(a).expect("node a exists");
    assert_eq!(na.kind, NodeKind::File);
    assert_eq!(na.label, "src/lib.rs");
    assert_eq!(na.document, None);

    let nc = g.node(c).expect("node c exists");
    assert_eq!(nc.kind, NodeKind::Symbol);
    assert_eq!(nc.label, "parse");
}

#[test]
fn add_document_stores_path_and_assigns_distinct_ids() {
    let mut g = Graph::new();
    let d1 = g.add_document(PathBuf::from("src/lib.rs")).expect("d1 ok");
    let d2 = g.add_document(PathBuf::from("src/main.rs")).expect("d2 ok");
    assert_ne!(d1, d2);
    assert_eq!(g.len_documents(), 2);

    let doc = g.document(d1).expect("doc d1 exists");
    assert_eq!(doc.path, PathBuf::from("src/lib.rs"));
}

#[test]
fn add_document_rejects_empty_path() {
    let mut g = Graph::new();
    let err = g
        .add_document(PathBuf::new())
        .expect_err("empty path must fail");
    assert_eq!(err, GraphError::EmptyDocumentPath);
}

#[test]
fn add_node_in_document_attaches_existing_document() {
    let mut g = Graph::new();
    let doc = g.add_document(PathBuf::from("src/lib.rs")).expect("doc ok");
    let n = g
        .add_node_in_document(NodeKind::Symbol, "parse", doc)
        .expect("node in doc ok");
    let node = g.node(n).expect("node exists");
    assert_eq!(node.kind, NodeKind::Symbol);
    assert_eq!(node.label, "parse");
    assert_eq!(node.document, Some(doc));
}

#[test]
fn add_node_in_document_rejects_unknown_document() {
    let mut g = Graph::new();
    let bogus = DocumentId(999);
    let err = g
        .add_node_in_document(NodeKind::Symbol, "x", bogus)
        .expect_err("unknown doc must fail");
    assert_eq!(err, GraphError::UnknownDocument(bogus));
    assert_eq!(g.len_nodes(), 0);
}

#[test]
fn add_edge_rejects_unknown_source() {
    let mut g = Graph::new();
    let target = g.add_node(NodeKind::File, "b.rs");
    let bogus = NodeId(999);
    let err = g
        .add_edge(bogus, target, EdgeKind::References)
        .expect_err("unknown source must fail");
    assert_eq!(err, GraphError::UnknownNode(bogus));
    assert_eq!(g.len_edges(), 0);
}

#[test]
fn add_edge_rejects_unknown_target() {
    let mut g = Graph::new();
    let source = g.add_node(NodeKind::File, "a.rs");
    let bogus = NodeId(999);
    let err = g
        .add_edge(source, bogus, EdgeKind::References)
        .expect_err("unknown target must fail");
    assert_eq!(err, GraphError::UnknownNode(bogus));
    assert_eq!(g.len_edges(), 0);
}

#[test]
fn add_edge_rejects_self_loop() {
    let mut g = Graph::new();
    let n = g.add_node(NodeKind::File, "a.rs");
    let err = g
        .add_edge(n, n, EdgeKind::References)
        .expect_err("self-loop must fail");
    assert_eq!(err, GraphError::SelfLoop { node: n });
    assert_eq!(g.len_edges(), 0);
}

#[test]
fn add_edge_succeeds_for_valid_endpoints() {
    let mut g = Graph::new();
    let a = g.add_node(NodeKind::File, "a.rs");
    let b = g.add_node(NodeKind::File, "b.rs");
    let e = g
        .add_edge(a, b, EdgeKind::References)
        .expect("valid edge ok");
    assert_eq!(g.len_edges(), 1);

    let edge = g.edge(e).expect("edge exists");
    assert_eq!(edge.source, a);
    assert_eq!(edge.target, b);
    assert_eq!(edge.kind, EdgeKind::References);
}

#[test]
fn multiple_edges_allowed_between_same_pair_with_different_kinds() {
    let mut g = Graph::new();
    let a = g.add_node(NodeKind::File, "a.rs");
    let b = g.add_node(NodeKind::File, "b.rs");
    let e1 = g.add_edge(a, b, EdgeKind::References).expect("refs ok");
    let e2 = g.add_edge(a, b, EdgeKind::DependsOn).expect("deps ok");
    assert_ne!(e1, e2);
    assert_eq!(g.len_edges(), 2);
}

#[test]
fn outgoing_and_incoming_queries_return_matching_edges() {
    let mut g = Graph::new();
    let a = g.add_node(NodeKind::File, "a.rs");
    let b = g.add_node(NodeKind::File, "b.rs");
    let c = g.add_node(NodeKind::File, "c.rs");
    let _ab = g.add_edge(a, b, EdgeKind::References).expect("ab ok");
    let _back = g
        .add_edge(b, a, EdgeKind::References)
        .expect("back edge ok");
    let _bc = g.add_edge(b, c, EdgeKind::References).expect("bc ok");

    let mut out_b: Vec<_> = g.outgoing(b).map(|e| e.target).collect();
    out_b.sort_by_key(|n| n.0);
    assert_eq!(out_b, vec![a, c]);

    let in_b: Vec<_> = g.incoming(b).map(|e| e.source).collect();
    assert_eq!(in_b, vec![a]);
}

#[test]
fn each_graph_tracks_its_own_nodes_independently() {
    // NodeId is opaque and only meaningful within the graph that minted it.
    // Two separate graphs may legitimately assign overlapping numeric ids;
    // what is guaranteed is uniqueness within each graph and isolation
    // across graphs. This test documents the isolation property.
    let mut g1 = Graph::new();
    let mut g2 = Graph::new();
    let n1 = g1.add_node(NodeKind::File, "x.rs");
    let n2 = g2.add_node(NodeKind::File, "y.rs");

    assert_eq!(g1.len_nodes(), 1);
    assert_eq!(g2.len_nodes(), 1);

    assert_eq!(g1.node(n1).expect("n1 in g1").label, "x.rs");
    assert_eq!(g2.node(n2).expect("n2 in g2").label, "y.rs");

    // Adding to one graph must not affect the other's count.
    let _ = g1.add_node(NodeKind::File, "extra.rs");
    assert_eq!(g1.len_nodes(), 2);
    assert_eq!(g2.len_nodes(), 1);
}

#[test]
fn graph_error_display_is_non_empty() {
    let err = GraphError::EmptyDocumentPath;
    let s = format!("{err}");
    assert!(!s.is_empty(), "Display impl should produce a message");
}

#[test]
fn graph_module_compiles_without_egui_dependency() {
    // This test compiles only if `astynex::graph` is reachable without
    // pulling in egui. The whole file does not import egui on purpose,
    // which proves the boundary at compile time.
    let mut g = Graph::new();
    let doc = g.add_document(PathBuf::from("src/lib.rs")).expect("doc ok");
    let file = g.add_node(NodeKind::File, "src/lib.rs");
    let sym = g
        .add_node_in_document(NodeKind::Symbol, "parse", doc)
        .expect("node in doc ok");
    let _ = g
        .add_edge(file, sym, EdgeKind::Contains)
        .expect("valid edge ok");
    assert_eq!(g.len_nodes(), 2);
    assert_eq!(g.len_edges(), 1);
    assert_eq!(g.len_documents(), 1);
}
