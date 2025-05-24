use crate::graph::operation::{OperationResult, get_substitution};
use crate::graph::pattern::{OperationArgument, OperationParameter, ParameterSubstition};
use crate::graph::semantics::{AbstractGraph, ConcreteGraph, SemanticsClone};
use crate::{NodeKey, OperationContext, OperationId, Semantics, SubstMarker};

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
//
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
        substitution: &ParameterSubstition,
    ) -> AbstractQueryOutput<Self::S>;

    fn query(
        &self,
        g: &mut ConcreteGraph<Self::S>,
        argument: OperationArgument,
        substitution: &ParameterSubstition,
    ) -> ConcreteQueryOutput;
}

pub(crate) fn run_builtin_query<S: SemanticsClone>(
    g: &mut ConcreteGraph<S>,
    query: &S::BuiltinQuery,
    selected_inputs: Vec<NodeKey>,
) -> OperationResult<ConcreteQueryOutput> {
    let param = query.parameter();
    let abstract_graph = S::concrete_to_abstract(g);
    let subst = get_substitution(&abstract_graph, &param, &selected_inputs)?;
    let argument = OperationArgument {
        selected_input_nodes: selected_inputs,
    };
    let output = query.query(g, argument, &subst);
    Ok(output)
}
