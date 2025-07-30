//! This module provides functionality related to building user defined operations.
//!
//! The main functionality is provided by the [`OperationBuilder`] type.
//!
//! This should be used as the primary backend used by frontends that want to allow end-users to create
//! their own operations.
//!
//! The main method of communication is through atomic "instructions" sent to the builder.
//! These are flat instructions: any nesting in the resulting user defined operation is a result
//! of explicit "nesting" instructions, such as [`OperationBuilder::start_query`].
//!
//! You can think of these instructions as the HIR (high-level intermediate representation) of
//! grabapl, which the builder compiles into bytecode (i.e., the final user defined operation) for the interpreter.
//!
//! See the [`OperationBuilder`] documentation for the available instructions.
//!
//! # Example
//! Assume we want to build a text-based frontend that allows users to create their own operations.
//!
//! We may want to support syntax such as the following:
//! ```rust
//! # use grabapl::semantics::example::ExampleSemantics;
//! # use syntax::grabapl_parse;
//! # grabapl_parse!(ExampleSemantics,
//! fn mark_children_as_visited(parent: int) {
//!     if shape [child: int, parent -> child: *] {
//!         mark_node<"visited">(child);
//!         // we found a child, hence we should recurse to find more children
//!         mark_children_as_visited(parent);
//!     }
//! }
//! # );
//! ```
//!
//! If we leverage this builder, all our frontend would need to do in order to get a finished user defined operation,
//! is turn the above syntax example into the following sequence of instructions:
//! 1. [`expect_parameter_node("parent", NodeType::Int)`](OperationBuilder::expect_parameter_node) - the parameter definition
//! 2. [`start_shape_query("<generated name>")`](OperationBuilder::start_shape_query) - the start of the shape query
//! 3. [`expect_shape_node("child", NodeType::Int)`](OperationBuilder::expect_shape_node) - the shape query expects a child node of type int
//! 4. [`expect_shape_edge("parent", "child", EdgeType::Wildcard)`](OperationBuilder::expect_shape_edge) - the shape query expects an edge from parent to child
//! 5. [`enter_true_branch()`](OperationBuilder::enter_true_branch) - we enter the true branch of the shape query
//!    * Note how this is a flat instruction: We don't pass the entire true branch as argument to the method.
//!      Instead, we _change the context_ to indicate the following instructions are part of the true branch.
//! 6. [`add_operation(LibBuiltinOperation::MarkNode("visited"), vec!["child"])`](OperationBuilder::add_operation) - we add an operation that marks the child node as visited
//! 7. [`add_operation(Recurse, vec!["parent"])`](OperationBuilder::add_operation) - we add an operation that recurses to find more children
//! 8. [`end_query()`](OperationBuilder::end_query) - we end the shape query
//!
//! After sending these instructions to the builder we can call [`OperationBuilder::build()`](OperationBuilder::build)
//! to get the final user defined operation that can then be added to a [`OperationContext`](crate::operation::OperationContext) and
//! executed by the interpreter via [`run_from_concrete`](crate::operation::run_from_concrete).
//!
//! # Example Frontends
//! See [`grabapl_syntax`](https://crates.io/crates/grabapl_syntax) for a text-based syntax
//! frontend that compiles parsed ASTs into instructions for this builder.
//! This implements our example from above.
//!
//! See `example_clients/simple_semantics/{simple_semantics_ffi, www}` for a basic visual editor that
//! uses commands from the user to convert into instructions for this builder, and takes the builder's
//! intermediate state to give visual feedback to the user.

use crate::operation::builtin::LibBuiltinOperation;
use crate::operation::marker::Marker;
use crate::operation::query::{BuiltinQuery, ShapeNodeIdentifier};
use crate::operation::signature::parameter::{
    AbstractOperationOutput, AbstractOutputNodeMarker, GraphWithSubstitution, OperationParameter,
    ParameterSubstitution,
};
use crate::operation::signature::{OperationSignature};
use crate::operation::user_defined::{
    AbstractNodeId, AbstractOperationArgument, AbstractOperationResultMarker, NamedMarker, OpLikeInstruction,
};
use crate::operation::{Operation, OperationError, OperationResult, get_substitution};
use crate::prelude::*;
use crate::semantics::{AbstractGraph};
use crate::util::bimap::BiMap;
use crate::util::log;
use crate::{NodeKey, Semantics, SubstMarker};
use error_stack::{Result, ResultExt, bail, report};
use petgraph::dot;
use petgraph::dot::Dot;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;use thiserror::Error;

mod programming_by_demonstration;
pub mod stack_based_builder;
/*
General overview:

1. While building, the builder just stores the messages sent to it.
We cannot do fancy compile-time checks like "every query has a condition and two branches", because
every step of that (condition, true branch, false branch) should be interruptible and resumable.
E.g., a frontend needs to be able to give intermediate feedback to the user, so that the user
can work with that feedback and send new messages to the builder.

However, to give good feedback for which messages are appropriate, we construct the operation on the fly (TODO: cache this?),
so that errors like invalid identifiers or ending a query without starting one can be caught immediately at message-time.
This is the same routine that can provide state feedback to the user like:
 * right now you're in this branch of that query
 * the abstract graph looks like this
 * more ???

The intermediate state returns a graph and a hashmap from nodes and edges to additional metadata, like their abstract node id.
*/

/// An operation that can be applied abstractly
enum AbstractOperation<'a, S: Semantics> {
    Op(Operation<'a, S>),
    Partial(&'a OperationSignature<S>),
}

impl<'a, S: Semantics> AbstractOperation<'a, S> {
    fn parameter(&self) -> OperationParameter<S> {
        match self {
            AbstractOperation::Op(op) => op.parameter(),
            AbstractOperation::Partial(sig) => sig.parameter.clone(),
        }
    }

    fn apply_abstract(
        &self,
        op_ctx: &OperationContext<S>,
        g: &mut GraphWithSubstitution<AbstractGraph<S>>,
    ) -> OperationResult<AbstractOperationOutput<S>> {
        match self {
            AbstractOperation::Op(op) => op.apply_abstract(op_ctx, g),
            AbstractOperation::Partial(sig) => Ok(sig.output.apply_abstract(g)),
        }
    }
}

pub enum BuilderOpLike<S: Semantics> {
    Builtin(S::BuiltinOperation),
    LibBuiltin(LibBuiltinOperation<S>),
    FromOperationId(OperationId),
    Recurse,
}

impl<S: Semantics> BuilderOpLike<S> {
    fn as_abstract_operation<'a>(
        &'a self,
        op_ctx: &'a OperationContext<S>,
        partial_self_signature: &'a OperationSignature<S>,
    ) -> Result<AbstractOperation<'a, S>, OperationBuilderError> {
        let op = match self {
            BuilderOpLike::Builtin(op) => AbstractOperation::Op(Operation::Builtin(op)),
            BuilderOpLike::LibBuiltin(op) => AbstractOperation::Op(Operation::LibBuiltin(op)),
            BuilderOpLike::FromOperationId(id) => {
                let op = op_ctx
                    .get(*id)
                    .ok_or(OperationBuilderError::NotFoundOperationId(*id))?;
                AbstractOperation::Op(op)
            }
            BuilderOpLike::Recurse => AbstractOperation::Partial(partial_self_signature),
        };
        Ok(op)
    }

    fn into_op_like_instruction(self, self_op_id: OperationId) -> OpLikeInstruction<S> {
        match self {
            BuilderOpLike::Builtin(op) => OpLikeInstruction::Builtin(op),
            BuilderOpLike::LibBuiltin(op) => OpLikeInstruction::LibBuiltin(op),
            BuilderOpLike::FromOperationId(id) => OpLikeInstruction::Operation(id),
            BuilderOpLike::Recurse => OpLikeInstruction::Operation(self_op_id),
        }
    }
}

impl<S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>> Clone for BuilderOpLike<S> {
    fn clone(&self) -> Self {
        match self {
            BuilderOpLike::Builtin(op) => BuilderOpLike::Builtin(op.clone()),
            BuilderOpLike::LibBuiltin(op) => BuilderOpLike::LibBuiltin(op.clone()),
            BuilderOpLike::FromOperationId(id) => BuilderOpLike::FromOperationId(*id),
            BuilderOpLike::Recurse => BuilderOpLike::Recurse,
        }
    }
}

// TODO: rename to BuilderMessage? since Instruction is already used in the user-defined operation context.
#[derive(derive_more::Debug)]
pub enum BuilderInstruction<S: Semantics> {
    #[debug("ExpectParameterNode({_0:?}, ???)")]
    ExpectParameterNode(SubstMarker, S::NodeAbstract),
    #[debug("ExpectContextNode({_0:?}, ???)")]
    ExpectContextNode(SubstMarker, S::NodeAbstract),
    #[debug("ExpectParameterEdge({_0:?}, {_1:?}, ???)")]
    ExpectParameterEdge(SubstMarker, SubstMarker, S::EdgeAbstract),
    #[debug("StartQuery(???, args: {_1:?})")]
    StartQuery(S::BuiltinQuery, Vec<AbstractNodeId>),
    #[debug("EnterTrueBranch")]
    EnterTrueBranch,
    #[debug("EnterFalseBranch")]
    EnterFalseBranch,
    // TODO: think about what happens when we start two shape queries with the same name. the gsq_op_marker if statement below somewhere is a problem.
    //  specifically, when they're nested (eg one with name "foo", true branch, another one with "foo").
    //  potentially could be fine to support, but needs implementation work.
    #[debug("StartShapeQuery({_0:?})")]
    StartShapeQuery(AbstractOperationResultMarker),
    #[debug("EndQuery")]
    EndQuery,
    #[debug("ExpectShapeNode({_0:?}, ???)")]
    // TODO: maybe should be renamed to ExpectNewShapeNode?
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeNodeChange({_0:?}, ???)")]
    ExpectShapeNodeChange(AbstractNodeId, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:?}, {_1:?}, ???)")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("SkipMarker({_0:?})")]
    SkipMarker(Marker),
    #[debug("SkipAllMarkers")]
    SkipAllMarkers,
    #[debug("AddNamedOperation({_0:?}, ???, args: {_2:?})")]
    AddNamedOperation(
        AbstractOperationResultMarker,
        BuilderOpLike<S>,
        Vec<AbstractNodeId>,
    ),
    // the same as AddNamedOperation, but without enforces the output to have a single node, and uses that node
    // to create a AbstractNodeId::named node to bind to it.
    #[debug("AddBangOperation({_0:?}, ???, args: {_2:?})")]
    AddBangOperation(NamedMarker, BuilderOpLike<S>, Vec<AbstractNodeId>),
    #[debug("AddOperation(???, args: {_1:?})")]
    AddOperation(BuilderOpLike<S>, Vec<AbstractNodeId>),
    #[debug("ReturnNode({_0:?}, {_1:?}, ???)")]
    ReturnNode(AbstractNodeId, AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ReturnEdge({_0:?}, {_1:?}, ???)")]
    ReturnEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("RenameNode({_0:?}, {_1:?})")]
    /// Rename a dynamic output marker.
    /// Invariants in the interpreter require that this is never a parameter node. (E.g., since we may want to return it)
    RenameNode(AbstractNodeId, NamedMarker),
    Finalize,
    /// Asserts that the current operation will return a node with the given abstract value and name.
    #[debug("SelfReturnNode({_0:?}, ???)")]
    SelfReturnNode(AbstractOutputNodeMarker, S::NodeAbstract),
    /// Diverge with a crash message.
    /// Has a static effect: The branch is considered to never return, hence merges will always take the other branch.
    #[debug("Diverge({_0})")]
    Diverge(String),
    /// Add the current operation's frame to the operation trace
    #[debug("Trace")]
    Trace,
}

impl<S: Semantics> BuilderInstruction<S> {
    /// Returns true if this is an instruction that is valid to break out of a body of query/operation
    /// instructions.
    fn can_break_body(&self) -> bool {
        use BuilderInstruction::*;
        matches!(
            self,
            EnterTrueBranch
                | EnterFalseBranch
                | EndQuery
                | ReturnNode(..)
                | ReturnEdge(..)
                | Finalize
        )
    }
}

impl<S: Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>> Clone for BuilderInstruction<S> {
    fn clone(&self) -> Self {
        use BuilderInstruction::*;
        match self {
            ExpectParameterNode(marker, node) => ExpectParameterNode(*marker, node.clone()),
            ExpectContextNode(marker, node) => ExpectContextNode(*marker, node.clone()),
            ExpectParameterEdge(source_marker, target_marker, edge) => {
                ExpectParameterEdge(*source_marker, *target_marker, edge.clone())
            }
            StartQuery(query, args) => StartQuery(query.clone(), args.clone()),
            EnterTrueBranch => EnterTrueBranch,
            EnterFalseBranch => EnterFalseBranch,
            StartShapeQuery(op_marker) => StartShapeQuery(*op_marker),
            EndQuery => EndQuery,
            ExpectShapeNode(marker, node) => ExpectShapeNode(*marker, node.clone()),
            ExpectShapeNodeChange(aid, node) => ExpectShapeNodeChange(*aid, node.clone()),
            ExpectShapeEdge(source, target, edge) => {
                ExpectShapeEdge(*source, *target, edge.clone())
            }
            SkipMarker(marker) => SkipMarker(*marker),
            SkipAllMarkers => SkipAllMarkers,
            AddNamedOperation(name, op, args) => AddNamedOperation(*name, op.clone(), args.clone()),
            AddBangOperation(name, op, args) => AddBangOperation(*name, op.clone(), args.clone()),
            AddOperation(op, args) => AddOperation(op.clone(), args.clone()),
            ReturnNode(aid, output_marker, node) => ReturnNode(*aid, *output_marker, node.clone()),
            ReturnEdge(src, dst, edge) => ReturnEdge(*src, *dst, edge.clone()),
            RenameNode(old_aid, new_name) => RenameNode(*old_aid, *new_name),
            Finalize => Finalize,
            SelfReturnNode(marker, node) => SelfReturnNode(*marker, node.clone()),
            Diverge(msg) => Diverge(msg.clone()),
            Trace => Trace,
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum OperationBuilderError {
    #[error("Expected a new unique subst marker, found repeat: {0:?}")]
    ReusedSubstMarker(SubstMarker),
    #[error("Expected an existing subst marker, but {0:?} was not found")]
    NotFoundSubstMarker(SubstMarker),
    #[error("Expected a new unique subst marker, found repeat: {0:?}")]
    ReusedShapeIdent(ShapeNodeIdentifier),
    #[error("Cannot call this while in a query context")]
    InvalidInQuery,
    #[error("Expected an operation or query")]
    ExpectedOperationOrQuery,
    #[error("Already visited the {0} branch of the active query")]
    AlreadyVisitedBranch(bool),
    #[error("Could not find abstract node id: {0:?}")]
    NotFoundAid(AbstractNodeId),
    #[error("AID {0:?} already exists")]
    AlreadyExistsAid(AbstractNodeId),
    #[error("Could not find operation ID: {0}")]
    NotFoundOperationId(OperationId),
    #[error("Could not apply operation due to mismatched arguments: {0}")]
    SubstitutionError(#[from] crate::operation::SubstitutionError),
    #[error("Could not apply operation due to mismatched arguments")]
    SubstitutionErrorNew,
    #[error("Could not abstractly apply operation {0} due to: {1}")]
    AbstractApplyOperationErrorWithId(OperationId, OperationError),
    #[error("Could not abstractly apply operation due to: {0}")]
    AbstractApplyOperationError(OperationError),
    #[error("Could not abstractly apply operation")]
    AbstractApplyOperationError2,
    #[error("Superfluous instruction {0}")]
    SuperfluousInstruction(String),
    #[error("Already selected to return node {0:?}")]
    AlreadySelectedReturnNode(AbstractNodeId),
    #[error("Already selected to return edge {0:?}->{1:?}")]
    AlreadySelectedReturnEdge(AbstractNodeId, AbstractNodeId),
    #[error("Could not find AID {0:?} for return node")]
    NotFoundReturnNode(AbstractNodeId),
    #[error("Invalid return node type for AID {0:?}, must be more generic")]
    InvalidReturnNodeType(AbstractNodeId),
    // this is now kind of allowed - in the future it might be disallowed again.
    #[error("Returned {0:?} node may have been created by a shape query, which is not allowed")]
    ReturnNodeMayOriginateFromShapeQuery(AbstractNodeId),
    #[error("Cannot return a parameter node: {0:?}")]
    CannotReturnParameter(AbstractNodeId),
    #[error("Could not find AID {0:?} for return edge source")]
    NotFoundReturnEdgeSource(AbstractNodeId),
    #[error("Could not find AID {0:?} for return edge target")]
    NotFoundReturnEdgeTarget(AbstractNodeId),
    #[error("Could not statically determine edge {0:?}->{1:?} to be available")]
    NotFoundReturnEdge(AbstractNodeId, AbstractNodeId),
    #[error("Invalid return edge type for AID {0:?}->{1:?}, must be more generic")]
    InvalidReturnEdgeType(AbstractNodeId, AbstractNodeId),
    #[error(
        "Return edge {0:?}->{1:?} may have been created by a shape query, which is not allowed"
    )]
    ReturnEdgeMayOriginateFromShapeQuery(AbstractNodeId, AbstractNodeId),
    #[error("internal error: {0}")]
    InternalError(&'static str),
    #[error("Explicitly selected input AID not found")]
    SelectedInputsNotFoundAid,
    #[error("Shape edge target node not found")]
    ShapeEdgeTargetNotFound,
    #[error("Shape edge source node not found")]
    ShapeEdgeSourceNotFound,
    #[error(
        "Cannot rename parameter node {0:?}, only new nodes from operation calls can be renamed"
    )]
    CannotRenameParameterNode(AbstractNodeId),
    #[error("Invalid parameter")]
    InvalidParameter,
    #[error("Unexpected instruction")]
    UnexpectedInstruction,
    #[error("Failed to build operation parameter")]
    ParameterBuildError,
    // TODO: maybe have enum variants for these
    #[error("{0}")]
    Oneoff(&'static str),
    #[error("Shape node already exists: {}", _0.0)]
    ShapeNodeAlreadyExists(ShapeNodeIdentifier)
}

// type alias to switch between implementations globally
// pub type OperationBuilder<'a, S> = OperationBuilderInefficient<'a, S>;
pub type OperationBuilder<'a, S> = stack_based_builder::OperationBuilder2<'a, S>;

/*
What kind of information do we want to give the user when they ask for the current state of the operation?

1. Current abstract graph
 * Realistically, this should be formatted by ignoring NodeKeys and only showing AbstractNodeId
 ==> We need a NodeKey => AbstractNodeId mapping
2. Available AbstractNodeIds and their abstract values
 * We can do this by mapping AbstractNodeId to NodeKey and then looking up the node in the graph.
 ==> We need an AbstractNodeId => NodeKey mapping
3. Current query state
 * How should this be represented?
 * Some path? Can we "visualize" queries?
 * then we could have paths like: "GtZero on AID_1 true branch, ShapeQuery Y (Shape queries will be difficult to visualize)
   on AID_2 and AID_3 false branch, EqValues on AID_3 and AID_4 no branch yet"


How do we store intermediate representation?
To do this memory-efficiently, some incremental representation would be nice. Like "this instruction added this AID".
But, for time reasons, let's just store a copy of the entire state from above after each instruction.
// TODO: update above comment. we store fewer intermediate states in the stack based builder.
*/

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryPath {
    Query(String),
    TrueBranch,
    FalseBranch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IntermediateStateAbstractOutputResult {
    new_aids: Vec<AbstractNodeId>,
    removed_aids: Vec<AbstractNodeId>,
}

// TODO: Store more information like:
//  - Are we still building the parameter graph?
//  - If we are inside a query, which branches have we not entered yet?
//  - Are we making a shape/non-shape query?

// TODO: should this be named "AbstractBuilderState"? since it's the state, which is abstract, which is used by the builder.
pub struct IntermediateState<S: Semantics> {
    pub graph: AbstractGraph<S>,
    pub node_keys_to_aid: BiMap<NodeKey, AbstractNodeId>,
    // TODO: Somehow remove AIDs from this set if they're completely overwritten by something non-shape-query.
    //  could be done by, whenever adding a new node, unconditionally removing the AID from this set as long as we're not in a shape query.
    //  since we have a different state at that point, it would get merged correctly (assuming we take the union).
    // *UPDATE*: these are currently being populated, but not used, since I realized nodes from shape queries are actually
    //  allowed to be returned. In the future these might be used again, e.g., when a shape query can read-only match an *already existing*
    //  outer node. In that case, the node would not be allowed to be returned, since it may exist in the caller.
    pub node_may_originate_from_shape_query: HashSet<AbstractNodeId>,
    pub edge_may_originate_from_shape_query: HashSet<(AbstractNodeId, AbstractNodeId)>,

    /// The most generic abstract type that may be written to each node, if any.
    pub node_may_be_written_to: HashMap<AbstractNodeId, S::NodeAbstract>,
    /// The most generic abstract type that may be written to each edge, if any.
    pub edge_may_be_written_to: HashMap<(AbstractNodeId, AbstractNodeId), S::EdgeAbstract>,

    // TODO: make query path
    // TODO: should probably remove query_path from the state struct, and add it to a final returned StateWithQueryPath struct?
    pub query_path: Vec<QueryPath>,

    pub op_marker_counter: u64,

    pub has_diverged: bool,
}

// TODO: unfortunately, we cannot derive Clone, since it implies a `S: Clone` bound.
//  - in theory, we could add that bound, since a Semantics as a value does not really store much. So clone should be fine.
impl<S: Semantics> Clone for IntermediateState<S> {
    fn clone(&self) -> Self {
        IntermediateState {
            graph: self.graph.clone(),
            node_keys_to_aid: self.node_keys_to_aid.clone(),
            node_may_originate_from_shape_query: self.node_may_originate_from_shape_query.clone(),
            edge_may_originate_from_shape_query: self.edge_may_originate_from_shape_query.clone(),
            node_may_be_written_to: self.node_may_be_written_to.clone(),
            edge_may_be_written_to: self.edge_may_be_written_to.clone(),
            query_path: self.query_path.clone(),
            op_marker_counter: self.op_marker_counter,
            has_diverged: self.has_diverged,
        }
    }
}

impl<S: Semantics> IntermediateState<S> {
    fn new() -> Self {
        IntermediateState {
            graph: AbstractGraph::<S>::new(),
            node_keys_to_aid: BiMap::new(),
            node_may_originate_from_shape_query: HashSet::new(),
            edge_may_originate_from_shape_query: HashSet::new(),
            node_may_be_written_to: HashMap::new(),
            edge_may_be_written_to: HashMap::new(),
            query_path: Vec::new(),
            op_marker_counter: 50000,
            has_diverged: false,
        }
    }

    fn from_param(param: &OperationParameter<S>) -> Self {
        let initial_graph = param.parameter_graph.clone();

        let mut initial_mapping = BiMap::new();

        for (key, subst) in param.node_keys_to_subst.iter() {
            let aid = AbstractNodeId::ParameterMarker(*subst);
            initial_mapping.insert(*key, aid);
        }

        IntermediateState {
            graph: initial_graph,
            node_keys_to_aid: initial_mapping,
            node_may_originate_from_shape_query: HashSet::new(),
            edge_may_originate_from_shape_query: HashSet::new(),
            node_may_be_written_to: HashMap::new(),
            edge_may_be_written_to: HashMap::new(),
            query_path: Vec::new(),
            op_marker_counter: 50000,
            has_diverged: false,
        }
    }

    fn get_next_op_result_marker(&mut self) -> AbstractOperationResultMarker {
        let marker = AbstractOperationResultMarker::Implicit(self.op_marker_counter);
        self.op_marker_counter += 1;
        marker
    }

    fn add_node(
        &mut self,
        aid: AbstractNodeId,
        node_abstract: S::NodeAbstract,
        from_shape_query: bool,
    ) {
        let node_key = self.graph.add_node(node_abstract);
        self.node_keys_to_aid.insert(node_key, aid);
        if from_shape_query {
            self.node_may_originate_from_shape_query.insert(aid);
        } else {
            // TODO: might be able to remove the AID from shape query.
        }
    }

    fn add_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge_abstract: S::EdgeAbstract,
        from_shape_query: bool,
    ) -> Result<(), OperationBuilderError> {
        let source_key = self
            .node_keys_to_aid
            .get_right(&source)
            .ok_or(OperationBuilderError::NotFoundAid(source))?;
        let target_key = self
            .node_keys_to_aid
            .get_right(&target)
            .ok_or(OperationBuilderError::NotFoundAid(target))?;

        self.graph.add_edge(*source_key, *target_key, edge_abstract);

        if from_shape_query {
            self.edge_may_originate_from_shape_query
                .insert((source, target));
        } else {
            // TODO: might be able to remove the AID.
        }
        Ok(())
    }

    fn set_node_av(
        &mut self,
        aid: AbstractNodeId,
        node_abstract: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        let node_key = self
            .node_keys_to_aid
            .get_right(&aid)
            .ok_or(OperationBuilderError::NotFoundAid(aid))?;
        self.graph.set_node_attr(*node_key, node_abstract);
        Ok(())
    }

    fn contains_aid(&self, aid: &AbstractNodeId) -> bool {
        self.node_keys_to_aid.contains_right(aid)
    }

    fn contains_edge(&self, source: &AbstractNodeId, target: &AbstractNodeId) -> bool {
        let Some(source_key) = self.node_keys_to_aid.get_right(source) else {
            return false;
        };
        let Some(target_key) = self.node_keys_to_aid.get_right(target) else {
            return false;
        };
        self.graph
            .get_edge_attr((*source_key, *target_key))
            .is_some()
    }

    pub fn node_av_of_aid(&self, aid: &AbstractNodeId) -> Option<&S::NodeAbstract> {
        let node_key = self.node_keys_to_aid.get_right(aid)?;
        self.graph.get_node_attr(*node_key)
    }

    pub fn edge_av_of_aid(
        &self,
        source: &AbstractNodeId,
        target: &AbstractNodeId,
    ) -> Option<&S::EdgeAbstract> {
        let source_key = self.node_keys_to_aid.get_right(source)?;
        let target_key = self.node_keys_to_aid.get_right(target)?;
        self.graph.get_edge_attr((*source_key, *target_key))
    }

    /// Modifies all mappings so that all mentions of `old_aid` are replaced with `new_aid`.
    fn rename_aid(
        &mut self,
        old_aid: AbstractNodeId,
        new_aid: AbstractNodeId,
    ) -> Result<(), OperationBuilderError> {
        // if we already have the new AID, return error
        if self.node_keys_to_aid.contains_right(&new_aid) {
            bail!(OperationBuilderError::AlreadyExistsAid(new_aid));
        }
        // Update the mappings
        if let Some(node_key) = self.node_keys_to_aid.remove_right(&old_aid) {
            self.node_keys_to_aid.insert(node_key, new_aid);
        } else {
            bail!(OperationBuilderError::NotFoundAid(old_aid));
        }

        // Update the shape query sets
        if self.node_may_originate_from_shape_query.remove(&old_aid) {
            self.node_may_originate_from_shape_query.insert(new_aid);
        }
        // edges too
        self.edge_may_originate_from_shape_query = self
            .edge_may_originate_from_shape_query
            .iter()
            .map(|&(src, dst)| {
                let new_src = if src == old_aid { new_aid } else { src };
                let new_dst = if dst == old_aid { new_aid } else { dst };
                (new_src, new_dst)
            })
            .collect();

        // Update writes
        if let Some(node_av) = self.node_may_be_written_to.remove(&old_aid) {
            self.node_may_be_written_to.insert(new_aid, node_av);
        }
        self.edge_may_be_written_to = self
            .edge_may_be_written_to
            .iter()
            .map(|(&(src, dst), edge_av)| {
                let new_src = if src == old_aid { new_aid } else { src };
                let new_dst = if dst == old_aid { new_aid } else { dst };
                ((new_src, new_dst), edge_av.clone())
            })
            .collect();

        Ok(())
    }

    fn diverge(&mut self) {
        // set our diverged flag
        if !self.has_diverged {
            self.has_diverged = true;
        }
    }

    /// Returns the abstract changes from applying the op as well as the new AIDs
    fn interpret_op(
        &mut self,
        op_ctx: &OperationContext<S>,
        marker: Option<AbstractOperationResultMarker>,
        op: AbstractOperation<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<
        (
            AbstractOperationArgument,
            IntermediateStateAbstractOutputResult,
        ),
        OperationBuilderError,
    > {
        // if we've diverged, issue a warning
        if self.has_diverged {
            log::warn!(
                "Trying to issue new instruction with name {marker:?} after path has diverged. This may lead to unexpected results regarding available node names."
            );
        }
        let param = op.parameter();
        let (subst, abstract_arg) = self.get_substitution(&param, args)?;

        // now apply op and store result
        let operation_output = {
            let mut gws = GraphWithSubstitution::new(&mut self.graph, &subst);
            op.apply_abstract(op_ctx, &mut gws)
                .change_context(OperationBuilderError::AbstractApplyOperationError2)?
        };
        let output = self.handle_abstract_output_changes(marker, operation_output)?;

        Ok((abstract_arg, output))
    }

    fn interpret_builtin_query(
        &mut self,
        query: &S::BuiltinQuery,
        args: Vec<AbstractNodeId>,
    ) -> Result<AbstractOperationArgument, OperationBuilderError> {
        let param = query.parameter();
        let (subst, abstract_arg) = self.get_substitution(&param, args)?;
        // now apply the query and store result
        let mut gws = GraphWithSubstitution::new(&mut self.graph, &subst);
        query.apply_abstract(&mut gws);
        Ok(abstract_arg)
    }

    /// Returns the newly added AIDs
    fn handle_abstract_output_changes(
        &mut self,
        marker: Option<AbstractOperationResultMarker>,
        operation_output: AbstractOperationOutput<S>,
    ) -> Result<IntermediateStateAbstractOutputResult, OperationBuilderError> {
        // go over new nodes
        let mut new_aids = Vec::new();
        for (node_marker, node_key) in operation_output.new_nodes {
            if let Some(op_marker) = marker {
                let aid = AbstractNodeId::DynamicOutputMarker(op_marker, node_marker);
                // TODO: override the may_come_from_shape_query set here! remove the node - it's a non-shape-query node.
                self.node_keys_to_aid.insert(node_key, aid);
                new_aids.push(aid);
            } else {
                // we don't keep track of it, so better remove it from the graph
                self.graph.remove_node(node_key);
            }
        }
        let mut removed_aids = Vec::new();
        for node_key in &operation_output.removed_nodes {
            // remove the node from the mapping
            if let Some(removed_aid) = self.node_keys_to_aid.remove_left(node_key) {
                removed_aids.push(removed_aid);
            }
        }

        // collect changes
        // TODO: What is a good idea regarding changes abstract values?
        //  I think it's a good idea to just propagate what we know for a fact _could_ be written (but in its most precise form).
        //  If instead we said "merge it with the current value", then we make it potentially join with the parameter.
        for (key, node_abstract) in operation_output.changed_abstract_values_nodes {
            let Ok(aid) = self.get_aid_from_key(&key) else {
                // note: this happens because our GraphWithSubstitution operation output generation does
                //  not skip changed nodes that were also deleted.
                // this is not really an error, but would be nice if we could avoid by doing that.
                log::warn!("internal error: changed node not found in mapping");
                continue;
            };
            self.node_may_be_written_to.insert(aid, node_abstract);
        }
        for ((source, target), edge_abstract) in operation_output.changed_abstract_values_edges {
            let Ok(source_aid) = self.get_aid_from_key(&source) else {
                // see above for why we may enter this and why it's fine
                log::warn!("internal error: changed edge source not found in mapping");
                continue;
            };
            let Ok(target_aid) = self.get_aid_from_key(&target) else {
                log::warn!("internal error: changed edge target not found in mapping");
                continue;
            };
            self.edge_may_be_written_to
                .insert((source_aid, target_aid), edge_abstract);
        }

        Ok(IntermediateStateAbstractOutputResult {
            new_aids,
            removed_aids,
        })
    }

    fn get_substitution(
        &self,
        param: &OperationParameter<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(ParameterSubstitution, AbstractOperationArgument), OperationBuilderError> {
        let selected_inputs = args
            .iter()
            .map(|aid| self.get_key_from_aid(aid))
            .collect::<Result<Vec<_>, _>>()
            .change_context(OperationBuilderError::SelectedInputsNotFoundAid)?;
        let subst = get_substitution(&self.graph, param, &selected_inputs)
            .change_context(OperationBuilderError::SubstitutionErrorNew)?;
        let subst_to_aid = subst.mapping.iter().map(|(subst, key)| {
            let aid = self.get_aid_from_key(key)
                .change_context(OperationBuilderError::InternalError("node key should be in mapping, because all node keys from the abstract graph should be in the mapping"))
                .unwrap();
            (*subst, aid)
        }).collect();

        let abstract_arg = AbstractOperationArgument {
            selected_input_nodes: args,
            subst_to_aid,
        };

        Ok((subst, abstract_arg))
    }

    fn get_key_from_aid(&self, aid: &AbstractNodeId) -> Result<NodeKey, OperationBuilderError> {
        self.node_keys_to_aid
            .get_right(aid)
            .cloned()
            .ok_or(report!(OperationBuilderError::NotFoundAid(*aid)))
    }

    fn get_aid_from_key(&self, key: &NodeKey) -> Result<AbstractNodeId, OperationBuilderError> {
        self.node_keys_to_aid.get_left(key).cloned().ok_or(report!(
            OperationBuilderError::InternalError("could not find node key")
        ))
    }

    fn as_param_for_shape_query(&self) -> (OperationParameter<S>, AbstractOperationArgument) {
        let param_graph = self.graph.clone();

        let mut all_node_keys = param_graph
            .node_attr_map
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        all_node_keys.sort_unstable(); // sort to ensure deterministic order

        let mut node_keys_to_subst: BiMap<NodeKey, SubstMarker> = BiMap::new();
        let mut explicit_input_nodes = Vec::new();
        let mut aid_args = Vec::new();
        let mut subst_to_aid = HashMap::new();
        for key in all_node_keys {
            let subst = SubstMarker::from(format!("{key:?}"));
            node_keys_to_subst.insert(key, subst);
            explicit_input_nodes.push(subst);
            // collect the AID for this key
            let aid = self.get_aid_from_key(&key).unwrap();
            aid_args.push(aid);
            subst_to_aid.insert(subst, aid);
        }

        let abstract_args = AbstractOperationArgument {
            selected_input_nodes: aid_args,
            subst_to_aid,
        };

        (
            OperationParameter {
                explicit_input_nodes,
                parameter_graph: param_graph,
                node_keys_to_subst,
            },
            abstract_args,
        )
    }
}

impl<S: Semantics<NodeAbstract: Debug, EdgeAbstract: Debug>> IntermediateState<S> {
    pub fn dot_with_aid(&self) -> String {
        struct PrettyAid<'a>(&'a AbstractNodeId);

        impl Debug for PrettyAid<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.0 {
                    AbstractNodeId::ParameterMarker(subst) => write!(f, "P({})", subst.0),
                    AbstractNodeId::DynamicOutputMarker(marker, node_marker) => {
                        let op_marker = match marker {
                            AbstractOperationResultMarker::Custom(c) => c,
                            AbstractOperationResultMarker::Implicit(..) => "<unnamed>",
                        };
                        write!(f, "O({}, {})", op_marker, node_marker.0)
                    }
                    AbstractNodeId::Named(name) => {
                        write!(f, "{name:?}")
                    }
                }
            }
        }

        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|_, (_src, _dst, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{dbg_attr_replaced}\"")
                },
                &|_, (node, _)| {
                    let aid = self
                        .node_keys_to_aid
                        .get_left(&node)
                        .expect("NodeKey not found in node_keys_to_aid");
                    let aid = PrettyAid(aid);
                    let aid = format!("{aid:?}");
                    let aid_replaced = aid.escape_debug();
                    let av = self
                        .graph
                        .get_node_attr(node)
                        .expect("NodeKey not found in graph");
                    let dbg_attr_format = format!("{av:?}");
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();

                    // format!("label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                    // format!("label = \"{dbg_attr_replaced}\", xlabel = \"{aid_replaced}\"")
                    format!("shape=Mrecord, label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                }
            )
        )
    }

    pub fn dot_with_aid_custom_aid_format(
        &self,
        fmt_aid: impl Fn(AbstractNodeId) -> String,
    ) -> String {
        format!(
            "{:?}",
            Dot::with_attr_getters(&self.graph.graph, &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel], &|_, (_src, _dst, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{dbg_attr_replaced}\"")
                }, &|_, (node, _)| {
                    let aid = self
                        .node_keys_to_aid
                        .get_left(&node)
                        .expect("NodeKey not found in node_keys_to_aid");
                    let aid = fmt_aid(*aid);
                    let aid_replaced = aid.escape_debug();
                    let av = self
                        .graph
                        .get_node_attr(node)
                        .expect("NodeKey not found in graph");
                    let dbg_attr_format = format!("{av:?}");
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();

                    // format!("label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                    // format!("label = \"{dbg_attr_replaced}\", xlabel = \"{aid_replaced}\"")
                    format!("shape=Mrecord, label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                })
        )
    }

    /// Turns AIDs into "name" or "map.name".
    pub fn dot_with_aid_with_dot_syntax(&self) -> String {
        self.dot_with_aid_custom_aid_format(|aid| match aid {
            AbstractNodeId::ParameterMarker(subst) => format!("{}", subst.0),
            AbstractNodeId::DynamicOutputMarker(
                AbstractOperationResultMarker::Custom(marker),
                node_marker,
            ) => {
                format!("{}.{}", marker, node_marker.0)
            }
            AbstractNodeId::Named(name) => name.0.to_string(),
            _ => "unknown".to_string(),
        })
    }

    /// Uses tables instead of Mrecord shapes. Uses "name" and "map.name" syntax for AIDs.
    pub fn dot_with_aid_table_based_with_color_names(
        &self,
        node_av_color_name: &str,
        edge_str_color_name: &str,
    ) -> String {
        let fmt_aid = |aid: AbstractNodeId| match aid {
            AbstractNodeId::ParameterMarker(subst) => format!("{}", subst.0),
            AbstractNodeId::DynamicOutputMarker(
                AbstractOperationResultMarker::Custom(marker),
                node_marker,
            ) => {
                format!("{}.{}", marker, node_marker.0)
            }
            AbstractNodeId::Named(name) => name.0.to_string(),
            _ => "unknown".to_string(),
        };
        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|_, (_src, _dst, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    // color attrs below are quite the hack for typst-plugin.
                    // TODO: move this entire function to typst plugin?
                    let color_attr = if dbg_attr_format.contains('"') {
                        format!(r#"fontcolor="{edge_str_color_name}""#)
                    } else {
                        format!(r#"fontcolor="{node_av_color_name}""#)
                    };
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("{color_attr} label = \"{dbg_attr_replaced}\"")
                },
                &|_, (node, _)| {
                    let aid = self
                        .node_keys_to_aid
                        .get_left(&node)
                        .expect("NodeKey not found in node_keys_to_aid");
                    let aid = fmt_aid(*aid);
                    let aid_replaced = aid.escape_debug();
                    let av = self
                        .graph
                        .get_node_attr(node)
                        .expect("NodeKey not found in graph");
                    let dbg_attr_format = format!("{av:?}");
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();

                    // format!("label = \"{aid_replaced}|{dbg_attr_replaced}\"")
                    // format!("label = \"{dbg_attr_replaced}\", xlabel = \"{aid_replaced}\"")
                    let table_label = format!(
                        r#"<
    <table style="rounded" cellpadding="5" cellspacing="0"  cellborder="0" border="1">
        <tr><td border="1" sides="R">{aid_replaced}</td><td><font color="{node_av_color_name}">{dbg_attr_replaced}</font></td></tr>
    </table>
>"#
                    );
                    format!(
                        r#"shape=plaintext, margin="0" height="0" width="0" label={table_label}"#
                    )
                }
            )
        )
    }
}

struct MergeStatesResult<S: Semantics> {
    merged_state: IntermediateState<S>,
    /// The AIDs that are present in the true state but not in the merged state.
    missing_from_true: HashSet<AbstractNodeId>,
    /// The AIDs that are present in the false state but not in the merged state.
    missing_from_false: HashSet<AbstractNodeId>,
}

/// Takes two intermediate states and computes the smallest subgraph and most general abstract values
/// such that the resulting state is a sound approximation of the two states.
///
/// Nodes are only merged if they have exactly the same abstract node ID in both branches.
///
/// Also, abstract type merging is fallible, so if two nodes are incompatible with each other, they don't appear in the resulting state.
///
/// # Example:
/// 1. Initial state is `P(0)|String`
/// 2. We branch:
/// 2a. True branch ends with graph `P(0)|String -> O(c1)|String -> O(c2)|String`
/// 2b. False branch ends with graph `P(0)|String -> O(c1)|Integer -> O(c3)|Object`
/// 3. The resulting state will be `P(0)|String -> O(c1)|Object`
///
/// Note how the second added node from the true branch is not present *with the same name* in the false branch, and
/// therefore is not present in the resulting state. Same for `O(c3)` from the false branch.
/// Also note how the node that exists in both branches, `O(c1)`, is present in the resulting state with the
/// least common supertype of the two branches, which is `Object` in this case.
fn merge_states_result<S: Semantics>(
    state_true: &IntermediateState<S>,
    state_false: &IntermediateState<S>,
) -> MergeStatesResult<S> {
    // check if either is diverged, if so, copy the other
    // TODO: warning, if divergence can ever be 'recovered', then we need to make sure the effects of
    //  the diverged branch are not lost _up to divergence point_.
    if state_true.has_diverged {
        return MergeStatesResult {
            merged_state: state_false.clone(),
            // everything from true is "missing". these IDs will get a ForgetAid inserted into the true branch.
            // since true has diverged, that shouldn't be a problem.
            missing_from_true: state_true
                .node_keys_to_aid
                .right_values()
                .copied()
                .collect(),
            missing_from_false: HashSet::new(),
        };
    }
    if state_false.has_diverged {
        return MergeStatesResult {
            merged_state: state_true.clone(),
            // everything from false is "missing". these IDs will get a ForgetAid inserted into the false branch.
            // since false has diverged, that shouldn't be a problem.
            missing_from_true: HashSet::new(),
            missing_from_false: state_false
                .node_keys_to_aid
                .right_values()
                .copied()
                .collect(),
        };
    }

    let mut new_state = IntermediateState::new();

    let mut common_aids = HashSet::new();
    // First, collect all AIDs that are present in both states.
    for aid in state_true.node_keys_to_aid.right_values() {
        if state_false.node_keys_to_aid.contains_right(aid) {
            common_aids.insert(*aid);
        }
    }

    // Now, for each common AID, we need to merge the nodes and info from both states.
    for aid in common_aids {
        let key_true = *state_true
            .node_keys_to_aid
            .get_right(&aid)
            .expect("internal error: AID should be in mapping");
        let key_false = *state_false
            .node_keys_to_aid
            .get_right(&aid)
            .expect("internal error: AID should be in mapping");

        // Get the abstract values from both states.
        let av_true = state_true
            .graph
            .get_node_attr(key_true)
            .expect("internal error: Key should be in graph");
        let av_false = state_false
            .graph
            .get_node_attr(key_false)
            .expect("internal error: Key should be in graph");

        // Merge the abstract values.
        let Some(merged_av) = S::join_nodes(av_true, av_false) else {
            // If we cannot merge the abstract values, we skip this AID.
            continue;
        };

        // Add the merged node to the new state.
        let new_key = new_state.graph.add_node(merged_av);
        new_state.node_keys_to_aid.insert(new_key, aid);
        // Keep track of the node originating from a shape query...
        if state_true
            .node_may_originate_from_shape_query
            .contains(&aid)
            || state_false
                .node_may_originate_from_shape_query
                .contains(&aid)
        {
            new_state.node_may_originate_from_shape_query.insert(aid);
        }
        // ... as well as the written types.
        // We take the join-union of the written types from both states.
        let written_av_true = state_true.node_may_be_written_to.get(&aid).cloned();
        let written_av_false = state_false.node_may_be_written_to.get(&aid).cloned();
        let merged_written_av = match (written_av_true, written_av_false) {
            (Some(av_true), Some(av_false)) => {
                // Note: we need this to be some, since we've already inserted the node in the new graph.
                // for more detail, see the comment in the edges section below.
                Some(S::join_nodes(&av_true, &av_false).expect(
                    "client semantics error: expected to be able to merge written node attributes",
                ))
            }
            (Some(av_true), None) => Some(av_true),
            (None, Some(av_false)) => Some(av_false),
            (None, None) => None,
        };
        if let Some(merged_av) = merged_written_av {
            new_state.node_may_be_written_to.insert(aid, merged_av);
        }
    }

    // Now we merge the edges.
    for (from_key_true, to_key_true, _) in state_true.graph.graph.all_edges() {
        let from_aid = state_true
            .node_keys_to_aid
            .get_left(&from_key_true)
            .expect("internal error: from key should be in mapping");
        let to_aid = state_true
            .node_keys_to_aid
            .get_left(&to_key_true)
            .expect("internal error: to key should be in mapping");
        let Some(from_key_merged) = new_state.node_keys_to_aid.get_right(from_aid) else {
            // If the from AID has not been merged, we skip this edge.
            continue;
        };
        let Some(to_key_merged) = new_state.node_keys_to_aid.get_right(to_aid) else {
            // If the to AID has not been merged, we skip this edge.
            continue;
        };
        let av_true = state_true
            .graph
            .get_edge_attr((from_key_true, to_key_true))
            .expect("internal error: edge should be in graph");

        // Skip edges whose endpoints are not in the common AIDs.
        // because of the above new_state let else check, this should always succeed, though.
        let Some(from_key_false) = state_false.node_keys_to_aid.get_right(from_aid) else {
            continue;
        };
        let Some(to_key_false) = state_false.node_keys_to_aid.get_right(to_aid) else {
            continue;
        };

        // Check if the edge exists in the false state.
        let Some(av_false) = state_false
            .graph
            .get_edge_attr((*from_key_false, *to_key_false))
        else {
            // If the edge does not exist in the false state, we skip it.
            continue;
        };
        let Some(merged_av) = S::join_edges(av_true, av_false) else {
            // If we cannot merge the edges, we skip this edge.
            continue;
        };
        // Add the merged edge to the new state.
        new_state
            .graph
            .add_edge(*from_key_merged, *to_key_merged, merged_av);
        // Keep track of the edge originating from a shape query.
        let edge = (*from_aid, *to_aid);
        if state_true
            .edge_may_originate_from_shape_query
            .contains(&edge)
            || state_false
                .edge_may_originate_from_shape_query
                .contains(&edge)
        {
            new_state.edge_may_originate_from_shape_query.insert(edge);
        }

        let written_av_true = state_true
            .edge_may_be_written_to
            .get(&(*from_aid, *to_aid))
            .cloned();
        let written_av_false = state_false
            .edge_may_be_written_to
            .get(&(*from_aid, *to_aid))
            .cloned();
        let merged_written_av = match (written_av_true, written_av_false) {
            (Some(av_true), Some(av_false)) => {
                // Note: this must be Some, because we have the edge in our merged graph for a fact.
                // If we were to ignore it *just for edge_may_be_written_to* if the written values could not be merged,
                // we'd unsoundly skip returning information about potential changes to the edge.
                // I.e., we expect client semantics to support written-av merges if the branch merge succeeded in the first place.
                // this should generally be the case.
                // Update: Below cannot panic for transitive client type systems.
                //  since we were able to join the edge, that means the inferred AVs for both branches
                //  were compatible. We *know* that a written_av *must be* a subtype of the inferred AVs,
                //  since whenever we write we join that write to the current abstract graph AV.
                //  So, we know that some super type of av_true and some super type of av_false
                //  were joinable, and in a transitive type system, that means that av_true and av_false
                //  must be joinable as well. QED.
                Some(S::join_edges(&av_true, &av_false).expect(
                    "client semantics error: expected to be able to merge written edge attributes",
                ))
            }
            (Some(av_true), None) => Some(av_true),
            (None, Some(av_false)) => Some(av_false),
            (None, None) => None,
        };
        if let Some(merged_av) = merged_written_av {
            new_state
                .edge_may_be_written_to
                .insert((*from_aid, *to_aid), merged_av);
        }
    }

    // which AIDs are actually present in the merged state, taking into account everything (names, type join)
    let final_merged_state_aids = new_state
        .node_keys_to_aid
        .right_values()
        .cloned()
        .collect::<HashSet<_>>();

    MergeStatesResult {
        merged_state: new_state,
        missing_from_true: state_true
            .node_keys_to_aid
            .right_values()
            .cloned()
            .filter(|aid| !final_merged_state_aids.contains(aid))
            .collect(),
        missing_from_false: state_false
            .node_keys_to_aid
            .right_values()
            .cloned()
            .filter(|aid| !final_merged_state_aids.contains(aid))
            .collect(),
    }
}
