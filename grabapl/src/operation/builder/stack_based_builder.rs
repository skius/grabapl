use thiserror::Error;
use crate::{OperationContext, Semantics};
use crate::operation::builder::{BuilderInstruction, BuilderOpLike, IntermediateState, UDInstructionsWithMarker};
use crate::operation::signature::parameter::OperationParameter;
use crate::operation::signature::parameterbuilder::{OperationParameterBuilder, ParameterBuilderError};

#[derive(Debug, Error)]
enum BuilderError {
    #[error("Unexpected instruction while building parameter")]
    UnexpectedInstruction,
    #[error("Failed to build operation parameter: {0}")]
    ParameterBuildError(#[from] ParameterBuilderError),
}

enum Frame<S: Semantics> {
    BuildingParameter(OperationParameterBuilder<S>),
    CollectingInstructions(UDInstructionsWithMarker<S>),
}

struct BuiltData<S: Semantics> {
    parameter: Option<OperationParameter<S>>,
    intermediate_state: Option<IntermediateState<S>>,
}

impl<S: Semantics> BuiltData<S> {
    pub fn new() -> Self {
        BuiltData {
            parameter: None,
            intermediate_state: None,
        }
    }

    pub fn provide_parameter(&mut self, parameter: OperationParameter<S>) {
        self.intermediate_state = Some(IntermediateState::from_param(&parameter));
        self.parameter = Some(parameter);
    }
}

pub struct Builder<'a, S: Semantics> {
    op_ctx: &'a OperationContext<S>,
    built: BuiltData<S>,
    stack: Vec<Frame<S>>,
}


impl<'a, S: Semantics> Builder<'a, S> {
    pub fn new(op_ctx: &'a OperationContext<S>) -> Self {
        Builder {
            op_ctx,
            built: BuiltData::new(),
            stack: vec![Frame::BuildingParameter(OperationParameterBuilder::new())],
        }
    }

    pub fn consume(&mut self, instruction: BuilderInstruction<S>) -> Result<(), BuilderError> {
        let mut instruction_opt = Some(instruction);

        while instruction_opt.is_some() {
            // TODO: maybe instead have signature of the consume_for_* methods take a mutable reference to the curr_frame, and return
            //  an option of the next frame if they want to change the frame?
            // actually no, not possible, since the mutable reference borrows from self, but we want to pass self as well.
            let curr_frame = self.stack.pop().unwrap();
            match curr_frame {
                Frame::BuildingParameter(param_builder) => {
                    self.consume_for_building_parameter(&mut instruction_opt, param_builder)?;
                }
                Frame::CollectingInstructions(instructions) => {
                    self.consume_for_collecting_instructions(&mut instruction_opt, instructions)?;
                }
            }
        }

        Ok(())
    }

    fn consume_for_building_parameter(
        &mut self,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
        mut param_builder: OperationParameterBuilder<S>,
    ) -> Result<(), BuilderError> {
        use BuilderInstruction as BI;

        let instruction = instruction_opt.as_ref().unwrap();

        let next_frame = match instruction {
            // TODO: ugly double match.
            //  instead: have BuilderInstruction have eg a .is_for_parameter() method.
            BI::ExpectParameterNode(..) | BI::ExpectContextNode(..) | BI::ExpectParameterEdge(..) => {
                // consume instruction
                let instruction = instruction_opt.take().unwrap();
                match instruction {
                    BI::ExpectParameterNode(marker, av) => {
                        param_builder.expect_explicit_input_node(marker, av)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    BI::ExpectContextNode(marker, av) => {
                        param_builder.expect_context_node(marker, av)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    BI::ExpectParameterEdge(src, dst, edge) => {
                        param_builder.expect_edge(src, dst, edge)?;
                        Frame::BuildingParameter(param_builder)
                    }
                    _ => unreachable!("we just checked that this matches"),
                }
            }
            _ => {
                // The user has decided that they're done building the parameter by sending a different instruction
                let parameter = param_builder.build()?;
                self.built.parameter = Some(parameter);

                Frame::CollectingInstructions(vec![])
            },
        };
        self.stack.push(next_frame);
        Ok(())
    }

    fn consume_for_collecting_instructions(
        &mut self,
        instruction_opt: &mut Option<BuilderInstruction<S>>,
        mut instructions: UDInstructionsWithMarker<S>,
    ) -> Result<(), BuilderError> {
        let instruction = instruction_opt.as_ref().unwrap();
        match instruction {
            BuilderInstruction::AddOperation(builder_op_like, params) => {

                let lib_builtin_op = match builder_op_like {
                    BuilderOpLike::LibBuiltin(op) => {op}
                    _ => todo!()
                };

                self.built.intermediate_state.as_mut().unwrap().apply_op(lib_builtin_op)
                // TODO: have "ApplyAbstract" trait auto-implemented for everything where necessary, and have intermediate state
                //  generically accept such a thing to abstract apply. obviously if there's more data that intermediate state
                //  needs for apply_abstract then maybe it doesnt work.

                // TODO: figure out how to now actually access the AIDs.
                //  we may have multiple intermediate states! we will also need to store those in the stack.
            }
            _ => return Err(BuilderError::UnexpectedInstruction),
        }
        Ok(())
    }
}