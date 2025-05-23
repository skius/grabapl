use crate::graph::pattern::{OperationArgument, OperationParameter, ParameterSubstition};
use crate::graph::semantics::{AbstractGraph, ConcreteGraph, SemanticsClone};
use crate::{NodeKey, OperationContext, OperationId, Semantics, SubstMarker};
use crate::graph::operation::{get_substitution, OperationResult};

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
    }
}

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