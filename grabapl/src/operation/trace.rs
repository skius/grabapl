//! Functionality related to tracing operations at runtime.

use std::collections::HashSet;
use std::fmt::Debug;
use petgraph::dot::{Config, Dot};
use petgraph::visit::NodeIndexable;
use crate::prelude::{AbstractNodeId, ConcreteGraph};
use crate::{NodeKey, Semantics};
use crate::graph::dot::DotCollector;
use crate::graph::EdgeAttribute;
use crate::operation::marker::MarkerSet;
use crate::util::bimap::BiMap;

pub struct TraceFrame<S: Semantics> {
    pub graph: ConcreteGraph<S>,
    pub hidden_nodes: HashSet<NodeKey>,
    pub marker_set: MarkerSet,
    pub node_aids: BiMap<NodeKey, AbstractNodeId>,
}

impl<S: Semantics<NodeConcrete: Debug, EdgeConcrete: Debug>> TraceFrame<S> {
    /// Returns the frame visualized as a DOT string.
    ///
    /// Hidden nodes and currently mapped nodes are visualized.
    pub fn dot(&self) -> String {
        let g = &self.graph.graph;

        let edge_get = |_g, (_, _, attr): (_, _, &EdgeAttribute<_>)| {
            let dbg_attr_format = format!("{:?}", attr.edge_attr);
            let dbg_attr_replaced = dbg_attr_format.escape_debug();
            format!("label = \"{dbg_attr_replaced}\"")
        };

        let node_get = |_g, (key, _)| {
            let value_debug = format!("{:?}", self.graph.get_node_attr(key).unwrap());
            let mut value_escaped = value_debug.escape_debug().to_string();
            if let Some(markers) = self.marker_set.marked_nodes_to_markers.get(&key) {
                // if the node has markers, append them to the value
                let markers_inner = markers.iter()
                    .map(|m| format!("{m:?}"))
                    .collect::<Vec<_>>()
                    .join(",");
                // escape { and } because they have semantic meaning in graphviz record shapes
                value_escaped = format!("{value_escaped}:\\{{{markers_inner}\\}}");
            }
            // decide between the following options:
            if let Some(node_aid) = self.node_aids.get_left(&key) {
                // node in current frame
                let aid_debug = node_aid.to_string_dot_syntax();
                let aid_escaped = aid_debug.escape_debug();
                format!("shape=Mrecord, color=\"blue\" label = \"{aid_escaped}|{value_escaped}\"")
            } else if self.hidden_nodes.contains(&key) {
                // hidden node
                format!("style=\"filled\" color=\"brown\" fillcolor=\"moccasin\" label = \"{value_escaped}\"")
            } else {
                // pure runtime node, available for shape matching
                format!("style=\"filled\" fillcolor=\"gray72\" label = \"{value_escaped}\"")
            }
        };
        let index_get = |g, key: NodeKey| format!("{}", key.0);
        let dot = Dot::with_attr_getters_and_index_getter(g, &[Config::EdgeNoLabel, Config::NodeNoLabel],
                                                          &edge_get,
                                                          &node_get,
                                                          &index_get,
        );

        format!("{dot:?}")
    }
}

pub struct Trace<S: Semantics> {
    pub frames: Vec<TraceFrame<S>>,
}

impl<S: Semantics> Trace<S> {
    pub fn new() -> Self {
        Trace { frames: Vec::new() }
    }

    pub fn push_frame(&mut self, frame: TraceFrame<S>) {
        self.frames.push(frame);
    }
}

impl<S: Semantics<NodeConcrete: Debug, EdgeConcrete: Debug>> Trace<S> {
    pub fn chained_dot(&self) -> String {
        let mut dot_collector = DotCollector::new();
        for frame in &self.frames {
            dot_collector.collect_raw(&frame.dot());
        }
        dot_collector.finalize()
    }
}