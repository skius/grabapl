use std::cmp::Ordering;
use std::str::FromStr;
use chumsky::{extra, select, IterParser, Parser};
use chumsky::error::Rich;
use chumsky::prelude::*;
use chumsky::input::ValueInput;
use grabapl::semantics::example::{EdgeType, ExampleOperation, ExampleQuery, ExampleSemantics, NodeType, NodeValue};
use crate::custom_syntax::{CustomSyntax, SemanticsWithCustomSyntax};
use crate::{MacroArgs, Span, Token};

#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomSyntax;

// TODO: borrow?
#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomStructField {
    pub name: String,
    pub typ: MyCustomType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MyCustomStruct {
    pub name: String,
    pub fields: Vec<MyCustomStructField>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MyCustomType {
    Primitive(String),
    Custom(MyCustomStruct),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CustomEdgeType {
    Exact(String),
    Wildcard,
}

impl CustomSyntax for MyCustomSyntax {
    type MacroArgType = Vec<String>;
    type AbstractNodeType = MyCustomType;
    type AbstractEdgeType = CustomEdgeType;

    fn get_macro_arg_parser<'src>()
    -> impl Parser<'src, &'src str, Self::MacroArgType, extra::Err<Rich<'src, char, Span>>> + Clone
    {
        // a comma separated list of strings

        any::<&'src str, extra::Err<Rich<'src, char, Span>>>()
            .filter(|c| *c != ']' && *c != ',')
            .repeated()
            .to_slice()
            .separated_by(just(','))
            .collect()
            .map(|args: Vec<&'src str>| args.into_iter().map(String::from).collect())
            .padded()
    }

    fn get_node_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractNodeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone {
        recursive(|my_typ| {
            let field_name = select! {
                Token::Ident(name) => name,
            }
            .labelled("field name");

            let primitive_type = select! {
                Token::Ident(primitive_name) => primitive_name,
            }
            .map(|name: &str| MyCustomType::Primitive(name.to_string()));

            let field_type = my_typ.labelled("field type");

            let field = field_name
                .then_ignore(just(Token::Ctrl(':')))
                .then(field_type)
                .map(|(name, typ)| MyCustomStructField {
                    name: name.to_string(),
                    typ,
                });
            let fields = field
                .separated_by(just(Token::Ctrl(',')))
                .allow_trailing()
                .collect::<Vec<_>>()
                .labelled("fields");

            let struct_name = select! {
                Token::Ident(name) => name,
            }
            .labelled("struct name");

            let entire_struct = struct_name
                .then_ignore(just(Token::Ctrl('{')))
                .then(fields)
                .then_ignore(just(Token::Ctrl('}')))
                .map(|(name, fields)| {
                    MyCustomType::Custom(MyCustomStruct {
                        name: name.to_string(),
                        fields,
                    })
                });

            entire_struct.or(primitive_type)
        })
    }

    fn get_edge_type_parser<
        'src: 'tokens,
        'tokens,
        I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
    >()
    -> impl Parser<'tokens, I, Self::AbstractEdgeType, extra::Err<Rich<'tokens, Token<'src>, Span>>>
    + Clone {
        // * is Wildcard
        // "string" is Exact
        let wildcard = just(Token::Ctrl('*'))
            .to(CustomEdgeType::Wildcard)
            .labelled("wildcard edge type");

        let str_ = select! {
            Token::Str(s) => s,
        };

        let exact = str_
            .map(|s: &'src str| CustomEdgeType::Exact(s.to_string()))
            .labelled("exact edge type");

        wildcard.or(exact)
    }
}

// Hooking it up to ExampleSemantics:

fn add_node_args_parser<'src>()
-> impl Parser<'src, &'src str, (NodeType, NodeValue), extra::Err<Rich<'src, char, Span>>> {
    any().repeated().to_slice().try_map_with(|src, e| {
        let toks = crate::lexer().parse(src).into_result().map_err(|errs| {
            Rich::custom(
                e.span(),
                format!("Failed to parse arguments: {}, errs: {:?}", src, errs),
            )
        })?;

        let node_typ_parser = MyCustomSyntax::get_node_type_parser()
            .try_map_with(|custom_typ, e| ExampleSemantics::convert_node_type(custom_typ).ok_or_else(|| {
                Rich::custom(
                    e.span(),
                    format!("node type not supported"),
                )
            }));
        // let node_value_parser = select! {
        //     Token::Num(num) => NodeValue::Integer(num),
        // };

        let num_parser = select! {
            Token::Num(num) => num,
        };

        let node_value_parser =
            just(Token::Ctrl('-'))
                .or_not()
                .then(num_parser)
                .map(|(sign, num)| {
                    if sign.is_some() {
                        NodeValue::Integer(-num)
                    } else {
                        NodeValue::Integer(num)
                    }
                });

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

impl SemanticsWithCustomSyntax for ExampleSemantics {
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
                _ => {
                    None
                }
            },
            MyCustomType::Custom(_) => {
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
