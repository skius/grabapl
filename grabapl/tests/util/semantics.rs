use chumsky::prelude::*;
use grabapl::operation::query::BuiltinQuery;
use grabapl::operation::BuiltinOperation;
use grabapl::semantics::{
    AbstractJoin, AbstractMatcher, ConcreteToAbstract,
};
use grabapl::Semantics;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use syntax::custom_syntax::SemanticsWithCustomSyntax;
pub mod helpers;

pub use grabapl::semantics::example::*;

pub use ExampleOperation as TestOperation;
pub use ExampleSemantics as TestSemantics;
pub use ExampleQuery as TestQuery;
use syntax::custom_syntax::CustomSyntax;