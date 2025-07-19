use crate::graph::EdgeAttribute;
use crate::operation::OperationResult;
use crate::operation::signature::parameter::{
    GraphWithSubstitution, OperationArgument, OperationParameter, ParameterSubstitution,
};
use crate::semantics::{AbstractGraph, AbstractMatcher, ConcreteGraph, Semantics};
use crate::util::bimap::BiMap;
use crate::util::{InternString, log};
use crate::{NodeKey, interned_string_newtype};
use derive_more::From;
use derive_more::with_trait::Into;
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::visit::NodeIndexable;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::operation::marker::{MarkerSet, SkipMarkers};

pub trait BuiltinQuery {
    type S: Semantics;

    /// The pattern to match against the graph.
    fn parameter(&self) -> OperationParameter<Self::S>;

    // TODO: add invariant (checked?) that the abstract graph does not get new nodes or deleted nodes.
    //  actually, do we really need modification at all? ...

    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>);

    // TODO: if we decide to actually support modification, we need to include an OperationOutput so that we can support new nodes and can keep track of
    //  changes of av's.
    fn query(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> ConcreteQueryOutput;
}

pub struct ConcreteQueryOutput {
    pub taken: bool,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, From, Into)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShapeNodeIdentifier(InternString);
interned_string_newtype!(ShapeNodeIdentifier);

#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound = "S: crate::serde::SemanticsSerde")
)]
pub struct GraphShapeQuery<S: Semantics> {
    // TODO: perhaps we don't need a fullblown OperationParameter here, since we don't really need SubstMarker?
    //  yeah, we really don't want to deal with context graphs here.
    // INVARIANT: the context graph must be empty. All inputs must be explicit.
    pub parameter: OperationParameter<S>,
    // The keys here for the existing nodes must be equivalent to parameter.graph
    // TODO: assert this property or refactor^
    pub expected_graph: AbstractGraph<S>,
    // In the expected graph, these nodes are _new_ nodes that are expected to be created by the operation and that will be returned with a mapping
    // the node keys is the key for the expected_graph.
    pub node_keys_to_shape_idents: BiMap<NodeKey, ShapeNodeIdentifier>,
    // nodes marked with which markers should be skipped in the shape query
    pub skip_markers: SkipMarkers,
}

impl<S: Semantics> Clone for GraphShapeQuery<S> {
    fn clone(&self) -> Self {
        GraphShapeQuery {
            parameter: self.parameter.clone(),
            expected_graph: self.expected_graph.clone(),
            node_keys_to_shape_idents: self.node_keys_to_shape_idents.clone(),
            skip_markers: self.skip_markers.clone(),
        }
    }
}

impl<S: Semantics> GraphShapeQuery<S> {
    pub fn new(
        parameter: OperationParameter<S>,
        expected_graph: AbstractGraph<S>,
        node_keys_to_shape_idents: BiMap<NodeKey, ShapeNodeIdentifier>,
    ) -> Self {
        GraphShapeQuery {
            parameter,
            expected_graph,
            node_keys_to_shape_idents,
            skip_markers: SkipMarkers::default(),
        }
    }

    pub fn with_skip_markers(
        mut self,
        skip_markers: SkipMarkers,
    ) -> Self {
        self.skip_markers = skip_markers;
        self
    }
}

pub struct ConcreteShapeQueryResult {
    /// The `NodeKey`s are the concrete keys of the real graph
    /// Some(mapping) if the shape query matched, None if it did not match.
    pub shape_idents_to_node_keys: Option<HashMap<ShapeNodeIdentifier, NodeKey>>,
}

pub(crate) fn run_builtin_query<S: Semantics>(
    g: &mut ConcreteGraph<S>,
    query: &S::BuiltinQuery,
    arg: OperationArgument,
) -> OperationResult<ConcreteQueryOutput> {
    let mut gws = GraphWithSubstitution::new(g, &arg.subst);
    let output = query.query(&mut gws);
    Ok(output)
}

// TODO: We could make the graph shape query have match arms in the form of a list of (match_arm_name, expected_graph) list that get checked in sequence
//  and the QueryInstructions would contain a hashmap from match_arm_name to the list of instructions to take assuming that match arm is taken.

/// Runs a shape query on the given concrete graph.
///
/// It works by finding an isomorphism between the expected abstract graph (of the shape query) and
/// most precise abstraction of the concrete graph. The selected inputs anchor the shape query to a
/// specific region of the concrete graph.
///
/// This is for concrete graphs. Abstract graphs handle shape queries explicitly in the [`OperationBuilder`].
///
/// [`OperationBuilder`]: crate::operation::builder::OperationBuilder
pub(crate) fn run_shape_query<S: Semantics>(
    g: &mut ConcreteGraph<S>,
    query: &GraphShapeQuery<S>,
    selected_inputs: &[NodeKey],
    hidden_nodes: &HashSet<NodeKey>,
    marker_set: &MarkerSet,
) -> OperationResult<ConcreteShapeQueryResult> {
    let abstract_graph = S::concrete_to_abstract(g);
    let subst = ParameterSubstitution::infer_explicit_for_param(selected_inputs, &query.parameter)?;

    // TODO: implement edge order?

    let mut hidden_nodes_incl_marker_hidden = hidden_nodes.clone();
    hidden_nodes_incl_marker_hidden.extend(marker_set.skipped_nodes(&query.skip_markers));


    get_shape_query_substitution(query, &abstract_graph, &subst, &hidden_nodes_incl_marker_hidden)

    // TODO: after calling this, the abstract graph needs to somehow know that it can be changed for changed values!
}

fn get_shape_query_substitution<S: Semantics>(
    query: &GraphShapeQuery<S>,
    dynamic_graph: &AbstractGraph<S>,
    subst: &ParameterSubstitution,
    hidden_nodes: &HashSet<NodeKey>,
) -> OperationResult<ConcreteShapeQueryResult> {
    log::trace!(
        "Running shape query with hidden nodes {:?} and parameter substitution {:?}",
        hidden_nodes,
        subst
    );
    let desired_shape = &query.expected_graph;

    let desired_shape_ref = &desired_shape.graph;
    let dynamic_graph_ref = &dynamic_graph.graph;

    // derive an enforced mapping from the existing parameter subst
    let mut enforced_desired_to_dynamic: HashMap<NodeKey, NodeKey> = HashMap::new();
    for (subst_marker, dynamic_node_key) in &subst.mapping {
        let desired_node_key = query
            .parameter
            .node_keys_to_subst
            .get_right(subst_marker)
            .expect("internal error: parameter substitution incorrect");
        // that key must be mapped to the same node in the dynamic query we're running
        enforced_desired_to_dynamic.insert(*desired_node_key, *dynamic_node_key);
    }

    let mut nm = |desired_shape_node_key: &NodeKey, dynamic_graph_node_key: &NodeKey| {
        if let Some(expected_dynamic_node_key) =
            enforced_desired_to_dynamic.get(desired_shape_node_key)
        {
            // if we have an enforced mapping ...
            if expected_dynamic_node_key != dynamic_graph_node_key {
                // ... and the mapping does not match, we early-exit
                // (but crucially don't return if it does match: we still need to check the attributes)
                return false;
            }
        } else {
            // out of the non-enforced-mapping nodes, we explicitly don't want to match hidden nodes
            if hidden_nodes.contains(dynamic_graph_node_key) {
                log::info!(
                    "Skipping hidden node {:?} in dynamic graph for shape query",
                    dynamic_graph_node_key
                );
                return false;
            }
        }

        let desired_shape_attr = desired_shape
            .get_node_attr(*desired_shape_node_key)
            .unwrap();
        let dynamic_graph_attr = dynamic_graph
            .get_node_attr(*dynamic_graph_node_key)
            .unwrap();
        S::NodeMatcher::matches(dynamic_graph_attr, desired_shape_attr)
    };

    let mut em =
        |desired_shape_edge_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>,
         dynamic_graph_edge_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>| {
            let desired_shape_edge_attr = &desired_shape_edge_attr_wrapper.edge_attr;
            let dynamic_graph_edge_attr = &dynamic_graph_edge_attr_wrapper.edge_attr;
            S::EdgeMatcher::matches(dynamic_graph_edge_attr, desired_shape_edge_attr)
        };

    let Some(isos) = general_subgraph_monomorphisms_iter(
        &desired_shape_ref,
        &dynamic_graph_ref,
        &mut nm,
        &mut em,
    ) else {
        return Ok(ConcreteShapeQueryResult {
            shape_idents_to_node_keys: None,
        });
    };

    let opt_mapping = isos
        .filter_map(|iso| {
            // TODO: handle edge orderedness (factor out into separate function)

            let mapping = iso
                .iter()
                .enumerate()
                .filter_map(|(desired_shape_idx, &dynamic_graph_idx)| {
                    let desired_shape_node_key = desired_shape_ref.from_index(desired_shape_idx);
                    let dynamic_graph_node_key = dynamic_graph_ref.from_index(dynamic_graph_idx);
                    Some((
                        query
                            .node_keys_to_shape_idents
                            .get_left(&desired_shape_node_key)?
                            .clone(),
                        dynamic_graph_node_key,
                    ))
                })
                .collect::<HashMap<_, _>>();

            Some(mapping)
        })
        .next();

    Ok(ConcreteShapeQueryResult {
        shape_idents_to_node_keys: opt_mapping,
    })
}
