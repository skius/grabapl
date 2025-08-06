use std::collections::HashMap;
use std::fmt::Debug;
use petgraph::dot::{Config, Dot};
use crate::graph::{EdgeAttribute, GraphTrait};
use crate::operation::signature::AbstractSignatureNodeId;
use crate::prelude::OperationSignature;
use crate::Semantics;
use crate::util::bimap::BiMap;

#[derive(Debug)]
enum Output<A> {
    Unchanged,
    MaybeDeleted,
    MaybeWritten(A),
    New(A),
}

pub fn visualize_signature<S: Semantics<NodeAbstract: Debug, EdgeAbstract: Debug>>(sig: &OperationSignature<S>) -> (String, String) {
    let input_graph = sig.parameter.parameter_graph.clone();

    let mut output_graph = input_graph.clone();
    let mut new_nodes_to_key = HashMap::new();
    let mut new_node_key_bimap = BiMap::new();
    let mut node_outputs: HashMap<_, Output<S::NodeAbstract>> = HashMap::new();
    for (key, subst) in sig.parameter.node_keys_to_subst.iter() {
        node_outputs.insert(subst.0, Output::Unchanged);
        new_node_key_bimap.insert(*key, subst.0);
    }
    for (subst, written_value) in sig.output.maybe_changed_nodes.iter() {
        node_outputs.insert(subst.0, Output::MaybeWritten(written_value.clone()));
    }
    for (marker, new_value) in sig.output.new_nodes.iter() {
        node_outputs.insert(marker.0, Output::New(new_value.clone()));
        let key = output_graph.add_node(new_value.clone());
        new_nodes_to_key.insert(marker, key);
        new_node_key_bimap.insert(key, marker.0);
    }
    for subst in &sig.output.maybe_deleted_nodes {
        node_outputs.insert(subst.0, Output::MaybeDeleted);
    }

    let mut edge_outputs: HashMap<_, Output<S::EdgeAbstract>> = HashMap::new();
    for (src, dst, _edge_av) in sig.parameter.parameter_graph.edges() {
        edge_outputs.insert((src, dst), Output::Unchanged);
    }
    for ((src, dst), edge_av) in sig.output.maybe_changed_edges.iter() {
        let src = sig.parameter.node_keys_to_subst.get_right(src).unwrap();
        let dst = sig.parameter.node_keys_to_subst.get_right(dst).unwrap();
        edge_outputs.insert((*src, *dst), Output::MaybeWritten(edge_av.clone()));
    }
    let sig_to_node_key = |sig_id: &AbstractSignatureNodeId| {
        match sig_id {
            AbstractSignatureNodeId::ExistingNode(subst) => {
                sig.parameter.node_keys_to_subst.get_right(subst).copied().unwrap()
            }
            AbstractSignatureNodeId::NewNode(output_marker) => {
                new_nodes_to_key.get(output_marker).copied().unwrap()
            }
        }
    };
    for ((src, dst), edge_av) in sig.output.new_edges.iter() {
        let src = sig_to_node_key(src);
        let dst = sig_to_node_key(dst);
        edge_outputs.insert((src, dst), Output::New(edge_av.clone()));
        output_graph.add_edge(src, dst, edge_av.clone());
    }
    for (src, dst) in sig.output.maybe_deleted_edges.iter() {
        let src = sig.parameter.node_keys_to_subst.get_right(src).unwrap();
        let dst = sig.parameter.node_keys_to_subst.get_right(dst).unwrap();
        edge_outputs.insert((*src, *dst), Output::MaybeDeleted);
    }

    // first, the parameter dot
    let g = &input_graph.graph;
    let node_label = |g: &_, (key, _)| {
        let subst = sig.parameter.node_keys_to_subst.get_left(&key).copied().unwrap();
        let subst = format!("{}", subst.0);
        let subst = format!("{}", subst.escape_debug());
        let av = sig.parameter.parameter_graph.get_node_attr(key).unwrap();
        let av = format!("{av:?}");
        let av = format!("{}", av.escape_debug());
        format!("label = \"{}: {}\"", subst, av)
    };
    let edge_label = |_, (_, _, attr): (_, _, &EdgeAttribute<_>)| {
        let attr = format!("{:?}", attr.edge_attr);
        let attr = format!("{}", attr.escape_debug());
        format!("label = \"{}\"", attr)
    };
    let param_dot = Dot::with_attr_getters(g,
                                     &[Config::NodeNoLabel, Config::EdgeNoLabel],
                                           &edge_label,
                                            &node_label,
    );

    // then, output dot
    let g = &output_graph.graph;
    let node_label = |g: &_, (key, _)| {
        let marker = new_node_key_bimap.get_left(&key).unwrap();
        let output = node_outputs.get(marker).unwrap();
        let output = format!("{output:?}");
        let output = format!("{}", output.escape_debug());
        format!("label = \"{}: {}\"", marker.0, output)
    };
    let edge_label = |_, (src, dst, attr): (_, _, &EdgeAttribute<_>)| {
        let out = edge_outputs.get(&(src, dst)).unwrap();
        let out = format!("{out:?}");
        let out = format!("{}", out.escape_debug());
        // let attr = format!("{:?}", attr.edge_attr);
        // let attr = format!("{}", attr.escape_debug());
        format!("label = \"{out}\"")
    };
    let output_dot = Dot::with_attr_getters(g,
                                     &[Config::NodeNoLabel, Config::EdgeNoLabel],
                                           &edge_label,
                                            &node_label,
    );

    (format!("{param_dot:?}"), format!("{output_dot:?}"))
}