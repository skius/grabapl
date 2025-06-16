use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::Peekable;
use std::marker::PhantomData;
use std::slice::Iter;
use thiserror::Error;
use crate::{Graph, NodeKey, OperationContext, OperationId, SubstMarker};
use crate::graph::operation::query::{GraphShapeQuery, ShapeNodeIdentifier};
use crate::graph::operation::user_defined::{AbstractNodeId, AbstractOperationResultMarker, QueryInstructions, UserDefinedOperation};
use crate::graph::pattern::{AbstractOutputNodeMarker, OperationParameter};
use crate::graph::semantics::{AbstractGraph, SemanticsClone};

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

pub enum Instruction<S: SemanticsClone> {
    Builtin(S::BuiltinOperation),
    FromOperationId(OperationId),
    Recurse,
}

#[derive(derive_more::Debug)]
enum BuilderInstruction<S: SemanticsClone> {
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
    #[debug("StartShapeQuery({_0:?})")]
    StartShapeQuery(AbstractOperationResultMarker),
    #[debug("EndQuery")]
    EndQuery,
    #[debug("ExpectShapeNode({_0:?}, ???)")]
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:?}, {_1:?}, ???)")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
    #[debug("AddNamedInstruction({_0:?}, ???, args: {_2:?})")]
    AddNamedInstruction(AbstractOperationResultMarker, Instruction<S>, Vec<AbstractNodeId>),
    #[debug("AddInstruction(???, args: {_1:?})")]
    AddInstruction(Instruction<S>, Vec<AbstractNodeId>),
}

#[derive(Error, Debug)]
pub enum OperationBuilderError {
    #[error("Expected a new unique subst marker, found repeat: {0}")]
    ReusedSubstMarker(SubstMarker),
    #[error("Expected an existing subst marker, but {0} was not found")]
    NotFoundSubstMarker(SubstMarker),
    #[error("Expected a new unique subst marker, found repeat: {0:?}")]
    ReusedShapeIdent(ShapeNodeIdentifier),
    #[error("Cannot call this while in a query context")]
    InvalidInQuery,
    #[error("Already visited the {0} branch of the active query")]
    AlreadyVisitedBranch(bool),
}

pub struct OperationBuilder<'a, S: SemanticsClone> {
    op_ctx: &'a OperationContext<S>,
    instructions: Vec<BuilderInstruction<S>>,
}

// TODO: all message adding, validate all args by building temp graph
impl<'a, S: SemanticsClone<BuiltinQuery: Clone, BuiltinOperation: Clone>> OperationBuilder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Self {
            instructions: Vec::new(),
            op_ctx,
        }
    }

    pub fn expect_parameter_node(
        &mut self,
        marker: SubstMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectParameterNode(marker, node));
        Ok(())
    }

    pub fn expect_context_node(
        &mut self,
        marker: SubstMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectContextNode(marker, node));
        // TODO: check if subst marker does not exist yet
        Ok(())
    }

    pub fn expect_parameter_edge(
        &mut self,
        source_marker: SubstMarker,
        target_marker: SubstMarker,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        self.instructions.push(BuilderInstruction::ExpectParameterEdge(
            source_marker, target_marker, edge,
        ));
        // TODO: check if both subst markers are valid
        Ok(())
    }
    
    pub fn start_query(
        &mut self,
        query: S::BuiltinQuery,
        args: Vec<AbstractNodeId>
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::StartQuery(query, args));
        Ok(())
    }
    
    pub fn enter_true_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EnterTrueBranch);
        Ok(())
    }
    
    pub fn enter_false_branch(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EnterFalseBranch);
        Ok(())
    }

    // TODO: get rid of AbstractOperationResultMarker requirement. Either completely or make it optional and autogenerate one.
    //  How to specify which shape node? ==> the shape node markers should be unique per path
    pub fn start_shape_query(&mut self, op_marker: AbstractOperationResultMarker) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::StartShapeQuery(op_marker));
        Ok(())
    }

    pub fn end_query(&mut self) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::EndQuery);
        Ok(())
    }

    // TODO: should expect_*_node really expect a marker? maybe it should instead return a marker?
    //  it could also take an Option<Marker> so that it can autogenerate one if it's none so the caller doesn't have to deal with it.
    pub fn expect_shape_node(
        &mut self,
        marker: AbstractOutputNodeMarker,
        node: S::NodeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO: check that any shape nodes are not free floating. maybe this should be in a GraphShapeQuery validator?
        self.instructions.push(BuilderInstruction::ExpectShapeNode(marker, node));
        Ok(())
    }

    pub fn expect_shape_edge(
        &mut self,
        source: AbstractNodeId,
        target: AbstractNodeId,
        edge: S::EdgeAbstract,
    ) -> Result<(), OperationBuilderError> {
        // TODO:
        self.instructions.push(BuilderInstruction::ExpectShapeEdge(source, target, edge));
        Ok(())
    }

    pub fn add_named_instruction(
        &mut self,
        name: AbstractOperationResultMarker,
        instruction: Instruction<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // TODO
        self.instructions.push(BuilderInstruction::AddNamedInstruction(name, instruction, args));
        Ok(())
    }
    
    pub fn add_instruction(
        &mut self,
        instruction: Instruction<S>,
        args: Vec<AbstractNodeId>,
    ) -> Result<(), OperationBuilderError> {
        // todo!()
        self.instructions.push(BuilderInstruction::AddInstruction(instruction, args));
        Ok(())
    }
    
    pub fn build(self, self_op_id: SubstMarker) -> Result<UserDefinedOperation<S>, OperationBuilderError> {
        // Here we would typically finalize the operation and return it.
        // For now, we just return Ok to indicate success.

        IntermediateStateBuilder::run(&self.instructions, self.op_ctx)?;

        todo!("build not implemented yet");
    }
}

impl<'a, S: SemanticsClone<NodeAbstract: Debug, EdgeAbstract: Debug>> OperationBuilder<'a, S> {
    /// Visualizes the current state of the operation builder.
    /// Provides context on the current nest level as well as the DOT representation of the graph
    /// at the current cursor.
    pub fn show_state(&self) -> String {
        let (g, subst_to_node_keys) = self.build_debug_graph_at_current_point();
        let dot = g.dot();


        let mut result = String::new();

        result.push_str(&"Current Operation Builder State:\n".to_string());
        result.push_str(&"Graph at current point:\n".to_string());
        result.push_str(&dot);
        result
    }

    fn build_debug_graph_at_current_point(
        &self,
    ) -> (Graph<S::NodeAbstract, S::EdgeAbstract>, HashMap<SubstMarker, NodeKey>) {
        let mut g = Graph::new();
        let mut subst_to_node_keys: HashMap<SubstMarker, NodeKey> = HashMap::new();

        for instruction in &self.instructions {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(*marker, key);
                },
                BuilderInstruction::ExpectContextNode(marker, node) => {
                    let key = g.add_node(node.clone());
                    subst_to_node_keys.insert(*marker, key);
                },
                BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge) => {
                    let source_key = *subst_to_node_keys
                        .get(source_marker)
                        .expect("Source marker not found in subst_to_node_keys");
                    let target_key = *subst_to_node_keys
                        .get(target_marker)
                        .expect("Target marker not found in subst_to_node_keys");
                    g.add_edge(
                        source_key,
                        target_key,
                        edge.clone(),
                    );
                },
                _ => {
                    eprintln!("Skipping instruction");
                }
            }
        }

        (g, subst_to_node_keys)
    }
}

struct NodeMetadata<S: SemanticsClone> {
    abstract_node_id: AbstractNodeId, // how do we refer to this node?
    _phantom: PhantomData<S>,
}

struct IntermediateState<S: SemanticsClone> {
    graph: AbstractGraph<S>,
    node_metadata: HashMap<NodeKey, NodeMetadata<S>>,
}

struct IntermediateStateBuilder<'a, S: SemanticsClone> {
    instructions: Peekable<Iter<'a, BuilderInstruction<S>>>,
    op_ctx: &'a OperationContext<S>,
}

use super::user_defined::Instruction as UDInstruction;

#[derive(derive_more::Debug)]
enum IntermediateInstruction<S: SemanticsClone> {
    #[debug("Final({_0:#?})")]
    Final(UDInstruction<S>),
    #[debug("GraphShapeQuery({_0:#?}, {_1:#?}, {_2:#?})")]
    GraphShapeQuery(AbstractOperationResultMarker, Vec<GraphShapeQueryInstruction<S>>, IntermediateQueryInstructions<S>),
    #[debug("BuiltinQuery(???, {_1:#?}, {_2:#?})")]
    BuiltinQuery(S::BuiltinQuery, Vec<AbstractNodeId>, IntermediateQueryInstructions<S>),
}

#[derive(derive_more::Debug)]
struct IntermediateQueryInstructions<S: SemanticsClone> {
    #[debug("[{}]", true_branch.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    true_branch: Vec<(Option<AbstractOperationResultMarker>, IntermediateInstruction<S>)>,
    #[debug("[{}]", false_branch.iter().map(|(opt, inst)| format!("({opt:#?}, {:#?})", inst)).collect::<Vec<_>>().join(", "))]
    false_branch: Vec<(Option<AbstractOperationResultMarker>, IntermediateInstruction<S>)>,
}

#[derive(derive_more::Debug)]
enum GraphShapeQueryInstruction<S: SemanticsClone> {
    #[debug("ExpectShapeNode({_0:#?})")]
    ExpectShapeNode(AbstractOutputNodeMarker, S::NodeAbstract),
    #[debug("ExpectShapeEdge({_0:#?}, {_1:#?})")]
    ExpectShapeEdge(AbstractNodeId, AbstractNodeId, S::EdgeAbstract),
}

// TODO: maybe this is not *intermediate* but actually the final state as well potentially?
impl<'a, S: SemanticsClone<BuiltinOperation: Clone, BuiltinQuery: Clone>> IntermediateStateBuilder<'a, S> {
    fn run(
        instructions: &'a [BuilderInstruction<S>],
        op_ctx: &'a OperationContext<S>,
    ) -> Result<IntermediateState<S>, OperationBuilderError> {
        /*
        General idea:
        Whenever we see start_query (or start_shape_query), we push a query state onto a stack.
        When we see

        */


        //
        // enum QueryBranchState {
        //     // if we haven't encountered an enter_*_branch message yet
        //     NoBranch,
        //     TrueBranch,
        //     FalseBranch,
        // }
        // struct QueryState<S: SemanticsClone> {
        //     true_instructions: Vec<UDInstruction<S>>,
        //     false_instructions: Vec<UDInstruction<S>>,
        //     current_branch: QueryBranchState,
        // }
        //
        // enum StackState {
        //
        // }
        //
        // #[derive(Clone, Copy, Debug)]
        // enum State {
        //     BuildingParameterGraph,
        //     ExpectingInstruction,
        //     BuildingQuery,
        //     BuildingShapeQuery,
        // }
        //
        // let mut current_query_branch_state: Option<QueryBranchState> = None;
        // let mut current_state = State::BuildingParameterGraph;
        //
        // let mut operation_parameter = OperationParameter::<S> {
        //     explicit_input_nodes: Vec::new(),
        //     parameter_graph: AbstractGraph::<S>::new(),
        //     subst_to_node_keys: HashMap::new(),
        //     node_keys_to_subst: HashMap::new(),
        // };
        //
        // // unsure if we need these.
        // let mut abstract_graph = AbstractGraph::<S>::new();
        // let mut aid_to_node_keys: HashMap<AbstractNodeId, NodeKey> = HashMap::new();
        // let mut node_keys_to_aid: HashMap<NodeKey, AbstractNodeId> = HashMap::new();
        //
        // // build a partial UserDefinedOperation.
        // // This UserDefinedOperation is what we will use to build the partial abstract graph at that state.
        //
        // // This is a stack of instruction vectors.
        // let mut instructions_vec_stack: Vec<Vec<UDInstruction<S>>> = Vec::new();
        // // We push any new instructions onto this vector.
        // let mut current_instructions_vec: Vec<UDInstruction<S>> = Vec::new();
        //
        //
        // for instruction in instructions {
        //     let mut next_state = current_state;
        //     match (current_state, instruction) {
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectParameterNode(marker, node_abstract)) => {
        //             if operation_parameter.subst_to_node_keys.contains_key(marker) {
        //                 return Err(OperationBuilderError::ReusedSubstMarker(*marker));
        //             }
        //             let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
        //             operation_parameter.subst_to_node_keys.insert(*marker, key);
        //             operation_parameter.node_keys_to_subst.insert(key, *marker);
        //             operation_parameter.explicit_input_nodes.push(*marker);
        //         }
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectContextNode(marker, node_abstract)) => {
        //             if operation_parameter.subst_to_node_keys.contains_key(marker) {
        //                 return Err(OperationBuilderError::ReusedSubstMarker(*marker));
        //             }
        //             let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
        //             operation_parameter.subst_to_node_keys.insert(*marker, key);
        //             operation_parameter.node_keys_to_subst.insert(key, *marker);
        //         }
        //         (State::BuildingParameterGraph, BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge_abstract)) => {
        //             let source_key = operation_parameter.subst_to_node_keys.get(source_marker)
        //                 .ok_or(OperationBuilderError::NotFoundSubstMarker(*source_marker))?;
        //             let target_key = operation_parameter.subst_to_node_keys.get(target_marker)
        //                 .ok_or(OperationBuilderError::NotFoundSubstMarker(*target_marker))?;
        //             operation_parameter.parameter_graph.add_edge(*source_key, *target_key, edge_abstract.clone());
        //         }
        //         (State::ExpectingInstruction | State::BuildingParameterGraph, BuilderInstruction::AddInstruction(instruction, args)) => {
        //             next_state = State::ExpectingInstruction;
        //
        //             match instruction {
        //                 Instruction::Builtin(builtin_op) => {
        //                     // Here we would typically apply the builtin operation to the abstract graph.
        //                     // For now, we just log it.
        //                     println!("Applying builtin operation: {:?} args: {args:?}", builtin_op);
        //
        //                     current_instructions_vec.push(UDInstruction::Builtin(builtin_op.clone(), args.clone()));
        //                 }
        //                 Instruction::FromOperationId(op_id) => {
        //                     // Here we would typically look up the operation by its ID and apply it.
        //                     // For now, we just log it.
        //                     println!("Applying operation with ID: {:?} args: {args:?}", op_id);
        //
        //                     current_instructions_vec.push(UDInstruction::Operation(op_id.clone(), args.clone()));
        //                 }
        //                 Instruction::Recurse => {
        //                     // This would typically mean we need to recurse into another operation.
        //                     // For now, we just log it.
        //                     println!("Recursing into self with args: {args:?}");
        //
        //                     // TODO: somehow denote 'self' instead of 0
        //                     current_instructions_vec.push(UDInstruction::Operation(0, args.clone()));
        //                 }
        //             }
        //         }
        //         (State::ExpectingInstruction | State::BuildingParameterGraph, BuilderInstruction::StartQuery(query, args)) => {
        //             next_state = State::BuildingQuery;
        //
        //             // Start a new query state
        //             current_query_branch_state = Some(QueryBranchState::NoBranch);
        //             instructions_vec_stack.push(current_instructions_vec);
        //             current_instructions_vec = Vec::new();
        //             // TODO: try continue here. the 'parsing' style below is easier, but this stack based version would allow easy caching.
        //         }
        //         _ => {}
        //     }
        //     current_state = next_state;
        // }





        let mut iter = instructions.iter().peekable();

        let op_parameter = Self::build_operation_parameter(&mut iter)?;

        let instructions = Self::build_many_instructions(&mut iter)?;

        dbg!(&instructions);

        todo!("Intermediate state builder not implemented yet");
    }

    fn build_many_instructions(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<Vec<(Option<AbstractOperationResultMarker>, IntermediateInstruction<S>)>, OperationBuilderError> {
        let mut instructions = Vec::new();

        while let Some(instr) = iter.peek() {
            // break on control flow instructions and don't consume
            if matches!(instr, BuilderInstruction::EndQuery | BuilderInstruction::EnterTrueBranch | BuilderInstruction::EnterFalseBranch) {
                break;
            }
            instructions.push(Self::build_instruction(iter)?);
        }
        Ok(instructions)
    }

    fn build_instruction(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<(Option<AbstractOperationResultMarker>, IntermediateInstruction<S>), OperationBuilderError> {
        let next_instruction = iter.peek().expect("should only be called when there is an instruction");
        match next_instruction {
            BuilderInstruction::AddNamedInstruction(_, instruction, args) | BuilderInstruction::AddInstruction(instruction, args) => {
                let name = if let BuilderInstruction::AddNamedInstruction(name, _, _) = next_instruction {
                    Some(*name)
                } else {
                    None
                };
                iter.next();

                match instruction {
                    Instruction::Builtin(builtin_op) => {
                        Ok((name, IntermediateInstruction::Final(UDInstruction::Builtin(builtin_op.clone(), args.clone()))))
                    }
                    Instruction::FromOperationId(op_id) => {
                        Ok((name, IntermediateInstruction::Final(UDInstruction::Operation(op_id.clone(), args.clone()))))
                    }
                    Instruction::Recurse => {
                        // TODO: somehow denote 'self' instead of 0
                        Ok((name, IntermediateInstruction::Final(UDInstruction::Operation(0, args.clone()))))
                    }
                }
            }
            BuilderInstruction::StartQuery(query, args) => {
                iter.next();
                // Start a new query state
                let query_instructions = Self::build_query_instruction(iter)?;
                Ok((None, IntermediateInstruction::BuiltinQuery(query.clone(), args.clone(), query_instructions)))
            }
            BuilderInstruction::StartShapeQuery(op_marker) => {
                iter.next();
                // Start a new shape query state
                let (gsq_instructions, branch_instructions) = Self::build_shape_query(iter, *op_marker)?;
                // Ok((Some(*op_marker), UDInstruction::ShapeQuery()))
                Ok((None, IntermediateInstruction::GraphShapeQuery(*op_marker, gsq_instructions, branch_instructions)))
            }
            _ => {
                Err(OperationBuilderError::InvalidInQuery)
            }
        }
    }

    fn build_shape_query(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
        operation_marker: AbstractOperationResultMarker,
    ) -> Result<(Vec<GraphShapeQueryInstruction<S>>, IntermediateQueryInstructions<S>), OperationBuilderError> {
        // we just consumed a StartShapeQuery instruction.

        // let mut gsq = GraphShapeQuery {
        //     parameter: OperationParameter {
        //         explicit_input_nodes: vec![],
        //         parameter_graph: Graph::new(),
        //         subst_to_node_keys: Default::default(),
        //         node_keys_to_subst: Default::default(),
        //     },
        //     expected_graph: Graph::new(),
        //     node_keys_to_shape_idents: Default::default(),
        //     shape_idents_to_node_keys: Default::default(),
        // };

        let mut gsq_instructions = vec![];

        let mut true_branch_instructions = None;
        let mut false_branch_instructions = None;

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::EnterTrueBranch => {
                    iter.next();
                    if true_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    true_branch_instructions = Some(Self::build_many_instructions(iter)?);
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(false));
                    }
                    false_branch_instructions = Some(Self::build_many_instructions(iter)?);
                }
                BuilderInstruction::EndQuery => {
                    iter.next();
                    break;
                }
                BuilderInstruction::ExpectShapeNode(marker, abstract_value) => {
                    iter.next();
                    // let shape_node_ident = marker.0.into();
                    // if gsq.shape_idents_to_node_keys.contains_key(&shape_node_ident) {
                    //     return Err(OperationBuilderError::ReusedShapeIdent(shape_node_ident));
                    // }
                    // let key = gsq.expected_graph.add_node(abstract_value.clone());
                    // gsq.node_keys_to_shape_idents.insert(key, shape_node_ident);
                    // gsq.shape_idents_to_node_keys.insert(shape_node_ident, key);

                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeNode(*marker, abstract_value.clone()));
                }
                BuilderInstruction::ExpectShapeEdge(source, target, abstract_value) => {
                    iter.next();
                    // TODO: we need a current view of the abstract graph (or, well, AID mappings) so that we can build the GraphShapeQuery here which requires
                    //  an actual `Graph`.

                    // instead, switch to deferred approach by just passing along the instructions
                    gsq_instructions.push(GraphShapeQueryInstruction::ExpectShapeEdge(*source, *target, abstract_value.clone()));
                }
                _ => {
                    return Err(OperationBuilderError::InvalidInQuery);
                }
            }
        }

        Ok((gsq_instructions, IntermediateQueryInstructions {
            true_branch: true_branch_instructions.unwrap_or_default(),
            false_branch: false_branch_instructions.unwrap_or_default(),
        }))
    }


    fn build_query_instruction(
        iter: &mut Peekable<Iter<BuilderInstruction<S>>>,
    ) -> Result<IntermediateQueryInstructions<S>, OperationBuilderError> {
        // we just consumed a StartQuery instruction.
        let mut true_branch_instructions = None;
        let mut false_branch_instructions = None;
        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::EnterTrueBranch => {
                    iter.next();
                    if true_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(true));
                    }
                    true_branch_instructions = Some(Self::build_many_instructions(iter)?);
                }
                BuilderInstruction::EnterFalseBranch => {
                    iter.next();
                    if false_branch_instructions.is_some() {
                        return Err(OperationBuilderError::AlreadyVisitedBranch(false));
                    }
                    false_branch_instructions = Some(Self::build_many_instructions(iter)?);
                }
                BuilderInstruction::EndQuery => {
                    iter.next();
                    break;
                }
                _ => {
                    return Err(OperationBuilderError::InvalidInQuery);
                }
            }
        }
        let true_branch = true_branch_instructions.unwrap_or_default();
        let false_branch = false_branch_instructions.unwrap_or_default();
        Ok(IntermediateQueryInstructions {
            true_branch,
            false_branch,
        })
    }

    fn build_operation_parameter(iter: &mut Peekable<Iter<BuilderInstruction<S>>>) -> Result<OperationParameter<S>, OperationBuilderError> {
        let mut operation_parameter = OperationParameter::<S> {
            explicit_input_nodes: Vec::new(),
            parameter_graph: AbstractGraph::<S>::new(),
            subst_to_node_keys: HashMap::new(),
            node_keys_to_subst: HashMap::new(),
        };

        while let Some(instruction) = iter.peek() {
            match instruction {
                BuilderInstruction::ExpectParameterNode(marker, node_abstract) => {
                    iter.next();
                    if operation_parameter.subst_to_node_keys.contains_key(marker) {
                        return Err(OperationBuilderError::ReusedSubstMarker(*marker));
                    }
                    let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
                    operation_parameter.subst_to_node_keys.insert(*marker, key);
                    operation_parameter.node_keys_to_subst.insert(key, *marker);
                    operation_parameter.explicit_input_nodes.push(*marker);
                }
                BuilderInstruction::ExpectContextNode(marker, node_abstract) => {
                    iter.next();
                    if operation_parameter.subst_to_node_keys.contains_key(marker) {
                        return Err(OperationBuilderError::ReusedSubstMarker(*marker));
                    }
                    let key = operation_parameter.parameter_graph.add_node(node_abstract.clone());
                    operation_parameter.subst_to_node_keys.insert(*marker, key);
                    operation_parameter.node_keys_to_subst.insert(key, *marker);
                }
                BuilderInstruction::ExpectParameterEdge(source_marker, target_marker, edge_abstract) => {
                    iter.next();
                    let source_key = operation_parameter.subst_to_node_keys.get(source_marker)
                        .ok_or(OperationBuilderError::NotFoundSubstMarker(*source_marker))?;
                    let target_key = operation_parameter.subst_to_node_keys.get(target_marker)
                        .ok_or(OperationBuilderError::NotFoundSubstMarker(*target_marker))?;
                    operation_parameter.parameter_graph.add_edge(*source_key, *target_key, edge_abstract.clone());
                }
                _ => {
                    break;
                }
            }
        }

        Ok(operation_parameter)
    }
}



















