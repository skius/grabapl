use crate::custom_syntax::example::{CustomEdgeType, MyCustomSyntax, MyCustomType};
use crate::custom_syntax::{CustomSyntax, SemanticsWithCustomSyntax};
use crate::interpreter::parse_abstract_node_type;
use crate::{MacroArgs, Span, Token};
use chumsky::error::Rich;
use chumsky::prelude::*;
use chumsky::{Parser, extra, select};
use grabapl::semantics::example_with_ref::{
    EdgeType, ExampleOperation, ExampleQuery, ExampleWithRefSemantics, NodeType, NodeValue,
};
use std::cmp::Ordering;
use std::str::FromStr;
// Hooking it up to ExampleWithRefSemantics:

fn add_node_args_parser<'src>()
-> impl Parser<'src, &'src str, (NodeType, NodeValue), extra::Err<Rich<'src, char, Span>>> {
    any().repeated().to_slice().try_map_with(|src, e| {
        let toks = crate::lexer().parse(src).into_result().map_err(|errs| {
            Rich::custom(
                e.span(),
                format!("Failed to parse arguments: {}, errs: {:?}", src, errs),
            )
        })?;

        let node_typ_parser =
            MyCustomSyntax::get_node_type_parser().try_map_with(|custom_typ, e| {
                ExampleWithRefSemantics::convert_node_type(custom_typ)
                    .ok_or_else(|| Rich::custom(e.span(), format!("node type not supported")))
            });
        // let node_value_parser = select! {
        //     Token::Num(num) => NodeValue::Integer(num),
        // };

        let num_parser = select! {
            Token::Num(num) => num,
        };
        let str_parser = select! {
            Token::Str(s) => s,
        };

        let node_value_parser = just(Token::Ctrl('-'))
            .or_not()
            .then(num_parser)
            .map(|(sign, num)| {
                if sign.is_some() {
                    NodeValue::Integer(-num)
                } else {
                    NodeValue::Integer(num)
                }
            })
            .or(str_parser.map(|s| NodeValue::String(s.to_string())));

        let tuple_parser = node_typ_parser
            .then_ignore(just(Token::Ctrl(',')))
            .then(node_value_parser)
            .map(|(node_type, value)| (node_type, value));

        let toks_input = toks
            .as_slice()
            .map((src.len()..src.len()).into(), |(t, s)| (t, s));

        tuple_parser
            .parse(toks_input)
            .into_result()
            .map_err(|errs| {
                Rich::custom(
                    e.span(),
                    format!("Failed to parse arguments: {}, errs: {:?}", src, errs),
                )
            })
    })
}

impl SemanticsWithCustomSyntax for ExampleWithRefSemantics {
    type CS = MyCustomSyntax;

    fn find_builtin_op(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinOperation> {
        match name.to_lowercase().as_str() {
            "add_node" => {
                let args = args?;
                let args_src = args.0;
                // must parse node_type, value parser
                let (node_type, node_value) =
                    add_node_args_parser().parse(args_src).into_result().ok()?;

                Some(ExampleOperation::AddNode {
                    node_type,
                    value: node_value,
                })
            }
            "add_edge" => {
                let args = args?;
                let args_src = args.0;
                // must parse string
                let str_src = args_src.trim_matches(&['"']).to_string();
                Some(ExampleOperation::AddEdge {
                    node_typ: NodeType::Object,
                    param_typ: EdgeType::Wildcard,
                    target_typ: EdgeType::Exact(str_src.clone()),
                    value: str_src,
                })
            }
            "increment" => Some(ExampleOperation::AddInteger(1)),
            "decrement" => Some(ExampleOperation::AddInteger(-1)),
            "remove_node" => Some(ExampleOperation::DeleteNode),
            "remove_edge" => Some(ExampleOperation::DeleteEdge),
            "copy_value_from_to" => Some(ExampleOperation::CopyValueFromTo),
            "make_ref" => Some(ExampleOperation::MakeRef),
            "extract_ref" => {
                let args_src = args?.0;
                // must parse node type
                let node_type = parse_abstract_node_type::<ExampleWithRefSemantics>(args_src)?;
                let node_type = ExampleWithRefSemantics::convert_node_type(node_type)?;
                Some(ExampleOperation::ExtractRef {
                    expected_inner_typ: node_type,
                })
            }
            _ => None,
        }
    }

    fn find_builtin_query(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinQuery> {
        match name.to_lowercase().as_str() {
            "cmp_fst_snd" => {
                let args = args?;
                let args_src = args.0;
                // must parse ordering
                let cmp = match args_src {
                    ">" => Ordering::Greater.into(),
                    "<" => Ordering::Less.into(),
                    "=" => Ordering::Equal.into(),
                    _ => return None,
                };
                Some(ExampleQuery::CmpFstSnd(cmp))
            }
            "is_zero" => Some(ExampleQuery::ValueEqualTo(NodeValue::Integer(0))),
            "is_eq" => {
                let args_src = args?.0;
                let x = i32::from_str(args_src).ok()?;
                Some(ExampleQuery::ValueEqualTo(NodeValue::Integer(x)))
            }
            _ => None,
        }
    }

    fn convert_node_type(
        x: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType,
    ) -> Option<Self::NodeAbstract> {
        match x {
            MyCustomType::Primitive(name) => match name.to_lowercase().as_str() {
                "string" => Some(NodeType::String),
                "integer" | "int" => Some(NodeType::Integer),
                "object" => Some(NodeType::Object),
                "separate" => Some(NodeType::Separate),
                _ => None,
            },
            MyCustomType::Custom(custom) => {
                if custom.name.to_lowercase() == "ref" {
                    if let [field] = custom.fields.as_slice()
                        && field.name.to_lowercase() == "inner"
                    {
                        let inner_typ =
                            ExampleWithRefSemantics::convert_node_type(field.typ.clone())?;
                        return Some(NodeType::Ref(Box::new(inner_typ)));
                    }
                }
                None
            }
        }
    }

    fn convert_edge_type(
        x: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType,
    ) -> Option<Self::EdgeAbstract> {
        match x {
            CustomEdgeType::Exact(s) => Some(EdgeType::Exact(s)),
            CustomEdgeType::Wildcard => Some(EdgeType::Wildcard),
        }
    }
}
