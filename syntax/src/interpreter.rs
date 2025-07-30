use crate::custom_syntax::{CustomSyntax, SemanticsWithCustomSyntax};
use crate::{
    Block, FnCallExpr, FnDef, FnImplicitParam, FnNodeParam, IfCond, IfStmt, LetStmt, MacroArgs,
    NodeId, Program, RenameStmt, ReturnStmt, ReturnStmtMapping, ShapeQueryParam, ShapeQueryParams,
    Span, Spanned, Statement, Token, lexer,
};
use chumsky::input::Stream;
use chumsky::prelude::*;
use error_stack::{Report, Result, ResultExt, report};
use grabapl::operation::builder::IntermediateState;
use grabapl::operation::marker::SkipMarkers;
use grabapl::prelude::*;
use std::collections::HashMap;
use thiserror::Error;

pub fn parse_abstract_node_type<S: SemanticsWithCustomSyntax>(
    src: &str,
) -> Option<<S::CS as CustomSyntax>::AbstractNodeType> {
    let tokens = lexer().parse(src).into_result().ok()?;
    let tokens_input = tokens
        .as_slice()
        .map((src.len()..src.len()).into(), |(t, s)| (t, s));

    let parser = S::CS::get_node_type_parser();
    parser.parse(tokens_input).into_result().ok()
}

fn find_lib_builtin_op<S: SemanticsWithCustomSyntax>(
    name: Spanned<&str>,
    args: Option<Spanned<MacroArgs>>,
) -> Option<Result<LibBuiltinOperation<S>, SpannedInterpreterError>> {
    let name_span = name.1;
    let name = name.0;
    match name {
        "mark_node" => {
            // wrap everything below inside a closure that returns a Result, since we guarantee we'll return a Some() in this branch.
            let result = (|| {
                let args = args.ok_or(
                    InterpreterError::Custom("mark_node requires macro arguments")
                        .with_span(name_span),
                )?;
                let args_src = args.0.0;
                let args_span = args.1;
                // parse something of the form: `"color_name", NodeType`
                // let first_quote = args_src.find('"')?;
                // let args_src = &args_src[first_quote + 1..];
                // let second_quote = args_src.find('"')?;
                // let color_name = &args_src[..second_quote];
                // let rest = &args_src[second_quote + 1..];
                // let comma_pos = rest.find(',')?;
                // let rest = &rest[comma_pos + 1..];

                // // parse S::CS::AbstractNodeType
                // let syntax_typ = parse_abstract_node_type::<S>(rest)?;
                // // TODO: these functions should return errors. Something like Option<Result<>>, so that a Some() indicates "we're responsible for this op name", and a Some(Err())
                // //  indicates "we are responsible, and we're telling you that this is an error".
                // let node_type = S::convert_node_type(syntax_typ)?;

                // let marker = color_name.into();

                let parser = {
                    let color_name = select! {
                        Token::Str(s) => s,
                    };
                    let syntax_node_type = S::CS::get_node_type_parser();
                    let semantics_node_type = syntax_node_type.try_map_with(|syntax_type, e| {
                        let node_type =
                            S::convert_node_type(syntax_type.clone()).ok_or_else(|| {
                                Rich::custom(
                                    e.span(),
                                    format!("Node type not supported: {syntax_type:?}"),
                                )
                            })?;
                        Ok(node_type)
                    });
                    let optional_node_type = just(Token::Ctrl(','))
                        .ignore_then(semantics_node_type)
                        .or_not()
                        .try_map_with(|type_, e| match type_ {
                            Some(typ) => Ok(typ),
                            None => S::top_node_abstract().ok_or(Rich::custom(
                                e.span(),
                                "No node type provided, and no top node abstract defined"
                                    .to_string(),
                            )),
                        });
                    color_name.then(optional_node_type)
                };
                let (color_name, node_type) = lex_then_parse(args_src, parser).change_context(
                    InterpreterError::Custom("Failed to parse arguments for mark_node")
                        .with_span(args_span),
                )?;

                Ok(LibBuiltinOperation::MarkNode {
                    marker: color_name.into(),
                    param: node_type,
                })
            })();
            match result {
                Ok(op) => Some(Ok(op)),
                Err(e) => Some(Err(e)),
            }
        }
        "remove_marker" => {
            let args = args?;
            let args_src = args.0.0;
            // parse something of the form: `"color_name"`
            let first_quote = args_src.find('"')?;
            let args_src = &args_src[first_quote + 1..];
            let second_quote = args_src.find('"')?;
            let color_name = &args_src[..second_quote];
            let rest = &args_src[second_quote + 1..].trim();
            if !rest.is_empty() {
                // too much stuff after the marker
                return None;
            }

            let marker = color_name.into();
            Some(Ok(LibBuiltinOperation::RemoveMarker { marker }))
        }
        _ => {
            // TODO: add more.
            None
        }
    }
}

#[derive(Error, Debug)]
pub enum InterpreterError {
    #[error("Failed to compile program due to semantic builder error")]
    BuilderError,
    #[error("Operation with name '{0}' not found in the program")]
    NotFoundOperation(String),
    #[error("Query with name '{0}' not found in the program")]
    NotFoundQuery(String),
    #[error("Node ID '{0}' not found in current context")]
    NotFoundNodeId(String),
    #[error("Return marker '{0}' not found in the function")]
    NotFoundReturnMarker(String),
    #[error("Failed to parse type: {0}")]
    InvalidType(String),
    #[error("Error: {0}")]
    Custom(&'static str),
    #[error("Error: {0}")]
    CustomOwned(String),
}

impl InterpreterError {
    pub fn with_span(self, span: Span) -> SpannedInterpreterError {
        SpannedInterpreterError { span, error: self }
    }
}

#[derive(Error, Debug)]
#[error("{error}")]
pub struct SpannedInterpreterError {
    pub span: Span,
    pub error: InterpreterError,
}

pub struct InterpreterResult<'src, S: SemanticsWithCustomSyntax, E> {
    pub op_ctx_and_map:
        std::result::Result<(OperationContext<S>, HashMap<&'src str, OperationId>), E>,
    // This is *outside* the result, since we might have a state_map even if the operation context fails to build!
    pub state_map: HashMap<String, IntermediateState<S>>,
}

pub fn interpret<S: SemanticsWithCustomSyntax>(
    prog: Spanned<Program<S::CS>>,
) -> InterpreterResult<S, Report<SpannedInterpreterError>> {
    let mut interpreter = Interpreter::<S>::new();
    let res = interpreter.interpret_program(prog);
    InterpreterResult {
        op_ctx_and_map: res.map(|_| (interpreter.built_op_ctx, interpreter.fns_to_op_ids)),
        state_map: interpreter.state_map,
    }
}

struct Interpreter<'src, S: SemanticsWithCustomSyntax> {
    fns_to_op_ids: HashMap<&'src str, u32>,
    built_op_ctx: OperationContext<S>,
    state_map: HashMap<String, IntermediateState<S>>,
}

impl<'src, S: SemanticsWithCustomSyntax> Interpreter<'src, S> {
    fn new() -> Self {
        Self {
            fns_to_op_ids: HashMap::new(),
            built_op_ctx: OperationContext::new(),
            state_map: HashMap::new(),
        }
    }

    fn interpret_program(
        &mut self,
        prog: Spanned<Program<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        // we iterate in reverse order such that all functions have their dependencies already parsed
        let mut err = None;
        for (name, fn_def) in prog.0.functions.into_iter().rev() {
            let op_id = self.fns_to_op_ids.len() as u32;
            self.fns_to_op_ids.insert(name, op_id);

            let res_user_op = self.interpret_fn_def(op_id, fn_def);
            match res_user_op {
                Ok(user_op) => {
                    self.built_op_ctx.add_custom_operation(op_id, user_op);
                }
                Err(e) => {
                    // Continue interpreting to get as many state maps as possible
                    err.get_or_insert(e);
                }
            }
        }
        if let Some(e) = err {
            return Err(e);
        }
        Ok(())
    }

    fn interpret_fn_def(
        &mut self,
        self_op_id: OperationId,
        fn_def: Spanned<FnDef<'src, S::CS>>,
    ) -> Result<UserDefinedOperation<S>, SpannedInterpreterError> {
        // use a OperationBuilder to interpret the function definition and build a user defined operation

        let mut builder = OperationBuilder::new(&self.built_op_ctx, self_op_id);

        let mut interpreter =
            FnInterpreter::new(&mut builder, &self.fns_to_op_ids, fn_def.0.name.0);
        let fn_span = fn_def.1;
        let res = interpreter.interpret_fn_def(fn_def);
        // get the state maps before returning an error
        self.state_map.extend(interpreter.state_map);
        res?;

        builder
            .build()
            .change_context(InterpreterError::BuilderError.with_span(fn_span))
    }
}

struct FnInterpreter<'src, 'a, 'op_ctx, S: SemanticsWithCustomSyntax> {
    builder: &'a mut OperationBuilder<'op_ctx, S>,
    self_name: &'src str,
    fn_names_to_op_ids: &'a HashMap<&'src str, u32>,
    single_node_aids: HashMap<&'src str, AbstractNodeId>,
    return_marker_to_av: HashMap<&'src str, S::NodeAbstract>,
    state_map: HashMap<String, IntermediateState<S>>,
    shape_query_counter: u64,
    current_path_diverged: bool,
}

impl<'src, 'a, 'op_ctx, S: SemanticsWithCustomSyntax> FnInterpreter<'src, 'a, 'op_ctx, S> {
    fn new(
        builder: &'a mut OperationBuilder<'op_ctx, S>,
        fn_names_to_op_ids: &'a HashMap<&'src str, u32>,
        self_name: &'src str,
    ) -> Self {
        Self {
            builder,
            self_name,
            fn_names_to_op_ids,
            single_node_aids: HashMap::new(),
            return_marker_to_av: HashMap::new(),
            state_map: HashMap::new(),
            shape_query_counter: 0,
            current_path_diverged: false,
        }
    }

    fn interpret_fn_def(
        &mut self,
        (fn_def, _): Spanned<FnDef<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        // interpret the parameter graph
        // explicit
        for param in fn_def.explicit_params {
            self.interpret_fn_node_param(true, param)?;
        }

        // implicit
        for (param, param_span) in fn_def.implicit_params {
            match param {
                FnImplicitParam::Node(node_param) => {
                    self.interpret_fn_node_param(false, (node_param, param_span))?;
                }
                FnImplicitParam::Edge(edge_param) => {
                    let src = edge_param.src.0;
                    let dst = edge_param.dst.0;
                    let typ =
                        S::convert_edge_type(edge_param.edge_type.0.clone()).ok_or(report!(
                            InterpreterError::InvalidType(format!("{:?}", edge_param.edge_type.0))
                                .with_span(edge_param.edge_type.1)
                        ))?;
                    self.builder
                        .expect_parameter_edge(src, dst, typ)
                        .change_context(InterpreterError::BuilderError.with_span(param_span))?;
                }
            }
        }

        // then immediately register the return signature
        for (return_sig, return_sig_span) in fn_def.return_signature {
            match return_sig {
                FnImplicitParam::Node(node_sig) => {
                    let name = node_sig.name.0;
                    let param_type =
                        S::convert_node_type(node_sig.node_type.0.clone()).ok_or(report!(
                            InterpreterError::InvalidType(format!("{:?}", node_sig.node_type.0))
                                .with_span(node_sig.node_type.1)
                        ))?;
                    self.return_marker_to_av.insert(name, param_type.clone());
                    self.builder
                        .expect_self_return_node(name, param_type)
                        .change_context(
                            InterpreterError::BuilderError.with_span(return_sig_span),
                        )?;
                }
                FnImplicitParam::Edge(edge_sig) => {
                    todo!("Edge return signatures are not yet supported in the OperationBuilder");
                }
            }
        }

        // then interpret the body
        // TODO: we need an explicit "force build parameter" command that does the validation, because otherwise
        //  we get a builder error when adding the first instruction if our parameter is invalid. That gives us a weird span and bad UX.
        self.interpret_block(fn_def.body)?;
        Ok(())
    }

    fn interpret_fn_node_param(
        &mut self,
        explicit: bool,
        (param, param_span): Spanned<FnNodeParam<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        let name = param.name.0;
        let param_type = S::convert_node_type(param.node_type.0.clone()).ok_or(report!(
            InterpreterError::InvalidType(format!("{:?}", param.node_type.0))
                .with_span(param.node_type.1)
        ))?;
        // TODO: instead of unwrap, should be returning results?
        if explicit {
            self.builder
                .expect_parameter_node(name, param_type)
                .change_context(InterpreterError::BuilderError.with_span(param_span))
                .attach_printable_lazy(|| {
                    format!("Failed to add explicit node parameter {name}")
                })?;
        } else {
            self.builder
                .expect_context_node(name, param_type)
                .change_context(InterpreterError::BuilderError.with_span(param_span))
                .attach_printable_lazy(|| {
                    format!("Failed to add implicit node parameter {name}")
                })?;
        }
        self.single_node_aids
            .insert(name, AbstractNodeId::param(name));
        Ok(())
    }

    fn interpret_block(
        &mut self,
        (body, _): Spanned<Block<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        // save and restore id mapping
        // let saved_single_node_aids = self.single_node_aids.clone();
        for stmt in body.statements {
            self.interpret_stmt(stmt)?;
        }
        // restore the single node aids mapping
        // self.single_node_aids = saved_single_node_aids;
        Ok(())
    }

    fn interpret_stmt(
        &mut self,
        (stmt, _): Spanned<Statement<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        match stmt {
            Statement::Let(let_stmt) => {
                self.interpret_let_stmt(let_stmt)?;
            }
            Statement::FnCall(fn_call) => {
                // println!("Interpreting function call: {:?}", fn_call);
                if self.interpret_hardcoded(&fn_call)? {
                    // if this was a hardcoded function call, we don't need to interpret it further
                    return Ok(());
                }

                let fn_call_span = fn_call.1;
                let (op_like, args) = self.call_expr_to_op_like(fn_call)?;
                self.interpret_op_like(None, op_like, args, fn_call_span)?;
            }
            Statement::If(if_stmt) => {
                self.interpret_if_stmt(if_stmt)?;
            }
            Statement::Return(return_stmt) => {
                self.interpret_return(return_stmt)?;
            }
            Statement::Rename(rename_stmt) => {
                self.interpret_rename(rename_stmt)?;
            }
        }
        Ok(())
    }

    /// Returns if this was a hardcoded function call that was successfully interpreted
    fn interpret_hardcoded(
        &mut self,
        fn_call: &Spanned<FnCallExpr<'src>>,
    ) -> Result<bool, SpannedInterpreterError> {
        // check if we're calling show();
        if fn_call.0.name.0 == "show_state" {
            let state_name = if let Some(arg_ident) = fn_call.0.args.first() {
                let as_str = arg_ident.0.single().ok_or(report!(
                    InterpreterError::Custom("needs a single node id for show_state")
                        .with_span(arg_ident.1)
                ))?;
                as_str.to_string()
            } else {
                "<unnamed state>".to_string()
            };
            let state = self
                .builder
                .show_state()
                .change_context(InterpreterError::BuilderError.with_span(fn_call.0.name.1))?;
            self.state_map.insert(state_name, state);
            return Ok(true);
        }

        if fn_call.0.name.0 == "trace" {
            // assert no args and no macro args
            if !fn_call.0.args.is_empty() || fn_call.0.macro_args.is_some() {
                return Err(report!(
                    InterpreterError::Custom("trace does not take any arguments")
                        .with_span(fn_call.0.name.1)
                ));
            }
            // start tracing
            self.builder
                .trace()
                .change_context(InterpreterError::BuilderError.with_span(fn_call.0.name.1))?;
            return Ok(true);
        }

        if fn_call.0.name.0 == "diverge" {
            // expect a string macro args
            let args = fn_call.0.macro_args;
            let args = args.ok_or(report!(
                InterpreterError::Custom("diverge requires a string argument")
                    .with_span(fn_call.0.name.1)
            ))?;
            let args_src = args.0.0;
            let args_span = args.1;
            // must parse a double-quote delimited string.
            // let mut container = vec![];
            // let value_inpt = parse_with_lexer(args_src, &mut container).ok_or(report!(
            //     InterpreterError::Custom("diverge requires a string argument")
            //         .with_span(args_span)
            // ))?;
            // let inner_msg = Parser::<_, _, extra::Default>::parse(&select! {
            //     Token::Str(s) => s,
            // }, value_inpt).into_result().map_err(|_| report!(
            //     InterpreterError::Custom("diverge requires a string argument")
            //         .with_span(args_span)
            // ))?;
            let inner_msg = lex_then_parse(args_src, select! { Token::Str(s) => s })
                .change_context(
                    InterpreterError::Custom("invalid diverge arguments").with_span(args_span),
                )?;
            // now we can diverge
            self.builder
                .diverge(inner_msg)
                .change_context(InterpreterError::BuilderError.with_span(fn_call.0.name.1))?;
            self.current_path_diverged = true;
            return Ok(true);
        }

        Ok(false)
    }

    fn interpret_rename(
        &mut self,
        (rename_stmt, rename_stmt_span): Spanned<RenameStmt<'src>>,
    ) -> Result<(), SpannedInterpreterError> {
        // assert that the new name does not exist yet
        if self.single_node_aids.contains_key(rename_stmt.new_name.0) {
            return Err(report!(
                InterpreterError::CustomOwned(format!(
                    "Node with name '{}' already exists",
                    rename_stmt.new_name.0
                ))
                .with_span(rename_stmt.new_name.1)
            ));
        }
        let new_name = rename_stmt.new_name.0;
        let new_aid = AbstractNodeId::named(new_name);
        let old_aid = self.node_id_to_aid(rename_stmt.src.0).ok_or(report!(
            InterpreterError::NotFoundNodeId(format!("{:?}", rename_stmt.src.0))
                .with_span(rename_stmt.src.1)
        ))?;
        self.builder
            .rename_node(old_aid, new_name)
            .change_context(InterpreterError::BuilderError.with_span(rename_stmt_span))
            .attach_printable_lazy(|| "Failed to rename")?;
        self.single_node_aids.insert(new_name, new_aid);
        Ok(())
    }

    fn interpret_if_stmt(
        &mut self,
        (if_stmt, if_stmt_span): Spanned<IfStmt<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        // start the branchable query (shape or builtin)

        // TODO: if queries could create nodes, this would need to be handled.
        let initial_nodes = self.single_node_aids.clone();
        let initial_diverged = self.current_path_diverged;

        let rename_instructions_then_branch = self.interpret_if_cond_and_start(if_stmt.cond)?;

        self.builder
            .enter_true_branch()
            .change_context(InterpreterError::BuilderError.with_span(if_stmt.then_block.1))
            .attach_printable_lazy(|| "Failed to enter true branch")?;
        // interpret the true branch
        // rename
        self.rename_many(rename_instructions_then_branch)?;
        self.interpret_block(if_stmt.then_block)?;
        self.builder
            .enter_false_branch()
            .change_context(InterpreterError::BuilderError.with_span(if_stmt.else_block.1))
            .attach_printable_lazy(|| "Failed to enter false branch")?;

        let true_branch_aids = std::mem::replace(&mut self.single_node_aids, initial_nodes);
        let true_branch_diverged = self.current_path_diverged;
        self.current_path_diverged = initial_diverged;

        // interpret the false branch
        self.interpret_block(if_stmt.else_block)?;
        self.builder
            .end_query()
            .change_context(InterpreterError::BuilderError.with_span(if_stmt_span))
            .attach_printable_lazy(|| "Failed to end query")?;

        let false_branch_diverged = self.current_path_diverged;
        (self.single_node_aids, self.current_path_diverged) = merge_node_aids(
            &true_branch_aids,
            true_branch_diverged,
            &self.single_node_aids,
            false_branch_diverged,
        );
        Ok(())
    }

    fn rename_many(
        &mut self,
        rename_instructions: HashMap<Spanned<&'src str>, AbstractNodeId>,
    ) -> Result<(), SpannedInterpreterError> {
        for (name, aid) in rename_instructions {
            let name_span = name.1;
            let name = name.0;
            let new_aid = AbstractNodeId::named(name);
            self.builder
                .rename_node(aid, name)
                .change_context(InterpreterError::BuilderError.with_span(name_span))
                .attach_printable_lazy(|| format!("Failed to rename node {aid:?} to {name}"))?;
            self.single_node_aids.insert(name, new_aid);
        }
        Ok(())
    }

    /// Returns rename instructions for the beginning of the then branch
    fn interpret_if_cond_and_start(
        &mut self,
        (cond, _): Spanned<IfCond<'src, S::CS>>,
    ) -> Result<HashMap<Spanned<&'src str>, AbstractNodeId>, SpannedInterpreterError> {
        // starts either a builtin query or a shape query
        match cond {
            IfCond::Query((fn_call, fn_call_span)) => {
                let query = self.query_name_to_builtin_query(fn_call.name, fn_call.macro_args)?;
                let args = fn_call
                    .args
                    .into_iter()
                    .map(|(arg, arg_span)| {
                        self.node_id_to_aid(arg).ok_or(report!(
                            InterpreterError::NotFoundNodeId(format!("{arg:?}"))
                                .with_span(arg_span)
                        ))
                    })
                    .collect::<Result<Vec<_>, SpannedInterpreterError>>()?;
                self.builder
                    .start_query(query, args)
                    .change_context(InterpreterError::BuilderError.with_span(fn_call_span))?;
                Ok(HashMap::new())
            }
            IfCond::Shape(shape_query_params) => {
                self.interpret_and_start_shape_query(shape_query_params)
            }
        }
    }

    /// Returns the required rename instructions at the start of the then branch
    fn interpret_and_start_shape_query(
        &mut self,
        (shape_query_params, sqp_span): Spanned<ShapeQueryParams<'src, S::CS>>,
    ) -> Result<HashMap<Spanned<&'src str>, AbstractNodeId>, SpannedInterpreterError> {
        // need to invent a marker.
        let marker = self.get_new_shape_query_marker()?;
        let marker = marker.as_str();
        self.builder
            .start_shape_query(marker)
            .change_context(InterpreterError::BuilderError.with_span(sqp_span))
            .attach_printable_lazy(|| {
                format!("Failed to start shape query with marker {marker}")
            })?;
        // send the skip markers
        match shape_query_params.skip_markers {
            SkipMarkers::All => {
                self.builder
                    .skip_all_markers()
                    // TODO: use better spans
                    .change_context(InterpreterError::BuilderError.with_span(sqp_span))
                    .attach_printable_lazy(|| "Failed to skip all markers")?;
            }
            SkipMarkers::Set(set) => {
                for marker in set {
                    // TODO: use better spans
                    self.builder
                        .skip_marker(marker)
                        .change_context(InterpreterError::BuilderError.with_span(sqp_span))
                        .attach_printable_lazy(|| format!("Failed to skip marker {marker:?}"))?;
                }
            }
        }
        // then interpret the shape query parameters
        let mut new_nodes_to_rename = HashMap::new();
        for (param, param_span) in shape_query_params.params {
            match param {
                ShapeQueryParam::Node(node_param) => {
                    let node_id = node_param.name.0;
                    let param_type =
                        S::convert_node_type(node_param.node_type.0.clone()).ok_or(report!(
                            InterpreterError::InvalidType(format!("{:?}", node_param.node_type.0))
                                .with_span(node_param.node_type.1)
                        ))?;

                    // we differentiate between an existing node, in which case we issue an expected value change,
                    // or a new one, in which case it must be a single.

                    if let Some(aid) = self.node_id_to_aid(node_id) {
                        // issue an expected value change
                        self.builder
                            .expect_shape_node_change(aid, param_type)
                            .change_context(InterpreterError::BuilderError.with_span(param_span))?;
                    } else {
                        // must be a new node
                        let name = node_id.must_single();
                        let aid = AbstractNodeId::dynamic_output(marker, name);
                        self.builder
                            .expect_shape_node(name.into(), param_type)
                            .change_context(InterpreterError::BuilderError.with_span(param_span))?;
                        // we need to rename these as soon as we enter the then branch.
                        new_nodes_to_rename.insert((name, node_param.name.1), aid);
                    }
                }
                ShapeQueryParam::Edge(edge_param) => {
                    let src = edge_param.src.0;
                    let dst = edge_param.dst.0;

                    let src_aid = self
                        .node_id_to_aid(src)
                        .or_else(|| Some(AbstractNodeId::dynamic_output(marker, src.single()?)))
                        .ok_or(report!(
                            InterpreterError::NotFoundNodeId(format!("{src:?}"))
                                .with_span(edge_param.src.1)
                        ))?;
                    let dst_aid = self
                        .node_id_to_aid(dst)
                        .or_else(|| Some(AbstractNodeId::dynamic_output(marker, dst.single()?)))
                        .ok_or(report!(
                            InterpreterError::NotFoundNodeId(format!("{dst:?}"))
                                .with_span(edge_param.dst.1)
                        ))?;

                    let typ =
                        S::convert_edge_type(edge_param.edge_type.0.clone()).ok_or(report!(
                            InterpreterError::InvalidType(format!("{:?}", edge_param.edge_type.0))
                                .with_span(edge_param.edge_type.1)
                        ))?;
                    self.builder
                        .expect_shape_edge(src_aid, dst_aid, typ)
                        .change_context(InterpreterError::BuilderError.with_span(param_span))?;
                }
            }
        }
        Ok(new_nodes_to_rename)
    }

    fn get_new_shape_query_marker(&mut self) -> Result<String, SpannedInterpreterError> {
        let marker = format!("shape_query_{}", self.shape_query_counter);
        self.shape_query_counter += 1;
        Ok(marker)
    }

    fn interpret_let_stmt(
        &mut self,
        (let_stmt, let_span): Spanned<LetStmt<'src>>,
    ) -> Result<(), SpannedInterpreterError> {
        if let_stmt.bang {
            let result_name = let_stmt.ident.0;
            let (op_like, args) = self.call_expr_to_op_like(let_stmt.call)?;
            self.builder
                .add_bang_operation(result_name, op_like, args)
                .change_context(InterpreterError::BuilderError.with_span(let_span))?;
            let new_aid = AbstractNodeId::named(result_name);
            self.single_node_aids.insert(result_name, new_aid);
        } else {
            let op_name = let_stmt.ident.0;
            let call_span = let_stmt.call.1;
            let (op_like, args) = self.call_expr_to_op_like(let_stmt.call)?;

            self.interpret_op_like(Some(op_name), op_like, args, call_span)?;
        }
        Ok(())
    }

    fn query_name_to_builtin_query(
        &self,
        (query_name, query_span): Spanned<&str>,
        args: Option<Spanned<MacroArgs>>,
    ) -> Result<S::BuiltinQuery, SpannedInterpreterError> {
        let args = args.map(|(args, _)| args);
        S::find_builtin_query(query_name, args).ok_or(report!(
            InterpreterError::NotFoundQuery(query_name.to_string()).with_span(query_span)
        ))
    }

    fn op_name_to_op_like(
        &self,
        spanned_op_name: Spanned<&str>,
        args: Option<Spanned<MacroArgs>>,
        err_span: Span,
    ) -> Result<BuilderOpLike<S>, SpannedInterpreterError> {
        // TODO: do we want to enforce consumption of a Some(macro_args)?

        // TODO: the order should be different. first: UDF, then builtin, then lib builtin.

        // first try lib builtin
        if let Some(op) = find_lib_builtin_op::<S>(spanned_op_name, args) {
            return Ok(BuilderOpLike::LibBuiltin(op?));
        }

        // we don't care about the spans here yet
        let args = args.map(|(args, _)| args);
        let op_name = spanned_op_name.0;

        // then try client builtin
        if let Some(op) = S::find_builtin_op(op_name, args) {
            return Ok(BuilderOpLike::Builtin(op));
        }

        if op_name == self.self_name {
            // if the operation name is the same as the function name, we must be recursing
            return Ok(BuilderOpLike::Recurse);
        }

        // otherwise must be a user defined operation
        let op_id = self.fn_names_to_op_ids.get(op_name).ok_or(report!(
            InterpreterError::NotFoundOperation(op_name.to_string()).with_span(err_span)
        ))?;

        Ok(BuilderOpLike::FromOperationId(*op_id))
    }

    fn interpret_op_like(
        &mut self,
        op_name: Option<&'src str>,
        op_like: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
        err_span: Span,
    ) -> Result<(), SpannedInterpreterError> {
        if let Some(op_name) = op_name {
            self.builder
                .add_named_operation(op_name.into(), op_like, args)
                .change_context(InterpreterError::BuilderError.with_span(err_span))
                .attach_printable_lazy(|| {
                    format!("Failed to add operation with result binding {op_name}")
                })?;
        } else {
            self.builder
                .add_operation(op_like, args)
                .change_context(InterpreterError::BuilderError.with_span(err_span))
                .attach_printable_lazy(|| "Failed to add operation without result binding")?;
        }
        Ok(())
    }

    fn call_expr_to_op_like(
        &mut self,
        (call_expr, _): Spanned<FnCallExpr<'src>>,
    ) -> Result<(BuilderOpLike<S>, Vec<AbstractNodeId>), SpannedInterpreterError> {
        let args = call_expr
            .args
            .into_iter()
            .map(|(arg, span)| {
                self.node_id_to_aid(arg).ok_or(report!(
                    InterpreterError::NotFoundNodeId(format!("{arg:?}")).with_span(span)
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let op_name = call_expr.name;
        let macro_args = call_expr.macro_args;

        let op_like = self.op_name_to_op_like(op_name, macro_args, call_expr.name.1)?;

        Ok((op_like, args))
    }

    fn interpret_return(
        &mut self,
        (return_stmt, _return_stmt_span): Spanned<ReturnStmt<'src, S::CS>>,
    ) -> Result<(), SpannedInterpreterError> {
        for (mapping, mapping_span) in return_stmt.mapping {
            match mapping {
                ReturnStmtMapping::Node { ret_name, node } => {
                    let aid = self
                        .node_id_to_aid(node.0)
                        .ok_or(report!(
                            InterpreterError::NotFoundNodeId(format!("{:?}", node.0))
                                .with_span(node.1)
                        ))
                        .attach_printable("return node AID not found")?;
                    let ret_name_str = ret_name.0;
                    let ret_ty = self.return_marker_to_av.get(ret_name_str).ok_or(report!(
                        InterpreterError::NotFoundReturnMarker(ret_name_str.to_string())
                            .with_span(ret_name.1)
                    ))?;
                    self.builder
                        .return_node(aid, ret_name_str.into(), ret_ty.clone())
                        .change_context(InterpreterError::BuilderError.with_span(mapping_span))?;
                }
                ReturnStmtMapping::Edge { .. } => {
                    todo!("Edge return mappings are not yet supported in the OperationBuilder");
                }
            }
        }
        Ok(())
    }

    fn node_id_to_aid(&self, node_id: NodeId) -> Option<AbstractNodeId> {
        match node_id {
            NodeId::Single(name) => self.single_node_aids.get(name).copied(),
            NodeId::Output((op_name, _), (node_name, _)) => {
                Some(AbstractNodeId::dynamic_output(op_name, node_name))
            }
        }
    }
}

// merges entries only if they're the same in both maps
/// Returns the merged single nodes and whether the merged paths diverged
fn merge_node_aids<'a>(
    true_branch: &HashMap<&'a str, AbstractNodeId>,
    true_diverged: bool,
    false_branch: &HashMap<&'a str, AbstractNodeId>,
    false_diverged: bool,
) -> (HashMap<&'a str, AbstractNodeId>, bool) {
    // if either branch diverged, just return the nodes from the other branch
    if true_diverged {
        return (false_branch.clone(), false_diverged);
    }
    if false_diverged {
        return (true_branch.clone(), true_diverged);
    }

    let mut merged = HashMap::new();
    for (name, aid) in true_branch.iter() {
        if let Some(false_aid) = false_branch.get(name) {
            if aid == false_aid {
                merged.insert(*name, *aid);
            }
        }
    }
    (merged, false)
}

// wow this is terrible
// fn parse_with_lexer<'container, 'src: 'container>(src: &'src str, container: &'container mut Vec<Spanned<Token<'src>>>) -> Option<impl ValueInput<'container, Token = Token<'src>, Span = Span> + SliceInput<
//     'container,
//     Token = Token<'src>,
//     Span = Span,
//     Slice = &'container [Spanned<Token<'src>>],
// >>
// {
//     let toks = lexer().parse(src).into_result().ok()?;
//     container.extend(toks);
//     let toks_input = container
//         .map((src.len()..src.len()).into(), |(t, s)| (t, s));
//     Some(toks_input)
//
// }

// ugly inside, but much better outside I think
pub fn lex_then_parse<'tokens, 'src: 'tokens, P, O>(
    src: &'src str,
    parser: P,
) -> std::result::Result<O, InterpreterError>
where
    P: Parser<
            'tokens,
            Stream<std::vec::IntoIter<Token<'src>>>,
            O,
            extra::Err<Rich<'tokens, Token<'src>, Span>>,
        >,
{
    let tokens = lexer()
        .parse(src)
        .into_result()
        .map_err(|e| {
            e.into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map_err(InterpreterError::CustomOwned)?;
    let tokens = tokens.into_iter().map(|(t, _)| t).collect::<Vec<_>>();
    let input = Stream::from_iter(tokens);
    parser
        .parse(input)
        .into_result()
        .map_err(|e| {
            e.into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map_err(InterpreterError::CustomOwned)
}
