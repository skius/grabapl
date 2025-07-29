//! This library defines the 'pluggable' parts of a [`grabapl`] implementation.
//!
//! These are all contained in the [`Semantics`] trait, which this library implements for the
//! [`TheSemantics`] holder type.
//!
//! For the non-pluggable parts of a [`grabapl`] implementation, see the [`grabapl`] crate and documentation.
//!
//! # Type Systems
//! The example semantics we're defining here is made up of a node and edge type system as follows:
//!
//! ## Node Type System
//! We have the following node values (also known as concrete node values) - see [`NodeValue`]:
//! - `i32` - representing integers (1, 2, -3, etc.)
//! - `String` - representing strings ("hello", "world", "", etc.)
//!
//! We have the following node types (also known as abstract node values) - see [`NodeType`]:
//! - `Integer` - representing nodes that hold integer values
//! - `String` - representing nodes that hold string values
//! - `Any` - a wildcard type that matches any node type, i.e., it represents nodes that can hold both integer and string values
//!
//! See [`NodeConcreteToAbstract`] for the implementation of getting the most precise node type of a node value.
//!
//! The type system on those types is induced by the following partially ordered set, visualized as a Hasse diagram:
//! ```text
//!           Any
//!          /   \
//!         /     \
//!     Integer  String
//! ```
//! In other words, `Integer` and `String` are subtypes of `Any`, `Any` is a supertype of both, and
//! a `String` is not a subtype of `Integer` and vice versa. See [`NodeSubtyping`] for the implementation of this type system.
//!
//! ## Edge Type System
//! We have the following edge values (also known as concrete edge values) - see [`EdgeValue`]:
//! - `()` (the unit type) - representing no interesting value besides presence (i.e., just the singleton value `()` of the unit type)
//! - `String` - representing strings ("next", "parent", etc.)
//! - `i32` - representing integers (1, 2, -3, etc.)
//!
//! We have the following edge types (also known as abstract edge values) - see [`EdgeType`]:
//! - `()`, the unit type - representing edges that do not carry any additional value besides presence
//! - `ExactString(s)` - representing edges that carry the specific string value `s`, e.g., `ExactString("next")` represents exactly those edges with a string value of `"next"`.
//! - `String` - representing edges that carry a string value
//! - `Integer` - representing edges that carry an integer value
//! - `Any` - a wildcard type that matches any edge type, i.e., it represents edges that can carry `()`, string, and integer values)
//!
//! See [`EdgeConcreteToAbstract`] for the implementation of getting the most precise edge type of an edge value.
//!
//! The type system on those types is induced by the following partially ordered set, visualized as a Hasse diagram:
//! ```text
//!               ____Any____
//!              /     |     \
//!             /      |      \
//!           ()   String      Integer
//!                / ... \
//! ExactString("a") ...  ExactString("zzzz...")
//! ```
//! In other words, all types are subtypes of `Any`, `ExactString(s)` is a subtype of `String` for any string `s`, and `String` and `Integer` and `()` are not subtypes of each other.
//! and there are no other relationships between the types. See [`EdgeSubtyping`] for the implementation of this type system.
//! The notion of storing a concrete string value inside a type is closely related to (but much weaker than) [refinement types].
//!
//! # Operations and Queries
//! Additionally, every [`Semantics`] implementation can define its own set of "builtin" operations and queries.
//! These are arbitrary Rust functions (or any other language through FFI and/or interpreters) that operate on the graph.
//!
//! ## Builtin Operations
//! Builtin operations are defined by the [`BuiltinOperation`] trait, which we have implemented for the [`TheOperation`] enum.
//!
//! Builtin operations can be used to manipulate the graph, e.g., adding nodes or edges, removing them,
//! changing their values, etc., but also for anything side-effectful, like printing a trace to the console.
//!
//! There is a set of generic operations that are defined in the [`LibBuiltinOperation`] enum, which can be used to
//! perform common operations on the graph independent of the custom semantics.
//!
//! See the [`BuiltinOperation`] trait for more details on how to implement operations.
//!
//! ## Builtin Queries
//! Queries are defined by the [`BuiltinQuery`] trait, which we have implemented for the [`TheQuery`] enum.
//!
//! Queries can be used to retrieve information from the graph that is used to decide which branch of two to take.
//! Essentially, these are the `if` conditions of traditional programming languages.
//! Notably, queries do not return a first-class node value, but rather a value only visible in how the control flow of the program proceeds.
//!
//! See the [`BuiltinQuery`] trait for more details on how to implement queries.
//!
//! For queries that are supposed to change the statically known abstract graph, see [TODO: link to GraphShapeQuery
//!
//! # Optional Features
//! See the [`syntax`](self::syntax) module if you want to use this semantics in concjuction with `grabapl`'s  pluggable
//! syntax parser and interpreter.
//!
//! # Usage
//! Once the semantics is defined, it can be used to build user defined operations and run operations
//! on concrete graphs.
//!
//! Continue in `template/README.md` for the next steps.
//!
//! # Your Turn
//! Feel free to copy this crate and adjust the semantics to your liking!
//!
//! [`grabapl`]: grabapl
//! [refinement types]: https://en.wikipedia.org/wiki/Refinement_type

pub mod syntax;

use grabapl::operation::ConcreteData;
use grabapl::operation::query::{BuiltinQuery, ConcreteQueryOutput};
use grabapl::operation::signature::parameter::{AbstractOperationOutput, OperationOutput};
use grabapl::prelude::*;
use std::collections::HashMap;

/// Defines the semantics of a client implementation via its `Semantics` implementation.
///
/// See the crate-level documentation for more details.
pub struct TheSemantics;

/// The node values used in our example semantics.
///
/// Also known as concrete node values.
#[derive(Clone, derive_more::Debug, PartialEq, Eq)]
pub enum NodeValue {
    /// Represents an integer value.
    #[debug("{_0}")]
    Integer(i32),
    /// Represents a string value.
    #[debug("{_0:?}")]
    String(String),
}

impl From<i32> for NodeValue {
    fn from(value: i32) -> Self {
        NodeValue::Integer(value)
    }
}

impl From<String> for NodeValue {
    fn from(value: String) -> Self {
        NodeValue::String(value)
    }
}

impl From<&str> for NodeValue {
    fn from(value: &str) -> Self {
        NodeValue::String(value.to_string())
    }
}

impl NodeValue {
    /// Returns the most precise node type of this node value.
    pub fn to_type(&self) -> NodeType {
        match self {
            NodeValue::Integer(_) => NodeType::Integer,
            NodeValue::String(_) => NodeType::String,
        }
    }
}

/// The node types used in our example semantics.
///
/// Also known as abstract node values.
#[derive(Clone, derive_more::Debug, PartialEq, Eq, Default)]
pub enum NodeType {
    /// Represents a node that holds an integer value.
    #[debug("int")]
    Integer,
    /// Represents a node that holds a string value.
    #[debug("string")]
    String,
    /// Represents a wildcard type that matches any node type.
    #[debug("any")]
    #[default]
    Any,
}

/// The edge values used in our example semantics.
///
/// Also known as concrete edge values.
#[derive(Clone, derive_more::Debug, PartialEq, Eq, Default)]
pub enum EdgeValue {
    /// Represents the unit type, i.e., no interesting value besides presence.
    #[default]
    #[debug("")]
    Unit,
    /// Represents a string value.
    #[debug("{_0:?}")]
    String(String),
    /// Represents an integer value.
    #[debug("{_0}")]
    Integer(i32),
}

impl From<String> for EdgeValue {
    fn from(value: String) -> Self {
        EdgeValue::String(value)
    }
}

impl From<&str> for EdgeValue {
    fn from(value: &str) -> Self {
        EdgeValue::String(value.to_string())
    }
}

impl From<i32> for EdgeValue {
    fn from(value: i32) -> Self {
        EdgeValue::Integer(value)
    }
}

impl EdgeValue {
    /// Returns the most precise edge type of this edge value.
    pub fn to_type(&self) -> EdgeType {
        match self {
            EdgeValue::Unit => EdgeType::Unit,
            EdgeValue::String(s) => EdgeType::ExactString(s.to_string()),
            EdgeValue::Integer(_) => EdgeType::Integer,
        }
    }
}

/// The edge types used in our example semantics.
///
/// Also known as abstract edge values.
#[derive(Clone, derive_more::Debug, PartialEq, Eq, Default)]
pub enum EdgeType {
    /// Represents an edge that does not carry any additional value besides presence.
    #[debug("")]
    Unit,
    /// Represents an edge that carries a specific string value.
    #[debug("{_0:?}")]
    ExactString(String),
    /// Represents an edge that carries any string value.
    #[debug("string")]
    String,
    /// Represents an edge that carries an integer value.
    #[debug("int")]
    Integer,
    /// Represents a wildcard type that matches any edge type.
    #[debug("*")]
    #[default]
    Any,
}

/// Defines the subtyping relationships between node types via its [`AbstractMatcher`] implementation.
pub struct NodeSubtyping;

impl AbstractMatcher for NodeSubtyping {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            // [anything] <: Any
            (_, NodeType::Any) => true,
            // parameter is not Any, hence the only remaining subtyping case is Integer <: Integer and String <: String,
            // i.e., the types must be equal
            _ => argument == parameter,
        }
    }
}

/// Defines the subtyping relationships between edge types via its [`AbstractMatcher`] implementation.
pub struct EdgeSubtyping;

impl AbstractMatcher for EdgeSubtyping {
    type Abstract = EdgeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            // [anything] <: Any
            (_, EdgeType::Any) => true,
            // ExactString(s) <: String for any string s
            (EdgeType::ExactString(_), EdgeType::String) => true,
            // parameter is not Any or String, hence the only remaining subtyping cases are
            // - Integer <: Integer
            // - () <: ()
            // - ExactString(s) <: ExactString(s) for the same string s
            // i.e., the types must be equal
            _ => argument == parameter,
        }
    }
}

/// Defines the join operation for node types via its [`AbstractJoin`] implementation.
///
/// The join returns the most specific type that is a supertype of both, if it exists.
///
/// See also [https://en.wikipedia.org/wiki/Join_and_meet](https://en.wikipedia.org/wiki/Join_and_meet).
pub struct NodeJoiner;

impl AbstractJoin for NodeJoiner {
    type Abstract = NodeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        match (a, b) {
            // Any is the most general type, so if either type is Any, the join is Any
            (NodeType::Any, _) | (_, NodeType::Any) => Some(NodeType::Any),
            _ => {
                // the only remaining possibilities for a and b are Integer or String.
                // If they are equal, we return that type, otherwise their most specific supertype is Any
                if a == b {
                    Some(a.clone())
                } else {
                    Some(NodeType::Any)
                }
            }
        }
    }
}

/// Defines the join operation for edge types via its [`AbstractJoin`] implementation.
///
/// The join returns the most specific type that is a supertype of both, if it exists.
///
/// See also [https://en.wikipedia.org/wiki/Join_and_meet](https://en.wikipedia.org/wiki/Join_and_meet).
pub struct EdgeJoiner;

impl AbstractJoin for EdgeJoiner {
    type Abstract = EdgeType;

    fn join(a: &Self::Abstract, b: &Self::Abstract) -> Option<Self::Abstract> {
        match (a, b) {
            // Any is the most general type, so if either type is Any, the join is Any
            (EdgeType::Any, _) | (_, EdgeType::Any) => Some(EdgeType::Any),
            // ExactString(s) <: String for any string s
            (EdgeType::ExactString(_), EdgeType::String) => Some(EdgeType::String),
            (EdgeType::String, EdgeType::ExactString(_)) => Some(EdgeType::String),
            // The most specific supertype of two ExactString is either themselves, if they are equal, or String, if they are not.
            (EdgeType::ExactString(s1), EdgeType::ExactString(s2)) if s1 == s2 => {
                Some(EdgeType::ExactString(s1.clone()))
            }
            (EdgeType::ExactString(_), EdgeType::ExactString(_)) => Some(EdgeType::String),
            // all remaining possibilities are either themselves, if they are equal, or Any, if they are not
            _ if a == b => Some(a.clone()),
            _ => Some(EdgeType::Any),
        }
    }
}

/// Defines the most precise node type of a node value via its [`ConcreteToAbstract`] implementation.
pub struct NodeConcreteToAbstract;
impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = NodeValue;
    type Abstract = NodeType;

    fn concrete_to_abstract(concrete: &Self::Concrete) -> Self::Abstract {
        concrete.to_type()
    }
}

/// Defines the most precise edge type of an edge value via its [`ConcreteToAbstract`] implementation.
pub struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = EdgeValue;
    type Abstract = EdgeType;

    fn concrete_to_abstract(concrete: &Self::Concrete) -> Self::Abstract {
        concrete.to_type()
    }
}

/// A value of this type represents a specific builtin operation in the semantics.
///
/// For example, `TheOperation::NewNode("hello")` and `TheOperation::NewNode(5)` represent
/// two different operations with potentially different behavior.
///
/// On the other hand, `TheOperation::AppendSndToFst` is a singleton variant, because all arguments to this operation
/// come via the passed argument nodes.
///
/// The implementation of such a specific builtin operation is given in this type's implementation of the
/// [`BuiltinOperation`] trait.
///
/// See the documentation on each variant for more details on what the operation does and what its signature is.
#[derive(Debug, Clone)]
pub enum TheOperation {
    /// Adds a node with the given value to the graph.
    ///
    /// Signature: `() -> (new: T)`, where `T` is the type of the node value.
    NewNode {
        /// The node value to add.
        value: NodeValue,
    },
    /// Removes the argument node from the graph.
    ///
    /// Signature: `(input: Any) -> ()`
    RemoveNode,
    /// Appends the string contained in the second node value to the first node value.
    ///
    /// Signature: `(first: String, second: String) -> ()`
    AppendSndToFst,
    /// Add the integer contained in the second node value to the first node value.
    ///
    /// Signature: `(first: Integer, second: Integer) -> ()`
    AddSndToFst,
    /// Adds the constant to the argument node's value.
    ///
    /// Signature: `(input: Integer) -> ()`
    AddConstant {
        /// The constant value to add to the first node value.
        constant: i32,
    },
    /// Copies the value from the first node to the second node.
    ///
    /// Signature: `(Any, Any) -> ()`, changes the second node's specific type to the first node's specific type.
    CopyValueFromTo,
    /// Adds an edge with the given value from the first node to the second node.
    ///
    /// If the edge already exists, it is replaced with the new value.
    ///
    /// Signature: `(first: Any, second: Any) -> ()`, adds an edge of the value's type between the two nodes.
    NewEdge {
        /// The edge's value.
        value: EdgeValue,
    },
    /// Removes the edge from the first node to the second node.
    ///
    /// Signature: `(from: Any, to: Any) -> ()`, removes the edge between the two nodes, if one exists.
    RemoveEdge,
    /// Extracts the value from the edge between the first two nodes into the output node.
    /// If the edge is a unit edge, the return node's value is 0.
    ///
    /// Signature: `(from: Any, to: Any, from -> to: Any) -> (value: Any)`, changes `value`'s specific type to the edge's specific type.
    ExtractEdgeToNode,
    /// Returns the length of the string contained in the first node.
    ///
    /// Signature: `(input: String) -> (length: Integer)`, where `length` is the length of the string in the input node.
    StringLength,
}

impl BuiltinOperation for TheOperation {
    type S = TheSemantics;

    /// Returns the operation's parameter.
    ///
    /// This is used to determine the statically how a user defined operation matches its nodes and edges to this operation's parameters.
    /// This is the part of the signature in the [`TheOperation`] variant documentation comments _before_ the `->` arrow.
    ///
    /// # Example
    /// The `NewNode` operation has an empty parameter (`()`), since it expects no input nodes or edges.
    ///
    /// On the other hand, the `AppendSndToFst` operation has a parameter that expects two nodes;
    /// the first node must be a string node and the second node must be a string node as well.
    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        // Note: many operations on the param_builder are fallible, but since we know our parameters are valid, we just unwrap.
        match self {
            TheOperation::NewNode { .. } => {}
            TheOperation::RemoveNode => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::Any)
                    .unwrap();
            }
            TheOperation::AppendSndToFst => {
                param_builder
                    .expect_explicit_input_node("first", NodeType::String)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("second", NodeType::String)
                    .unwrap();
            }
            TheOperation::AddSndToFst => {
                param_builder
                    .expect_explicit_input_node("first", NodeType::Integer)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("second", NodeType::Integer)
                    .unwrap();
            }
            TheOperation::AddConstant { .. } => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::Integer)
                    .unwrap();
            }
            TheOperation::CopyValueFromTo => {
                param_builder
                    .expect_explicit_input_node("first", NodeType::Any)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("second", NodeType::Any)
                    .unwrap();
            }
            TheOperation::NewEdge { .. } => {
                param_builder
                    .expect_explicit_input_node("from", NodeType::Any)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("to", NodeType::Any)
                    .unwrap();
            }
            TheOperation::RemoveEdge => {
                param_builder
                    .expect_explicit_input_node("from", NodeType::Any)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("to", NodeType::Any)
                    .unwrap();
                // if we wanted to only allow this operation to be called if we statically know that there is an edge between the two nodes,
                // then we could uncomment the following line:
                // param_builder.expect_edge("from", "to", EdgeType::Any).unwrap();
            }
            TheOperation::ExtractEdgeToNode => {
                param_builder
                    .expect_explicit_input_node("from", NodeType::Any)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("to", NodeType::Any)
                    .unwrap();
                // this time we _must_ have an edge in order to extract its value.
                param_builder
                    .expect_edge("from", "to", EdgeType::Any)
                    .unwrap();
            }
            TheOperation::StringLength => {
                param_builder
                    .expect_explicit_input_node("input", NodeType::String)
                    .unwrap();
            }
        }
        // we `expect` here, because we know that our parameters should always be valid.
        param_builder
            .build()
            .expect("Failed to build operation parameter")
    }

    /// Defines the operation's behavior on an abstract graph.
    ///
    /// This function must always soundly capture the operation's behavior in the concrete, i.e., in [`BuiltinOperation::apply`].
    /// See [`BuiltinOperation::apply_abstract`] for more details on sound approximations.
    ///
    /// Furthermore, the modifications done on the passed abstract graph must be communicated via the return value.
    ///
    /// For many purposes, sticking to the top exposed functions of the [`GraphWithSubstitution`] is sufficient for the return value.
    /// These will store the changes done to be returned at the end of this method.
    ///
    /// We will need to take manual care to ensure that our [`TheOperation::apply`] implementation does not do anything that we do not describe in this method.
    ///
    /// Note that this allows you to describe more complex abstract behavior than possible in a user defined operation.
    ///
    /// For example, the `CopyValueFromTo` operation is implemented in a way that it expects two `Any` nodes,
    /// but it will actually "know" the specific type of the first node in the given abstract graph and
    /// will change the second node's type to that of the first node. A user defined operation could
    /// only change the second node's type to `Any`, due to how modularity works in the language.
    /// I.e., the builtin operation can turn a `(Int, String)` argument into `(Int, Int)`, while a user defined operation can
    /// only turn it into `(Int, Any)`. (Note that read-only nodes do not change their type, even in user defined operations).
    fn apply_abstract(
        &self,
        g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>,
    ) -> AbstractOperationOutput<Self::S> {
        let mut local_names_to_output_names = HashMap::new();
        match self {
            TheOperation::NewNode { value } => {
                // We add a new node with the given value to the graph.
                g.add_node("some_name", value.to_type());
                // Now we tell the caller that we added a new node under the name "new". The name "some_name" is just a local placeholder for us.
                local_names_to_output_names.insert("some_name".into(), "new".into());
            }
            TheOperation::RemoveNode => {
                // We remove the node that is passed to the parameter we have called "input".
                // Nodes from parameters (as opposed to newly added nodes within this method call) are indicated by `SubstMarker`.
                g.delete_node(SubstMarker::from("input"));
            }
            TheOperation::AppendSndToFst => {
                // Abstractly we don't do any changes here. The second node will still be a String node.
            }
            TheOperation::AddSndToFst => {
                // Abstractly we don't do any changes here. The second node will still be an Integer node.
            }
            TheOperation::AddConstant { .. } => {
                // Abstractly we don't do any changes here. The node will still be an Integer node.
            }
            TheOperation::CopyValueFromTo => {
                // We know that in the concrete, we will copy the value from the first node to the second node.
                // This means that the type from the first node will be valid to be used for the second node.
                // Note: don't be confused by this function being called `get_node_*value*`.
                // The "value" in the name is simply because the wrapper type is also used in the concrete graph,
                // so it refers to [abstract] or [concrete] values, depending on the context.
                // Note: We unwrap here, because we know the node must exist, since we have asked for it in the parameter.
                let first_node_type = g.get_node_value(SubstMarker::from("first")).unwrap();
                // We change the second node's type to the first node's type.
                g.set_node_value(SubstMarker::from("second"), first_node_type.clone());
            }
            TheOperation::NewEdge { value } => {
                // We add a new edge with the given value from the first node to the second node.
                g.add_edge(
                    SubstMarker::from("from"),
                    SubstMarker::from("to"),
                    value.to_type(),
                );
                // Note that contrary to the `NewNode` operation, we do not need to return a new edge by name here,
                // since an edge will always be unique between two nodes. Also, we cannot name edges directly.
            }
            TheOperation::RemoveEdge => {
                // We remove the edge from the first node to the second node.
                g.delete_edge(SubstMarker::from("from"), SubstMarker::from("to"));
            }
            TheOperation::ExtractEdgeToNode => {
                // We extract the edge from the first node to the second node.
                let edge_type = g
                    .get_edge_value(SubstMarker::from("from"), SubstMarker::from("to"))
                    .unwrap();
                let node_type = match edge_type {
                    // We said () edges get turned into 0
                    EdgeType::Unit | EdgeType::Integer => NodeType::Integer,
                    EdgeType::ExactString(_) | EdgeType::String => NodeType::String,
                    EdgeType::Any => NodeType::Any,
                };
                g.add_node("value", node_type);
                local_names_to_output_names.insert("value".into(), "value".into());
            }
            TheOperation::StringLength => {
                // We add a new node that will hold the length of the string in the input node.
                g.add_node("length", NodeType::Integer);
                local_names_to_output_names.insert("length".into(), "length".into());
            }
        }
        g.get_abstract_output(local_names_to_output_names)
    }

    /// Defines the operation's behavior on a concrete graph.
    ///
    /// This must always execute a 'subset of effects' of the [`TheOperation::apply_abstract`] method.
    /// For example, if the abstract effect says that a node may receive a new integer value, then
    /// we're not allowed to write a string to the node.
    fn apply(
        &self,
        g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>,
        concrete_data: &mut ConcreteData,
    ) -> OperationOutput {
        let mut local_names_to_output_names = HashMap::new();
        match self {
            TheOperation::NewNode { value } => {
                // We add a new node with the given value to the graph.
                g.add_node("some_name", value.clone());
                // Now we tell the caller that we added a new node under the name "new". The name "some_name" is just a local placeholder for us.
                // "new" must be the same name as we gave in the abstract operation.
                local_names_to_output_names.insert("some_name".into(), "new".into());
            }
            TheOperation::RemoveNode => {
                // We remove the node that is passed to the parameter we have called "input".
                g.delete_node(SubstMarker::from("input"));
            }
            TheOperation::AppendSndToFst => {
                // We append the second node's string value to the first node's string value.
                let first_value = g.get_node_value(SubstMarker::from("first")).unwrap();
                let second_value = g.get_node_value(SubstMarker::from("second")).unwrap();
                if let (NodeValue::String(first), NodeValue::String(second)) =
                    (first_value, second_value)
                {
                    let new_value = format!("{}{}", first, second);
                    g.set_node_value(SubstMarker::from("first"), NodeValue::String(new_value));
                } else {
                    log::error!(
                        "AppendSndToFst: expected both nodes to be strings, but got {:?} and {:?}",
                        first_value,
                        second_value
                    );
                }
            }
            TheOperation::AddSndToFst => {
                // We add the second node's integer value to the first node's integer value.
                let first_value = g.get_node_value(SubstMarker::from("first")).unwrap();
                let second_value = g.get_node_value(SubstMarker::from("second")).unwrap();
                if let (NodeValue::Integer(first), NodeValue::Integer(second)) =
                    (first_value, second_value)
                {
                    let new_value = first + second;
                    g.set_node_value(SubstMarker::from("first"), NodeValue::Integer(new_value));
                } else {
                    log::error!(
                        "AddSndToFst: expected both nodes to be integers, but got {:?} and {:?}",
                        first_value,
                        second_value
                    );
                }
            }
            TheOperation::AddConstant { constant } => {
                // We add the constant to the first node's integer value.
                let input_value = g.get_node_value(SubstMarker::from("input")).unwrap();
                if let NodeValue::Integer(input) = input_value {
                    let new_value = input + constant;
                    g.set_node_value(SubstMarker::from("input"), NodeValue::Integer(new_value));
                } else {
                    log::error!(
                        "AddConstant: expected input node to be an integer, but got {:?}",
                        input_value
                    );
                }
            }
            TheOperation::CopyValueFromTo => {
                // We copy the value from the first node to the second node.
                let first_value = g.get_node_value(SubstMarker::from("first")).unwrap();
                g.set_node_value(SubstMarker::from("second"), first_value.clone());
            }
            TheOperation::NewEdge { value } => {
                // We add a new edge with the given value from the first node to the second node.
                g.add_edge(
                    SubstMarker::from("from"),
                    SubstMarker::from("to"),
                    value.clone(),
                );
                // Note that contrary to the `NewNode` operation, we do not need to return a new edge by name here,
                // since an edge will always be unique between two nodes. Also, we cannot name edges directly.
            }
            TheOperation::RemoveEdge => {
                // We remove the edge from the first node to the second node.
                g.delete_edge(SubstMarker::from("from"), SubstMarker::from("to"));
            }
            TheOperation::ExtractEdgeToNode => {
                // We extract the edge from the first node to the second node.
                let edge_value = g
                    .get_edge_value(SubstMarker::from("from"), SubstMarker::from("to"))
                    .unwrap();
                let node_value = match edge_value {
                    EdgeValue::Unit => NodeValue::Integer(0), // unit edges are represented as 0
                    EdgeValue::String(s) => NodeValue::String(s.clone()),
                    EdgeValue::Integer(i) => NodeValue::Integer(*i),
                };
                g.add_node("value", node_value);
                local_names_to_output_names.insert("value".into(), "value".into());
            }
            TheOperation::StringLength => {
                // We get the string value from the input node and calculate its length.
                let input_value = g.get_node_value(SubstMarker::from("input")).unwrap();
                let length = if let NodeValue::String(s) = input_value {
                    s.len() as i32 // convert to i32
                } else {
                    log::error!(
                        "StringLength: expected input node to be a string, but got {:?}",
                        input_value
                    );
                    // note: we would be fine to panic and crash here. The static guarantees of grabapl
                    // are enough to ensure that this operation is only called on a string node.
                    0
                };
                g.add_node("length", NodeValue::Integer(length));
                local_names_to_output_names.insert("length".into(), "length".into());
            }
        }
        g.get_concrete_output(local_names_to_output_names)
    }
}

/// A value of this type represents a specific builtin query in the semantics.
///
/// For example, `TheQuery::IsEq(5)` represents a query that checks if a node's value is equal to 5.
/// On the other hand, `TheQuery::Equal` is a singleton variant, because it checks if two nodes are equal without any additional parameters.
///
/// The implementation of such a specific builtin query is given in this type's implementation of the [`BuiltinQuery`] trait.
/// See the documentation on each variant for more details on what the query checks and what its signature is.
///
/// Note that the signatures of the variants do not include a return type: the "return type" is always
/// whether the then branch or the else branch is taken.
///
/// For operations that are supposed to modify the graph and/or return new nodes or edges, see [`TheOperation`].
#[derive(Debug, Clone)]
pub enum TheQuery {
    /// Checks if the node's value is equal to the given value.
    ///
    /// Signature: `(input: Any)`
    IsEq {
        /// The value to compare the node's value to.
        value: NodeValue,
    },
    /// Checks if the two nodes are equal.
    ///
    /// Signature: `(first: Any, second: Any)`
    Equal,
    /// Compares the first node's integer value to the second node's integer value according to the given comparison.
    ///
    /// Signature: `(first: Integer, second: Integer)`
    CompareInt {
        /// Which comparison to perform.
        cmp: IntComparison,
    },
}

/// The different ways to compare integer values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntComparison {
    /// Checks if the first integer is less than the second.
    Lt,
    /// Checks if the first integer is greater than the second.
    Gt,
    /// Checks if the first integer is equal to the second.
    Eq,
    /// Checks if the first integer is less than or equal to the second.
    Lte,
    /// Checks if the first integer is greater than or equal to the second.
    Gte,
}

impl BuiltinQuery for TheQuery {
    type S = TheSemantics;

    /// The query's parameter.
    ///
    /// See [`TheOperation::parameter`] for more details on parameters.
    fn parameter(&self) -> OperationParameter<Self::S> {
        let mut param_builder = OperationParameterBuilder::new();
        // Note: many operations on the param_builder are fallible, but since we know our parameters are valid, we just unwrap.
        match self {
            TheQuery::IsEq { value } => {
                // we could decide to request the value's specific type here, but taking `Any` instead
                // potentially makes for a better user experience.
                param_builder
                    .expect_explicit_input_node("input", NodeType::Any)
                    .unwrap();
            }
            TheQuery::Equal => {
                param_builder
                    .expect_explicit_input_node("first", NodeType::Any)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("second", NodeType::Any)
                    .unwrap();
            }
            TheQuery::CompareInt { cmp } => {
                param_builder
                    .expect_explicit_input_node("first", NodeType::Integer)
                    .unwrap();
                param_builder
                    .expect_explicit_input_node("second", NodeType::Integer)
                    .unwrap();
            }
        }
        // we `expect` here, because we know that our parameters should always be valid.
        param_builder
            .build()
            .expect("Failed to build query parameter")
    }

    /// Defines the query's behavior on an abstract graph.
    ///
    /// Our query does not modify the abstract graph, so this method does not do anything.
    ///
    /// See [`TheOperation::apply_abstract`] for more details on abstract changes.
    fn apply_abstract(&self, g: &mut GraphWithSubstitution<AbstractGraph<Self::S>>) {
        // We probably want to enforce non-modifying queries across the entire language, but I have not
        // asserted that position yet.
        // So for now, this method exists.
    }

    /// Defines the query's behavior on a concrete graph and returns which branch to take of the query.
    ///
    /// See [`TheOperation::apply`] for more details on concrete changes.
    fn query(&self, g: &mut GraphWithSubstitution<ConcreteGraph<Self::S>>) -> ConcreteQueryOutput {
        // `true` if we take the then branch, `false` if we take the else branch.
        let mut taken = false;

        match self {
            TheQuery::IsEq { value } => {
                // We check if the node's value is equal to the given value.
                let input_value = g.get_node_value(SubstMarker::from("input")).unwrap();
                taken = input_value == value;
            }
            TheQuery::Equal => {
                // We check if the two nodes are equal.
                let first_value = g.get_node_value(SubstMarker::from("first")).unwrap();
                let second_value = g.get_node_value(SubstMarker::from("second")).unwrap();
                taken = first_value == second_value;
            }
            TheQuery::CompareInt { cmp } => {
                // We compare the first node's integer value to the second node's integer value according to the given comparison.
                let first_value = g.get_node_value(SubstMarker::from("first")).unwrap();
                let second_value = g.get_node_value(SubstMarker::from("second")).unwrap();

                if let (NodeValue::Integer(first), NodeValue::Integer(second)) =
                    (first_value, second_value)
                {
                    taken = match cmp {
                        IntComparison::Lt => first < second,
                        IntComparison::Gt => first > second,
                        IntComparison::Eq => first == second,
                        IntComparison::Lte => first <= second,
                        IntComparison::Gte => first >= second,
                    };
                } else {
                    log::error!(
                        "CompareInt: expected both nodes to be integers, but got {:?} and {:?}",
                        first_value,
                        second_value
                    );
                    // again, would be fine to crash here, since we should never enter this.
                }
            }
        }

        ConcreteQueryOutput { taken }
    }
}

impl Semantics for TheSemantics {
    type NodeConcrete = NodeValue;
    type NodeAbstract = NodeType;
    type EdgeConcrete = EdgeValue;
    type EdgeAbstract = EdgeType;
    type NodeMatcher = NodeSubtyping;
    type EdgeMatcher = EdgeSubtyping;
    type NodeJoin = NodeJoiner;
    type EdgeJoin = EdgeJoiner;
    type NodeConcreteToAbstract = NodeConcreteToAbstract;
    type EdgeConcreteToAbstract = EdgeConcreteToAbstract;
    type BuiltinOperation = TheOperation;
    type BuiltinQuery = TheQuery;

    /// Returns `Some(NodeType::Any)`, because that is the top node type in our example semantics.
    ///
    /// See the [crate-level documentation](self) for more details on the node type system.
    fn top_node_abstract() -> Option<Self::NodeAbstract> {
        Some(NodeType::Any)
    }

    /// Returns `Some(EdgeType::Any)`, because that is the top edge type in our example semantics.
    ///
    /// See the [crate-level documentation](self) for more details on the edge type system.
    fn top_edge_abstract() -> Option<Self::EdgeAbstract> {
        Some(EdgeType::Any)
    }
}
