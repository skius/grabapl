use chumsky::prelude::*;
use grabapl::Semantics;
use grabapl::operation::BuiltinOperation;
use grabapl::operation::query::BuiltinQuery;
use grabapl::semantics::{AbstractJoin, AbstractMatcher, ConcreteToAbstract};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use syntax::custom_syntax::SemanticsWithCustomSyntax;
pub mod helpers;

pub use grabapl::semantics::example::*;

pub use ExampleOperation as TestOperation;
pub use ExampleQuery as TestQuery;
pub use ExampleSemantics as TestSemantics;
use syntax::custom_syntax::CustomSyntax;
