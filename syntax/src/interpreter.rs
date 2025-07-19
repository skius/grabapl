use crate::{
    Block, CustomSyntax, FnCallExpr, FnDef, FnImplicitParam, FnNodeParam, IfCond, IfStmt, LetStmt,
    MacroArgs, NodeId, Program, RenameStmt, ReturnStmt, ReturnStmtMapping, ShapeQueryParam,
    ShapeQueryParams, Spanned, Statement, lexer,
};
use chumsky::prelude::*;
use grabapl::operation::marker::SkipMarkers;
use grabapl::prelude::*;
use std::collections::HashMap;

fn parse_abstract_node_type<S: SemanticsWithCustomSyntax>(
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
    name: &str,
    args: Option<MacroArgs>,
) -> Option<LibBuiltinOperation<S>> {
    match name {
        "mark_node" => {
            let args = args?;
            let args_src = args.0;
            // parse something of the form: `"color_name", NodeType`
            let first_quote = args_src.find('"')?;
            let args_src = &args_src[first_quote + 1..];
            let second_quote = args_src.find('"')?;
            let color_name = &args_src[..second_quote];
            let rest = &args_src[second_quote + 1..];
            let comma_pos = rest.find(',')?;
            let rest = &rest[comma_pos + 1..];

            // parse S::CS::AbstractNodeType
            let syntax_typ = parse_abstract_node_type::<S>(rest)?;
            let node_type = S::convert_node_type(syntax_typ);

            let marker = color_name.into();
            Some(LibBuiltinOperation::MarkNode {
                marker,
                param: node_type,
            })
        }
        "remove_marker" => {
            let args = args?;
            let args_src = args.0;
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
            Some(LibBuiltinOperation::RemoveMarker { marker })
        }
        _ => {
            // TODO: add more.
            None
        }
    }
}

pub trait SemanticsWithCustomSyntax:
    Semantics<BuiltinOperation: Clone, BuiltinQuery: Clone>
{
    type CS: CustomSyntax;

    fn find_builtin_op(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinOperation>;

    fn find_builtin_query(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinQuery>;

    fn convert_node_type(
        syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType,
    ) -> Self::NodeAbstract;
    fn convert_edge_type(
        syn_typ: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType,
    ) -> Self::EdgeAbstract;
}

pub fn interpret<S: SemanticsWithCustomSyntax>(
    prog: Spanned<Program<S::CS>>,
) -> (OperationContext<S>, HashMap<&'_ str, OperationId>) {
    let mut interpreter = Interpreter::<S>::new();
    interpreter.interpret_program(prog);
    (interpreter.built_op_ctx, interpreter.fns_to_op_ids)
}

struct Interpreter<'src, S: SemanticsWithCustomSyntax> {
    fns_to_op_ids: HashMap<&'src str, u32>,
    built_op_ctx: OperationContext<S>,
}

impl<'src, S: SemanticsWithCustomSyntax> Interpreter<'src, S> {
    fn new() -> Self {
        Self {
            fns_to_op_ids: HashMap::new(),
            built_op_ctx: OperationContext::new(),
        }
    }

    fn interpret_program(&mut self, prog: Spanned<Program<'src, S::CS>>) {
        // we iterate in reverse order such that all functions have their dependencies already parsed
        for (name, fn_def) in prog.0.functions.into_iter().rev() {
            let op_id = self.fns_to_op_ids.len() as u32;
            self.fns_to_op_ids.insert(name, op_id);

            let user_op = self.interpret_fn_def(op_id, fn_def);
            self.built_op_ctx.add_custom_operation(op_id, user_op);
        }
    }

    fn interpret_fn_def(
        &mut self,
        self_op_id: OperationId,
        fn_def: Spanned<FnDef<S::CS>>,
    ) -> UserDefinedOperation<S> {
        // use a OperationBuilder to interpret the function definition and build a user defined operation

        let mut builder = OperationBuilder::new(&self.built_op_ctx, self_op_id);

        let mut interpreter =
            FnInterpreter::new(&mut builder, &self.fns_to_op_ids, fn_def.0.name.0);
        interpreter.interpret_fn_def(fn_def);

        builder.build().unwrap()
    }
}

struct FnInterpreter<'src, 'a, 'op_ctx, S: SemanticsWithCustomSyntax> {
    builder: &'a mut OperationBuilder<'op_ctx, S>,
    self_name: &'src str,
    fn_names_to_op_ids: &'a HashMap<&'src str, u32>,
    single_node_aids: HashMap<&'src str, AbstractNodeId>,
    return_marker_to_av: HashMap<&'src str, S::NodeAbstract>,
    shape_query_counter: u64,
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
            shape_query_counter: 0,
        }
    }

    fn interpret_fn_def(&mut self, (fn_def, _): Spanned<FnDef<'src, S::CS>>) {
        // interpret the parameter graph
        // explicit
        for (param, _) in fn_def.explicit_params {
            self.interpret_fn_node_param(true, param);
        }

        // implicit
        for (param, _) in fn_def.implicit_params {
            match param {
                FnImplicitParam::Node(node_param) => {
                    self.interpret_fn_node_param(false, node_param);
                }
                FnImplicitParam::Edge(edge_param) => {
                    let src = edge_param.src.0;
                    let dst = edge_param.dst.0;
                    let typ = S::convert_edge_type(edge_param.edge_type.0);
                    self.builder.expect_parameter_edge(src, dst, typ).unwrap();
                }
            }
        }

        // then immediately register the return signature
        for (return_sig, _) in fn_def.return_signature {
            match return_sig {
                FnImplicitParam::Node(node_sig) => {
                    let name = node_sig.name.0;
                    let param_type = S::convert_node_type(node_sig.node_type.0);
                    self.return_marker_to_av.insert(name, param_type.clone());
                    self.builder
                        .expect_self_return_node(name, param_type)
                        .unwrap();
                }
                FnImplicitParam::Edge(edge_sig) => {
                    todo!("Edge return signatures are not yet supported in the OperationBuilder");
                }
            }
        }

        // then interpret the body
        self.interpret_block(fn_def.body);
    }

    fn interpret_fn_node_param(&mut self, explicit: bool, param: FnNodeParam<'src, S::CS>) {
        let name = param.name.0;
        let param_type = S::convert_node_type(param.node_type.0);
        // TODO: instead of unwrap, should be returning results?
        if explicit {
            self.builder
                .expect_parameter_node(name, param_type)
                .unwrap();
        } else {
            self.builder.expect_context_node(name, param_type).unwrap();
        }
        self.single_node_aids
            .insert(name, AbstractNodeId::param(name));
    }

    fn interpret_block(&mut self, (body, _): Spanned<Block<'src, S::CS>>) {
        for stmt in body.statements {
            self.interpret_stmt(stmt);
        }
    }

    fn interpret_stmt(&mut self, (stmt, _): Spanned<Statement<'src, S::CS>>) {
        match stmt {
            Statement::Let(let_stmt) => {
                self.interpret_let_stmt(let_stmt);
            }
            Statement::FnCall(fn_call) => {
                let (op_like, args) = self.call_expr_to_op_like(fn_call);
                self.interpret_op_like(None, op_like, args);
            }
            Statement::If(if_stmt) => {
                self.interpret_if_stmt(if_stmt);
            }
            Statement::Return(return_stmt) => {
                self.interpret_return(return_stmt);
            }
            Statement::Rename(rename_stmt) => {
                self.interpret_rename(rename_stmt);
            }
        }
    }

    fn interpret_rename(&mut self, (rename_stmt, _): Spanned<RenameStmt<'src>>) {
        let new_name = rename_stmt.new_name.0;
        let new_aid = AbstractNodeId::named(new_name);
        let old_aid = self
            .node_id_to_aid(rename_stmt.src.0)
            .expect("Old name not found in single node aids");
        self.builder.rename_node(old_aid, new_name).unwrap();
        self.single_node_aids.insert(new_name, new_aid);
    }

    fn interpret_if_stmt(&mut self, (if_stmt, _): Spanned<IfStmt<'src, S::CS>>) {
        // start the branchable query (shape or builtin)
        self.interpret_if_cond_and_start(if_stmt.cond);

        self.builder.enter_true_branch().unwrap();
        // interpret the true branch
        self.interpret_block(if_stmt.then_block);
        self.builder.enter_false_branch().unwrap();
        // interpret the false branch
        self.interpret_block(if_stmt.else_block);
        self.builder.end_query().unwrap();
    }

    fn interpret_if_cond_and_start(&mut self, (cond, _): Spanned<IfCond<'src, S::CS>>) {
        // starts either a builtin query or a shape query
        match cond {
            IfCond::Query(fn_call) => {
                let query = self.query_name_to_builtin_query(fn_call.name.0, fn_call.macro_args);
                let args = fn_call
                    .args
                    .into_iter()
                    .map(|(arg, _)| self.node_id_to_aid(arg).unwrap())
                    .collect::<Vec<_>>();
                self.builder.start_query(query, args).unwrap();
            }
            IfCond::Shape(shape_query_params) => {
                self.interpret_and_start_shape_query(shape_query_params);
            }
        }
    }

    fn interpret_and_start_shape_query(
        &mut self,
        shape_query_params: ShapeQueryParams<'src, S::CS>,
    ) {
        // need to invent a marker.
        let marker = self.get_new_shape_query_marker();
        let marker = marker.as_str();
        self.builder.start_shape_query(marker).unwrap();
        // send the markers
        match shape_query_params.skip_markers {
            SkipMarkers::All => {
                self.builder.skip_all_markers().unwrap();
            }
            SkipMarkers::Set(set) => {
                for marker in set {
                    self.builder.skip_marker(marker).unwrap();
                }
            }
        }
        // then interpret the shape query parameters
        for (param, _) in shape_query_params.params {
            match param {
                ShapeQueryParam::Node(node_param) => {
                    let node_id = node_param.name.0;
                    let param_type = S::convert_node_type(node_param.node_type.0);

                    // we differentiate between an existing node, in which case we issue an expected value change,
                    // or a new one, in which case it must be a single.

                    // TODO: since we are scoping the identifiers here,
                    //  we need to enter a new scope and reset a scope when entering if/else branches. (i guess in interpret_block?)
                    //  since we should *not* expect a value change for a node defined in a distinct scope.

                    if let Some(aid) = self.node_id_to_aid(node_id) {
                        // issue an expected value change
                        self.builder
                            .expect_shape_node_change(aid, param_type)
                            .unwrap();
                    } else {
                        // must be a new node
                        let name = node_id.must_single();
                        let aid = AbstractNodeId::dynamic_output(marker, name);
                        self.builder
                            .expect_shape_node(name.into(), param_type)
                            .unwrap();
                        self.single_node_aids.insert(name, aid);
                    }
                }
                ShapeQueryParam::Edge(edge_param) => {
                    let src = edge_param.src.0;
                    let dst = edge_param.dst.0;

                    let src_aid = self.node_id_to_aid(src).unwrap_or_else(|| {
                        AbstractNodeId::dynamic_output(marker, src.must_single())
                    });
                    let dst_aid = self.node_id_to_aid(dst).unwrap_or_else(|| {
                        AbstractNodeId::dynamic_output(marker, dst.must_single())
                    });

                    let typ = S::convert_edge_type(edge_param.edge_type.0);
                    self.builder
                        .expect_shape_edge(src_aid, dst_aid, typ)
                        .unwrap();
                }
            }
        }
    }

    fn get_new_shape_query_marker(&mut self) -> String {
        let marker = format!("shape_query_{}", self.shape_query_counter);
        self.shape_query_counter += 1;
        marker
    }

    fn interpret_let_stmt(&mut self, (let_stmt, _): Spanned<LetStmt<'src>>) {
        if let_stmt.bang {
            let result_name = let_stmt.ident.0;
            let (op_like, args) = self.call_expr_to_op_like(let_stmt.call);
            self.builder
                .add_bang_operation(result_name, op_like, args)
                .unwrap();
            let new_aid = AbstractNodeId::named(result_name);
            self.single_node_aids.insert(result_name, new_aid);
        } else {
            let op_name = let_stmt.ident.0;
            let (op_like, args) = self.call_expr_to_op_like(let_stmt.call);

            self.interpret_op_like(Some(op_name), op_like, args);
        }
    }

    fn query_name_to_builtin_query(
        &self,
        query_name: &str,
        args: Option<Spanned<MacroArgs>>,
    ) -> S::BuiltinQuery {
        let args = args.map(|(args, _)| args);
        S::find_builtin_query(query_name, args).expect("Query name not found in builtins")
    }

    fn op_name_to_op_like(
        &self,
        op_name: &str,
        args: Option<Spanned<MacroArgs>>,
    ) -> BuilderOpLike<S> {
        // TODO: do we want to enforce consumption of a Some(macro_args)?

        // we don't care about the span
        let args = args.map(|(args, _)| args);

        // first try lib builtin
        if let Some(op) = find_lib_builtin_op::<S>(op_name, args) {
            return BuilderOpLike::LibBuiltin(op);
        }

        // then try client builtin
        if let Some(op) = S::find_builtin_op(op_name, args) {
            return BuilderOpLike::Builtin(op);
        }

        if op_name == self.self_name {
            // if the operation name is the same as the function name, we must be recursing
            return BuilderOpLike::Recurse;
        }

        // otherwise must be a user defined operation
        let op_id = self.fn_names_to_op_ids.get(op_name).expect(&format!(
            "Operation name {op_name} not found in function definitions"
        ));

        BuilderOpLike::FromOperationId(*op_id)
    }

    fn interpret_op_like(
        &mut self,
        op_name: Option<&'src str>,
        op_like: BuilderOpLike<S>,
        args: Vec<AbstractNodeId>,
    ) {
        if let Some(op_name) = op_name {
            self.builder
                .add_named_operation(op_name.into(), op_like, args)
                .unwrap();
        } else {
            self.builder.add_operation(op_like, args).unwrap();
        }
    }

    fn call_expr_to_op_like(
        &mut self,
        (call_expr, _): Spanned<FnCallExpr<'src>>,
    ) -> (BuilderOpLike<S>, Vec<AbstractNodeId>) {
        let args = call_expr
            .args
            .into_iter()
            .map(|(arg, _)| self.node_id_to_aid(arg).unwrap())
            .collect();

        let op_name = call_expr.name.0;
        let macro_args = call_expr.macro_args;

        let op_like = self.op_name_to_op_like(op_name, macro_args);

        (op_like, args)
    }

    fn interpret_return(&mut self, (return_stmt, _): Spanned<ReturnStmt<'src, S::CS>>) {
        for (mapping, _) in return_stmt.mapping {
            match mapping {
                ReturnStmtMapping::Node { ret_name, node } => {
                    let aid = self
                        .node_id_to_aid(node.0)
                        .expect("return node AID not found");
                    let ret_name = ret_name.0;
                    let ret_ty = self
                        .return_marker_to_av
                        .get(ret_name)
                        .expect("Return marker not found");
                    self.builder
                        .return_node(aid, ret_name.into(), ret_ty.clone())
                        .unwrap();
                }
                ReturnStmtMapping::Edge { .. } => {
                    todo!("Edge return mappings are not yet supported in the OperationBuilder");
                }
            }
        }
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
