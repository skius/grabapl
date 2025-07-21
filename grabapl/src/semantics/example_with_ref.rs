//! A 'canonical' example semantics implementation. Mostly used for testing purposes.
//!
//! Defined here for easy reusability elsewhere without running into cyclic crate dependency issues.

use derive_more::From;
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use crate::operation::ConcreteData;
use crate::operation::query::ConcreteQueryOutput;
use crate::operation::signature::parameter::{AbstractOperationOutput, OperationOutput};
use crate::semantics::*;
use crate::util::log;

pub struct ExampleWithRefSemantics;

pub struct NodeMatcher;
impl AbstractMatcher for NodeMatcher {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        if argument == &NodeType::Separate || parameter == &NodeType::Separate {
            // if either is Separate, they can only match themselves.
            return argument == parameter;
        }
        // TODO: check ref behavior?
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
            NodeValue::Reference(_, t) => NodeType::Ref(Box::new(t.clone())),
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

#[derive(Clone, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NodeType {
    String,
    Integer,
    /// Top type.
    #[default]
    Object,
    /// Holds a reference to a node of the inner type.
    Ref(Box<NodeType>),
    /// Not joinable with any of the other.
    Separate,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NodeValue {
    String(String),
    Integer(i32),
    /// A reference to a node with a node type of `NodeType` when this was created.
    // TODO: what if the node type changes in between?
    //  .. maybe it's solved naturally via shape queries
    Reference(NodeKey, NodeType),
}

impl NodeValue {
    pub fn must_string(&self) -> &str {
        match self {
            NodeValue::String(s) => s,
            _ => {
                panic!("type unsoundness: expected a string node value, found integer")
            }
        }
    }

    pub fn must_integer(&self) -> i32 {
        match self {
            NodeValue::Integer(i) => *i,
            _ => {
                panic!("type unsoundness: expected an integer node value, found string")
            }
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
pub enum ExampleOperation {
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
    MakeRef,
    // TODO: honestly, since the type is checked dynamically at shape-query-time anyway, maybe we could just make Ref an atomic type? without storing the inner type.
    //  it's useful to be able to say "this is a ref<int> vs ref<string>", but in practice this point is kind of moot, since you
    //  don't have a guarantee your shape query will match the ref<int> anyway. since the inner value could have been changed to string.
    //  Actually! We would get type safety if there's no "change type" operations. i.e., if write_str and write_int both require a node of correct type.
    ExtractRef {
        expected_inner_typ: NodeType,
    }
}

impl BuiltinOperation for ExampleOperation {
    type S = ExampleWithRefSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            ExampleOperation::NoOp => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::Object)
                    .unwrap();
            }
            ExampleOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("target", op_typ.clone())
                    .unwrap();
            }
            ExampleOperation::SetEdgeTo {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("src", node_typ.clone())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", node_typ.clone())
                    .unwrap();
                param_builder
                    .expect_edge(
                        SubstMarker::from("src"),
                        SubstMarker::from("dst"),
                        op_typ.clone(),
                    )
                    .unwrap();
            }
            ExampleOperation::AddEdge {
                node_typ,
                param_typ: op_typ,
                target_typ,
                value,
            } => {
                param_builder
                    .expect_explicit_input_node("src", node_typ.clone())
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("dst", node_typ.clone())
                    .unwrap();
            }
            ExampleOperation::AddNode { node_type, value } => {}
            ExampleOperation::CopyValueFromTo => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::Object)
                    .unwrap();
            }
            ExampleOperation::SwapValues => {
                param_builder
                    .expect_explicit_input_node("source", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("destination", NodeType::Object)
                    .unwrap();
            }
            ExampleOperation::DeleteNode => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::Object)
                    .unwrap();
            }
            ExampleOperation::DeleteEdge => {
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
            ExampleOperation::AddInteger(i) => {
                param_builder
                    .expect_explicit_input_node("target", NodeType::Integer)
                    .unwrap();
            }
            ExampleOperation::AModBToC => {
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
            ExampleOperation::MakeRef => {
                // We expect any node.
                // TODO: node refs are not subtypes of object, so maybe this is too restrictive?
                param_builder.expect_explicit_input_node("src", NodeType::Object).unwrap();
            }
            ExampleOperation::ExtractRef { expected_inner_typ } => {
                // we expect the reference node
                // TODO: actually, we don't need to expect the expected inner type here,
                //  what we want is to say "we expect a reference node of any type"
                param_builder.expect_explicit_input_node("ref_node", NodeType::Ref(Box::new(expected_inner_typ.clone()))).unwrap();
                // and a node to which we will potentially attach the resulting node.
                param_builder.expect_explicit_input_node("attach_to", NodeType::Object).unwrap();
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
            ExampleOperation::NoOp => {
                // No operation, so no changes to the abstract graph.
            }
            ExampleOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the abstract value of the node to the specified type.
                g.set_node_value(SubstMarker::from("target"), target_typ.clone())
                    .unwrap();
            }
            ExampleOperation::SetEdgeTo {
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
            ExampleOperation::AddEdge {
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
            ExampleOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", node_type.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            ExampleOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            ExampleOperation::SwapValues => {
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
            ExampleOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            ExampleOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            ExampleOperation::AddInteger(i) => {
                // no abstract changes
            }
            ExampleOperation::AModBToC => {
                // no abstract changes
            }
            ExampleOperation::MakeRef => {
                // we return a new node that is a reference to the source node.
                let src_type = g.get_node_value(SubstMarker::from("src")).unwrap().clone();
                g.add_node("result", NodeType::Ref(Box::new(src_type)));
                new_node_names.insert("result".into(), "result".into());
            }
            ExampleOperation::ExtractRef {..} => {
                // no abstract changes.
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
            ExampleOperation::NoOp => {
                // No operation, so no changes to the concrete graph.
            }
            ExampleOperation::SetTo {
                op_typ,
                target_typ,
                value,
            } => {
                // Set the concrete value of the node to the specified value.
                g.set_node_value(SubstMarker::from("target"), value.clone())
                    .unwrap();
            }
            ExampleOperation::SetEdgeTo {
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
            ExampleOperation::AddEdge {
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
            ExampleOperation::AddNode { node_type, value } => {
                // Add a new node with the specified type and value.
                g.add_node("new", value.clone());
                new_node_names.insert("new".into(), "new".into());
            }
            ExampleOperation::CopyValueFromTo => {
                // Copy the value from one node to another.
                let value = g.get_node_value(SubstMarker::from("source")).unwrap();
                g.set_node_value(SubstMarker::from("destination"), value.clone())
                    .unwrap();
            }
            ExampleOperation::SwapValues => {
                // Swap the values of two nodes.
                let value1 = g.get_node_value(SubstMarker::from("source")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("destination")).unwrap();
                let v1 = value1.clone();
                let v2 = value2.clone();
                g.set_node_value(SubstMarker::from("source"), v2).unwrap();
                g.set_node_value(SubstMarker::from("destination"), v1)
                    .unwrap();
            }
            ExampleOperation::DeleteNode => {
                // Delete the node.
                g.delete_node(SubstMarker::from("target")).unwrap();
            }
            ExampleOperation::DeleteEdge => {
                // Delete the edge from source to destination.
                g.delete_edge(SubstMarker::from("src"), SubstMarker::from("dst"))
                    .unwrap();
            }
            ExampleOperation::AddInteger(i) => {
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
            ExampleOperation::AModBToC => {
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
            ExampleOperation::MakeRef => {
                // Create a reference to the source node.
                let src_key = g.get_node_key(&SubstMarker::from("src").into()).unwrap();
                let src_value = g.get_node_value(SubstMarker::from("src")).unwrap();
                let new_node_name = "result";
                let src_type = NodeConcreteToAbstract::concrete_to_abstract(src_value);
                g.add_node(new_node_name, NodeValue::Reference(src_key, src_type));
                new_node_names.insert(new_node_name.into(), new_node_name.into());
            }
            ExampleOperation::ExtractRef { expected_inner_typ } => {
                let ref_node_val = g.get_node_value(SubstMarker::from("ref_node")).unwrap();
                let NodeValue::Reference(ref_node_key, ref_node_type) = ref_node_val else {
                    panic!("type unsoundness: expected a reference node value");
                };

                let attach_to_key = g.get_node_key(&SubstMarker::from("attach_to").into()).unwrap();

                // TODO: check if node is shape hidden.
                if g.graph.node_attr_map.contains_key(ref_node_key) {
                    // only proceed if it contains the ref_node_key
                    let val = g.graph.get_node_attr(*ref_node_key).unwrap();
                    // Check if the reference node type matches the expected inner type.
                    let actual_type = NodeConcreteToAbstract::concrete_to_abstract(val);
                    if NodeMatcher::matches(&actual_type, expected_inner_typ) {
                        // we match. (we allow subtypes: if the stored node is a Int, then a Ref(Obj) still works.
                        // TODO: check if shape hidden
                        // if not shape hidden:
                        g.graph.add_edge(attach_to_key, *ref_node_key, "attached".to_string());
                    } else {
                        log::info!("ExtractRef operation: reference node key {:?} has type {:?}, expected subtype of {:?}", ref_node_key, actual_type, expected_inner_typ);
                    }
                } else {
                    log::info!("ExtractRef operation: reference node key {:?} does not exist in the graph, skipping attachment", ref_node_key);
                }


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
pub enum ExampleQuery {
    ValuesEqual,
    ValueEqualTo(NodeValue),
    CmpFstSnd(MyOrdering),
}

impl BuiltinQuery for ExampleQuery {
    type S = ExampleWithRefSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        match self {
            ExampleQuery::ValuesEqual => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("b", NodeType::Object)
                    .unwrap();
            }
            ExampleQuery::ValueEqualTo(_) => {
                param_builder
                    .expect_explicit_input_node("a", NodeType::Object)
                    .unwrap();
            }
            ExampleQuery::CmpFstSnd(_) => {
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
            ExampleQuery::ValuesEqual => {
                let value1 = g.get_node_value(SubstMarker::from("a")).unwrap();
                let value2 = g.get_node_value(SubstMarker::from("b")).unwrap();
                ConcreteQueryOutput {
                    taken: value1 == value2,
                }
            }
            ExampleQuery::ValueEqualTo(value) => {
                let node_value = g.get_node_value(SubstMarker::from("a")).unwrap();
                ConcreteQueryOutput {
                    taken: node_value == value,
                }
            }
            ExampleQuery::CmpFstSnd(ordering) => {
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

impl Semantics for ExampleWithRefSemantics {
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
    type BuiltinOperation = ExampleOperation;
    type BuiltinQuery = ExampleQuery;
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
