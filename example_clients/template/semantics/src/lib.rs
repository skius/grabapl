//! This library defines the 'pluggable' parts of a [`grabapl`] implementation.
//!
//! These are all contained in the [`Semantics`] trait, which this library implements for the
//! [`TheSemantics`] holder type.
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
//! [`grabapl`]: grabapl
//! [refinement types]: https://en.wikipedia.org/wiki/Refinement_type


use grabapl::prelude::*;

/// Defines the semantics of a client implementation via its `Semantics` implementation.
///
/// See the crate-level documentation for more details.
pub struct TheSemantics;

/// The node values used in our example semantics.
///
/// Also known as concrete node values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeValue {
    /// Represents an integer value.
    Integer(i32),
    /// Represents a string value.
    String(String),
    // TODO: decide if we want NodeReferences here.
    //  ah: node references can be simulated via a node that may or may not have an edge to the pointee.
}

/// The node types used in our example semantics.
///
/// Also known as abstract node values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeType {
    /// Represents a node that holds an integer value.
    Integer,
    /// Represents a node that holds a string value.
    String,
    /// Represents a wildcard type that matches any node type.
    Any,
}

/// The edge values used in our example semantics.
///
/// Also known as concrete edge values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeValue {
    /// Represents the unit type, i.e., no interesting value besides presence.
    Unit,
    /// Represents a string value.
    String(String),
    /// Represents an integer value.
    Integer(i32),
}

/// The edge types used in our example semantics.
///
/// Also known as abstract edge values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeType {
    /// Represents an edge that does not carry any additional value besides presence.
    Unit,
    /// Represents an edge that carries a specific string value.
    ExactString(String),
    /// Represents an edge that carries any string value.
    String,
    /// Represents an edge that carries an integer value.
    Integer,
    /// Represents a wildcard type that matches any edge type.
    Any,
}

/// Defines the subtyping relationships between node types via its [`AbstractMatcher`] implementation.
pub struct NodeSubtyping;

impl AbstractMatcher for NodeSubtyping {
    type Abstract = NodeType;

    fn matches(argument: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (argument, parameter) {
            // [anything] <: Any
            (_ , NodeType::Any) => true,
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
            (EdgeType::ExactString(s1), EdgeType::ExactString(s2)) if s1 == s2 => Some(EdgeType::ExactString(s1.clone())),
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
        match concrete {
            NodeValue::Integer(_) => NodeType::Integer,
            NodeValue::String(_) => NodeType::String,
        }
    }
}

/// Defines the most precise edge type of an edge value via its [`ConcreteToAbstract`] implementation.
pub struct EdgeConcreteToAbstract;
impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = EdgeValue;
    type Abstract = EdgeType;

    fn concrete_to_abstract(concrete: &Self::Concrete) -> Self::Abstract {
        match concrete {
            EdgeValue::Unit => EdgeType::Unit,
            EdgeValue::String(s) => EdgeType::ExactString(s.clone()),
            EdgeValue::Integer(_) => EdgeType::Integer,
        }
    }
}

/// A value of this type represents a specific builtin operation in the semantics.
///
/// For example, `BuiltinOperation::AddNode("hello")` and `BuiltinOperation::AddNode(5)` represent
/// two different operations with potentially different behavior.
///
/// The implementation of such a specific builtin operation is given in this type's implementation of the
/// [`BuiltinOperation`](grabapl::prelude::BuiltinOperation) trait.
pub enum BuiltinOperation {
    /// Adds a node with the given value to the graph.
    AddNode {
        /// The node value to add.
        value: NodeValue,
    }
}

// impl Semantics for TheSemantics {
//     type NodeConcrete = NodeValue;
//     type NodeAbstract = NodeType;
//     type EdgeConcrete = EdgeValue;
//     type EdgeAbstract = EdgeType;
//     type NodeMatcher = NodeSubtyping;
//     type EdgeMatcher = EdgeSubtyping;
//     type NodeJoin = NodeJoiner;
//     type EdgeJoin = EdgeJoiner;
//     type NodeConcreteToAbstract = NodeConcreteToAbstract;
//     type EdgeConcreteToAbstract = EdgeConcreteToAbstract;
//     type BuiltinOperation = BuiltinOperation;
//     type BuiltinQuery = ();
// }