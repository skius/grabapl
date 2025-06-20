use std::collections::HashMap;
use derive_more::From;
use derive_more::with_trait::Into;
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::visit::NodeIndexable;
use crate::graph::operation::{OperationResult, get_substitution};
use crate::graph::pattern::{AbstractOutputNodeMarker, OperationArgument, OperationParameter, ParameterSubstitution};
use crate::graph::semantics::{AbstractGraph, AbstractMatcher, ConcreteGraph, SemanticsClone};
use crate::{Graph, NodeKey, OperationContext, OperationId, Semantics, SubstMarker};
use crate::graph::EdgeAttribute;
use crate::graph::operation::user_defined::{AbstractOperationResultMarker, QueryInstructions};

pub struct AbstractQueryOutput<S: Semantics> {
    pub changes: Vec<AbstractQueryChange<S>>,
}

pub struct ConcreteQueryOutput {
    pub taken: bool,
}

pub enum AbstractQueryChange<S: Semantics> {
    ExpectNode(NodeChange<S>),
    ExpectEdge(EdgeChange<S>),
}

pub enum NodeChange<S: Semantics> {
    NewNode(SubstMarker, S::NodeAbstract),
}

pub enum EdgeChange<S: Semantics> {
    // TODO: maybe use AbstractNodeId as input for the SubstMarkers?
    ChangeEdgeValue {
        from: SubstMarker,
        to: SubstMarker,
        edge: S::EdgeAbstract,
    },
}

// TODO: What to do about operations that conditionally _remove_ nodes or edges?
//  This implies that our abstract graph may not only be an underapproximation of the concrete graph, but also an overapproximation.
//  This is a problem because we expect anything we see in the abstract graph can be used concretely.
//  One fix might be to turn the abstract change of a "conditional remove" to just abstractly always remove. Then the user would have
//  to check again if something is present, so the same behavior as if we instead added something. This is tedious, but should work.

// TODO: wrt above, the same goes for operations that conditionally _change_ an abstract value. I think it should be the "merge"
//  of the new value and the old value, where the old value is the actual _argument_ abstract value, not the _parameter_ (potentially upcast) abstract
//  value that is defined in the child operation. So basically, the operation says "ChangeTo(new abstract value)", and then the caller has to
//  update its abstract graph accordingly with the merge.

// TODO: Note:
//  What if shape queries had just one builtin, and that builtin was of the form:
//  1. This is my current abstract graph
//  2. Let me make 'pseudo' changes to it. For example, I add a node, and set it as the child of some existing node.
//  3. The query tells me if this matches.
//  How would that work?
//  As the writer of a user defined op, I would need to have know my current abstract graph. We kind of do have that atm I guess? it's the parameter + the sequence of all instructions
//  Then I propose changes. Like NewNode(some ident), AbstractValue(some ident, like new node, param, or dynamic output), Edge, etc.
//  Then I can call the query with those two args (abstract graph, proposed changes) and act based on true/false.
//  Okay.
//  How does the query work?
//  Abstractly it's clear what changes. So do we even need that?
//  Concretely, it's more difficult I think:
//  1. We have the concrete graph
//  2. We have the input abstract graph.
//   - Here we should have some known mapping from concrete to abstract (Side note: potentially again a problem with assigning one node to multiple abstract nodes)
//  3. We also have the proposed changes
//   - With these changes we can build a 'new' abstract graph
//  Can we now use isomorphisms to find a mapping from the new abstract graph (the subgraph) to the concrete graph, that:
//   a) makes sure unchanged nodes in the abstract graph still get mapped to the same nodes in the concrete graph
//   b) changed nodes in the abstract graph can be matched against the ToAbstract version of the concrete nodes' values
//
//   Can the input abstract graph just be a subgraph of the actual abstract graph? Just enough to imply all the necessary context like "next child" or "prev child" or similar?
pub trait BuiltinQuery {
    type S: Semantics;

    /// The pattern to match against the graph.
    fn parameter(&self) -> OperationParameter<Self::S>;

    // TODO: methods to describe abstract shape changes, and a method to dynamically determine which path to take
    //  perhaps the abstract shape change should just more or less be a const fn that returns (Taken, NotTaken) changes?
    //  And concrete just returns which path to take?

    fn abstract_changes(
        &self,
        g: &mut AbstractGraph<Self::S>,
        argument: OperationArgument,
        substitution: &ParameterSubstitution,
    ) -> AbstractQueryOutput<Self::S>;

    fn query(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        substitution: &ParameterSubstitution,
    ) -> ConcreteQueryOutput;
}

pub struct ShapeQuery<S: Semantics> {
    // The context abstract graph to expect
    pub parameter: OperationParameter<S>,
    pub changes: Vec<ShapeQueryChange<S>>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, From, Into)]
pub struct ShapeNodeIdentifier(&'static str);

pub enum AbstractShapeNodeIdentifier {
    /// A node in the parameter graph.
    ParameterMarker(SubstMarker),
    /// A node that is expected from this shape query.
    ShapeQueryNode(ShapeNodeIdentifier),
}

pub enum ShapeQueryChange<S: Semantics> {
    ExpectNode(ShapeNodeChange<S>),
    ExpectEdge(ShapeEdgeChange<S>),
}

pub enum ShapeNodeChange<S: Semantics> {
    /// Expect a new node with the given abstract value and give it the identifier
    NewNode(ShapeNodeIdentifier, S::NodeAbstract),
}

pub enum ShapeEdgeChange<S: Semantics> {
    /// Expect an edge from the node with the given identifier to the node with the given identifier, with the given abstract value
    ExpectedEdgeValue {
        from: AbstractShapeNodeIdentifier,
        to: AbstractShapeNodeIdentifier,
        /// The expected abstract edge value
        edge: S::EdgeAbstract,
    },
}

// pub enum GraphShapeQueryNodeWrapper<S: Semantics> {
//     /// A node that is expected from this shape query.
//     ShapeQueryNode(ShapeNodeIdentifier, S::NodeAbstract),
//     /// A node that has already existed.
//     ExistingNode(NodeKey),
// }
//
// pub enum GraphShapeQueryEdgeWrapper<S: Semantics> {
//     /// An edge that is expected from this shape query.
//     ExpectedEdgeValue {
//         from: AbstractShapeNodeIdentifier,
//         to: AbstractShapeNodeIdentifier,
//         edge: S::EdgeAbstract,
//     },
//     /// An edge that has already existed.
//     ExistingEdge(NodeKey, NodeKey),
// }
//
// pub struct GraphShapeQuery<S: Semantics> {
//     pub parameter: OperationParameter<S>,
//     pub expected_graph: Graph<GraphShapeQueryNodeWrapper<S>, GraphShapeQueryEdgeWrapper<S>>,
// }

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
    pub node_keys_to_shape_idents: HashMap<NodeKey, ShapeNodeIdentifier>,
    pub shape_idents_to_node_keys: HashMap<ShapeNodeIdentifier, NodeKey>,
}


pub struct ConcreteShapeQueryResult {
    // the node_keys here are the concrete keys of the mapped graph
    pub shape_idents_to_node_keys: Option<HashMap<ShapeNodeIdentifier, NodeKey>>,
}

pub(crate) fn run_builtin_query<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    query: &S::BuiltinQuery,
    arg: OperationArgument,
) -> OperationResult<ConcreteQueryOutput> {
    // let param = query.parameter();
    // let abstract_graph = S::concrete_to_abstract(g);
    // let subst = get_substitution(&abstract_graph, &param, &selected_inputs)?;
    let output = query.query(g, &arg.subst);
    Ok(output)
}

// TODO: We could make the graph shape query have match arms in the form of a list of (match_arm_name, expected_graph) list that get checked in sequence
// and the QueryInstructions would contain a hashmap from match_arm_name to the list of instructions to take assuming that match arm is taken.

pub(crate) fn run_shape_query<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    query: &GraphShapeQuery<S>,
    selected_inputs: Vec<NodeKey>,
) -> OperationResult<ConcreteShapeQueryResult> {
    let abstract_graph = S::concrete_to_abstract(g);
    // assert that the abstract graph matches the parameter. this is not the dynamic check yet, this is just asserting
    // that the preconditions of the query are met.
    // ^ since we have an invariant that graphshapequeries dont have context graphs, the returned substitution should just always be the explicit node mapping.
    // let subst = get_substitution(&abstract_graph, &query.parameter, &selected_inputs)?;

    let subst = OperationArgument::infer_explicit_for_param(selected_inputs, &query.parameter)?.subst;

    // Check if the concrete graph matches the expected shape
    // needs to satisfy conditions 1-3 and a-c from above TODO

    // What are we looking for?
    //  We want a mapping from the ShapeNodeIdentifiers in the shape query to the found matched nodes in the concrete graph, if they exist.
    //  At the same time, the concrete graph must match the expected abstract shape changes.

            // Hmm...
            // Maybe it would be nicer to have the ShapeQuery be an explicit Graph with some special node/edge types?
            // The current "list of instructions" is potentially good for an interactive shape query builder, but maybe not the underlying raw representation?
            // let's put ^ on the backburner for now.

    // What do we need from our isomorphism?
    //  1. It needs to search the desired abstract subgraph in the concrete graph (turned into an abstract graph)
    //  2. It needs to assert that the subgraph matches the original parameter substitution result
    //  3. It needs to assert that for any changed pattern, the new pattern is valid according to the instructions.


    // actually, let's try the graph approach first.



    // TODO: implement edge order?

    get_shape_query_substitution(query, &abstract_graph, &subst)

    // TODO: after calling this, the abstract graph needs to somehow know that it can be changed for changed values!
}

fn get_shape_query_substitution<S: SemanticsClone>(
    query: &GraphShapeQuery<S>,
    dynamic_graph: &AbstractGraph<S>,
    subst: &ParameterSubstitution,
) -> OperationResult<ConcreteShapeQueryResult> {
    let desired_shape = &query.expected_graph;

    let desired_shape_ref = &desired_shape.graph;
    let dynamic_graph_ref = &dynamic_graph.graph;

    // derive an enforced mapping from the existing parameter subst
    let mut enforced_desired_to_dynamic: HashMap<NodeKey, NodeKey> = HashMap::new();
    for (subst_marker, dynamic_node_key) in &subst.mapping {
        let desired_node_key = query.parameter.subst_to_node_keys.get(subst_marker).expect("internal error: parameter substitution incorrect");
        // that key must be mapped to the same node in the dynamic query we're running
        enforced_desired_to_dynamic.insert(*desired_node_key, *dynamic_node_key);
    }

    let mut nm = |desired_shape_node_key: &NodeKey, dynamic_graph_node_key: &NodeKey| {
        if let Some(expected_dynamic_node_key) = enforced_desired_to_dynamic.get(desired_shape_node_key) {
            return expected_dynamic_node_key == dynamic_graph_node_key;
        }

        let desired_shape_attr = desired_shape.get_node_attr(*desired_shape_node_key).unwrap();
        let dynamic_graph_attr = dynamic_graph.get_node_attr(*dynamic_graph_node_key).unwrap();
        S::NodeMatcher::matches(dynamic_graph_attr, desired_shape_attr)
    };

    let mut em = |desired_shape_edge_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>, dynamic_graph_edge_attr_wrapper: &EdgeAttribute<S::EdgeAbstract>| {
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

    let opt_mapping = isos.filter_map(|iso| {
        // TODO: handle edge orderedness (factor out into separate function)

        let mapping = iso
            .iter()
            .enumerate()
            .filter_map(|(desired_shape_idx, &dynamic_graph_idx)| {
                let desired_shape_node_key = desired_shape_ref.from_index(desired_shape_idx);
                let dynamic_graph_node_key = dynamic_graph_ref.from_index(dynamic_graph_idx);
                Some((
                    *query.node_keys_to_shape_idents.get(&desired_shape_node_key)?,
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















