use chumsky::prelude::*;
use derive_more::From;
use grabapl::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::operation::signature::parameter::{
    AbstractOperationOutput, GraphWithSubstitution, OperationOutput, OperationParameter,
};
use grabapl::operation::signature::parameterbuilder::OperationParameterBuilder;
use grabapl::operation::{BuiltinOperation, ConcreteData};
use grabapl::semantics::{
    AbstractGraph, AbstractJoin, AbstractMatcher, ConcreteGraph, ConcreteToAbstract,
};
use grabapl::{Semantics, SubstMarker};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use syntax::interpreter::SemanticsWithCustomSyntax;
use syntax::{CustomSyntax, MacroArgs, MyCustomSyntax, MyCustomType, Span, Token};

pub struct TestSemantics;

fn add_node_args_parser<'src>()
-> impl Parser<'src, &'src str, (NodeType, NodeValue), extra::Err<Rich<'src, char, Span>>> {
    any().repeated().to_slice().try_map_with(|src, e| {
        let toks = syntax::lexer().parse(src).into_result().map_err(|errs| {
            Rich::custom(
                e.span(),
                format!("Failed to parse arguments: {}, errs: {:?}", src, errs),
            )
        })?;

        let node_typ_parser = MyCustomSyntax::get_node_type_parser()
            .map(|custom_typ| TestSemantics::convert_node_type(custom_typ));
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

impl SemanticsWithCustomSyntax for TestSemantics {
    type CS = MyCustomSyntax;

    fn find_builtin_op(name: &str, args: Option<MacroArgs>) -> Option<Self::BuiltinOperation> {
        match name.to_lowercase().as_str() {
            "add_node" => {
                let args = args?;
                let args_src = args.0;
                // must parse node_type, value parser
                let (node_type, node_value) =
                    add_node_args_parser().parse(args_src).into_result().ok()?;

                Some(TestOperation::AddNode {
                    node_type,
                    value: node_value,
                })
            }
            "add_edge" => {
                let args = args?;
                let args_src = args.0;
                // must parse string
                let str_src = args_src.trim_matches(&['"']).to_string();
                Some(TestOperation::AddEdge {
                    node_typ: NodeType::Object,
                    param_typ: EdgeType::Wildcard,
                    target_typ: EdgeType::Exact(str_src.clone()),
                    value: str_src,
                })
            }
            "increment" => {
                Some(TestOperation::AddInteger(1))
            }
            "decrement" => {
                Some(TestOperation::AddInteger(-1))
            }
            "remove_node" => Some(TestOperation::DeleteNode),
            "remove_edge" => Some(TestOperation::DeleteEdge),
            "copy_value_from_to" => Some(TestOperation::CopyValueFromTo),
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
                Some(TestQuery::CmpFstSnd(cmp))
            }
            "is_zero" => {
                Some(TestQuery::ValueEqualTo(NodeValue::Integer(0)))
            }
            "is_eq" => {
                let args_src = args?.0;
                let x = i32::from_str(args_src).ok()?;
                Some(TestQuery::ValueEqualTo(NodeValue::Integer(x)))
            }
            _ => None,
        }
    }

    fn convert_node_type(
        x: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractNodeType,
    ) -> Self::NodeAbstract {
        match x {
            MyCustomType::Primitive(name) => match name.to_lowercase().as_str() {
                "string" => NodeType::String,
                "integer" | "int" => NodeType::Integer,
                "object" => NodeType::Object,
                "separate" => NodeType::Separate,
                _ => {
                    panic!("unsupported node type: {name}");
                }
            },
            MyCustomType::Custom(_) => {
                panic!("unsupported")
            }
        }
    }

    fn convert_edge_type(
        x: <<Self as SemanticsWithCustomSyntax>::CS as CustomSyntax>::AbstractEdgeType,
    ) -> Self::EdgeAbstract {
        match x {
            syntax::EdgeType::Exact(s) => EdgeType::Exact(s),
            syntax::EdgeType::Wildcard => EdgeType::Wildcard,
        }
    }
}

pub struct NodeMatcher;
impl AbstractMatcher for NodeMatcher {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        if argument == &NodeType::Separate || parameter == &NodeType::Separate {
            // if either is Separate, they can only match themselves.
            return argument == parameter;
        }
        match (argument, parameter) {
            (_, NodeType::Object) => true,
            _ => argument == parameter,
        }
    }
}

pub struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            (_, EdgeType::Wildcard) => true,
            (EdgeType::Exact(a), EdgeType::Exact(b)) => a == b,
            _ => false,
        }
    }
}

pub struct NodeJoiner;
impl AbstractJoin for NodeJoiner {
    type Abstract = NodeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        if a == b {
            Some(a.clone())
        } else if a != &NodeType::Separate && b != &NodeType::Separate {
            Some(NodeType::Object)
        } else {
            // Separate have no join.
            None
        }
    }
}

pub struct EdgeJoiner;
impl AbstractJoin for EdgeJoiner {
    type Abstract = EdgeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        match (a, b) {
            (EdgeType::Exact(a), EdgeType::Exact(b)) if a == b => Some(EdgeType::Exact(a.clone())),
            _ => Some(EdgeType::Wildcard),
        }
    }
}

pub struct NodeConcreteToAbstract;
impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = NodeValue;
    type Abstract = NodeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        match c {
            NodeValue::String(_) => NodeType::String,
            NodeValue::Integer(_) => NodeType::Integer,
        }
    }
}

pub struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = String;
    type Abstract = EdgeType;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgeType::Exact(c.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NodeType {
    String,
    Integer,
    /// Top type.
    #[default]
    Object,
    /// Not joinable with any of the other.
    Separate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NodeValue {
    String(String),
    Integer(i32),
}

impl NodeValue {
    pub fn must_string(&self) -> &str {
        match self {
            NodeValue::String(s) => s,
            NodeValue::Integer(_) => {
                panic!("type unsoundness: expected a string node value, found integer")
            }
        }
    }

    pub fn must_integer(&self) -> i32 {
        match self {
            NodeValue::String(_) => {
                panic!("type unsoundness: expected an integer node value, found string")
            }
            NodeValue::Integer(i) => *i,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EdgeType {
    Wildcard,
    Exact(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TestOperation {
    NoOp,
    SetTo {
        op_typ: NodeType,
        target_typ: NodeType,
        value: NodeValue,
    },
    SetEdgeTo {
        node_typ: NodeType,
        param_typ: EdgeType,
        target_typ: EdgeType,
        value: String,
    },
    AddEdge {
        node_typ: NodeType,
        // TODO: remove. unused.
        param_typ: EdgeType,
        target_typ: EdgeType,
        value: String,
    },
    AddNode {
        node_type: NodeType,
        value: NodeValue,
    },
    CopyValueFromTo,
    SwapValues,
    DeleteNode,
    DeleteEdge,
    AddInteger(i32),
    AModBToC,
}

impl BuiltinOperation for TestOperation {
    type S = TestSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestOperation::NoOp => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::Object)
                    .unwrap();
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("target", *op_typ)
                    .unwrap();
            }
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("src", *node_typ)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", *node_typ)
                    .unwrap();
                param_builder
                    .expect_edge(
                        SubstMarker::from("src"),
                        SubstMarker::from("dst"),
                        op_typ.clone(),
                    )
                    .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("src", *node_typ)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", *node_typ)
                    .unwrap();
            }
            TestOperation::AddNode { node_type, value } => {}
            TestOperation::CopyValueFromTo => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::Object)
                    .unwrap();
            }
            TestOperation::SwapValues => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::Object)
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::Object)
                    .unwrap();
            }
            TestOperation::DeleteEdge => {
                param_builder
                    .expect_explicit_input_node("src", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_edge(
                        SubstMarker::from("src"),
                        SubstMarker::from("dst"),
                        EdgeType::Wildcard,
                    )
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::Integer)
                    .unwrap();
            }
            TestOperation::AModBToC => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Integer)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::Integer)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("c", NodeType::Integer)
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S> {
        let mut new_node_names = HashMap::new();
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the abstract graph.
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the abstract value of the node to the specified type.
                g.set_node_value(SubstMarker::from("target"), *target_typ)
                    .unwrap();
            }
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Set the edge from source to destination with the specified type.
                g.set_edge_value(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    target_typ.clone(),
                )
                .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Add an edge from source to destination with the specified type.
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    target_typ.clone(),
                );
            }
            TestOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", node_type.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            TestOperation::SwapValues => {
                // Swap the values of two nodes.
                let value1 = g.get_node_value(SubstMarker::from("source")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("destination")).unwrap();
                let v1 = value1.clone();
                let v2 = value2.clone();
                g.set_node_value(SubstMarker::from("source"), v2).unwrap();
                g.set_node_value(SubstMarker::from("destination"), v1)
                    .unwrap();
                // TODO: talk about above problem in user defined op.
                //  Specifically: We take two objects as input, and swap them. Nothing guarantees that the most precise
                //  types of the two nodes are the same, hence the valid inferred signature (if this were an user defined op) would be
                //  that both nodes end up as type Object. However, because this is builtin, it can actually in a sense
                //  "look at" the real, most precise values of the nodes, and just say that it swaps those.
                //  If we didn't have this, we'd need monomorphized swapvaluesInt etc operations, or support for generics.
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                // no abstract changes
            }
            TestOperation::AModBToC => {
                // no abstract changes
            }
        }
        g.get_abstract_output(new_node_names)
    }

    fn apply(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>,
        _: &mut ConcreteData,
    ) -> OperationOutput {
        let mut new_node_names = HashMap::new();
        match self {
            TestOperation::NoOp => {
                // No operation, so no changes to the concrete graph.
            }
            TestOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the concrete value of the node to the specified value.
                g.set_node_value(SubstMarker::from("target"), value.clone())
                    .unwrap();
            }
            TestOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Set the edge from source to destination with the specified value.
                g.set_edge_value(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    value.clone(),
                )
                .unwrap();
            }
            TestOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                // Add an edge from source to destination with the specified value.
                g.add_edge(
                    SubstMarker::from("src"),
                    SubstMarker::from("dst"),
                    value.clone(),
                );
            }
            TestOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", value.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            TestOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            TestOperation::SwapValues => {
                // Swap the values of two nodes.
                let value1 = g.get_node_value(SubstMarker::from("source")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("destination")).unwrap();
                let v1 = value1.clone();
                let v2 = value2.clone();
                g.set_node_value(SubstMarker::from("source"), v2).unwrap();
                g.set_node_value(SubstMarker::from("destination"), v1)
                    .unwrap();
            }
            TestOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            TestOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            TestOperation::AddInteger(i) => {
                // Add an integer to the node.
                let NodeValue::Integer(old_value) =
                    g.get_node_value(SubstMarker::from("target")).unwrap()
                else {
                    panic!(
                        "expected an integer node value for AddInteger operation - type unsoundness"
                    );
                };
                let value = NodeValue::Integer(*i + *old_value);
                g.set_node_value(SubstMarker::from("target"), value)
                    .unwrap();
            }
            TestOperation::AModBToC => {
                // Compute a % b and store it in c.
                let a = g.get_node_value(SubstMarker::from("a")).unwrap();
                let b = g.get_node_value(SubstMarker::from("b")).unwrap();
                let c = g.get_node_value(SubstMarker::from("c")).unwrap();
                let NodeValue::Integer(a_val) = a else {
                    panic!(
                        "expected an integer node value for AModBToC operation - type unsoundness"
                    );
                };
                let NodeValue::Integer(b_val) = b else {
                    panic!(
                        "expected an integer node value for AModBToC operation - type unsoundness"
                    );
                };
                let NodeValue::Integer(c_val) = c else {
                    panic!(
                        "expected an integer node value for AModBToC operation - type unsoundness"
                    );
                };
                let result = a_val % b_val;
                g.set_node_value(SubstMarker::from("c"), NodeValue::Integer(result))
                    .unwrap();
            }
        }
        g.get_concrete_output(new_node_names)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Copy, From)]
pub struct MyOrdering(std::cmp::Ordering);

impl Deref for MyOrdering {
    type Target = std::cmp::Ordering;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TestQuery {
    ValuesEqual,
    ValueEqualTo(NodeValue),
    CmpFstSnd(MyOrdering),
}

impl BuiltinQuery for TestQuery {
    type S = TestSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            TestQuery::ValuesEqual => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::Object)
                    .unwrap();
            }
            TestQuery::ValueEqualTo(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
            }
            TestQuery::CmpFstSnd(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::Object)
                    .unwrap();
            }
        }
        param_builder.build().unwrap()
    }

    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>) {
        // does nothing, not testing side-effect-ful queries here
    }

    fn query(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> ConcreteQueryOutput {
        match self {
            TestQuery::ValuesEqual => {
                let value1 = g.get_node_value(SubstMarker::from("a")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("b")).unwrap();
                ConcreteQueryOutput {
                    taken: value1 == value2,
                }
            }
            TestQuery::ValueEqualTo(value) => {
                let node_value = g.get_node_value(SubstMarker::from("a")).unwrap();
                ConcreteQueryOutput {
                    taken: node_value == value,
                }
            }
            TestQuery::CmpFstSnd(ordering) => {
                let value1 = g.get_node_value(SubstMarker::from("a")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("b")).unwrap();
                let cmp_result = match (value1, value2) {
                    (NodeValue::Integer(a), NodeValue::Integer(b)) => a.cmp(&b),
                    _ => {
                        panic!("type unsoundness: expected integers for comparison");
                    }
                };
                ConcreteQueryOutput {
                    taken: &cmp_result == ordering.deref(),
                }
            }
        }
    }
}

impl Semantics for TestSemantics {
    type NodeConcrete = NodeValue;
    type NodeAbstract = NodeType;
    type EdgeConcrete = String;
    type EdgeAbstract = EdgeType;
    type NodeMatcher = NodeMatcher;
    type EdgeMatcher = EdgeMatcher;
    type NodeJoin = NodeJoiner;
    type EdgeJoin = EdgeJoiner;
    type NodeConcreteToAbstract = NodeConcreteToAbstract;
    type EdgeConcreteToAbstract = EdgeConcreteToAbstract;
    type BuiltinOperation = TestOperation;
    type BuiltinQuery = TestQuery;
}

// additions for serde support
#[cfg(feature = "serde")]
impl Serialize for MyOrdering {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // serialize as -1, 0, or 1 for Less, Equal, Greater
        let value = match self.0 {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        value.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MyOrdering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;
        let ordering = match value {
            -1 => std::cmp::Ordering::Less,
            0 => std::cmp::Ordering::Equal,
            1 => std::cmp::Ordering::Greater,
            _ => return Err(serde::de::Error::custom("invalid ordering value")),
        };
        Ok(MyOrdering(ordering))
    }
}
