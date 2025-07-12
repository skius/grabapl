/*
TODO s from old `match to pattern:

TODO: Implement priority based on closeness of siblings. If the pattern expects two siblings, then we should prefer in A->{B,C,D} the subgraph A->{B,C} or A->{C,D} over A->{B,D}.
    We should however also support A->{D,A} as mapping for example, since we want circular orders.
TODO: I propose doing this via a hard and soft check of orders:
    * The hard check checks that there is no going back and forth for >2 siblings, or, in other words, for some picked starting point of the circular order, the remaining children are in-order of at most a full loop.
    * The soft check prioritizes the returned results such that the first child is preferably also the first child, and any siblings are as close as possible to the input node. If we want to expand this definition, we could say we proceed in BFS order.
TODO: Add circular order to the child order


TODO: Add option to ignore parent order?


*/

/* === Old notes === */


// TODO: should we instead have an 'AbstractAttribute' as well, and the pattern matcher works on that?
// From every concrete graph you can get its abstract graph. That should be like the type.
// so a concrete i32 attr node (say '5') would for example get mapped into a 'i32' node.
// Hmm. Then you would have operations acting on both concrete values but also abstract values.
// For example, an operation might take i32 i32 ANY as input, and turn it into i32 i32 i32. (this is the example of arg3 <- arg1 + arg2)
// this should be statically describable?
// But queries also need a place here. A pattern query definitely returns a node with abstract values, since that's
// the same 'language' that operation inputs speak where patterns are also used, but how do we do a query like "has equal values"?
// such a query would need to be on the concrete level.
// Aah - this does not matter. Queries at runtime typically dont result in value changes, instead they influence the control flow.
// So, 'concrete' queries and 'pattern' queries are unified:
// 1. statically, a query takes as input some abstract graph. This needs to match its expected pattern, so it works exactly like operations.
//    * then, it can produce static changes to the abstract graph, per branch.
//    * This is 'typed', so like a match arm in rust.
// 2. at runtime, these inputs are then replaced by concrete values.
//    * the concrete values decide where the control flow goes and in case of match-arms, which concrete
//      values to bind.
// In other words, a query needs both a concrete and an abstract implementation. I think this is the same as operations: they need the concrete changes, and the abstract pattern + if they change any types
//
//  ** UPDATE: **
// Because we'll want to work abstractly with a pattern graph, we'll want the pattern type to be the type that pattern matches against.
// In other words, we want the pattern type to be the analogue of the PL-"type", with subtyping. eg. a wildcard is just the analogue of the Top type

use grabapl::SubstMarker;

pub struct WithSubstMarker<T> {
    marker: SubstMarker,
    value: T,
}

// TODO: figure out what to do for PatternKind/PatternWrapper
pub enum PatternKind {
    Input,
    Derived,
}

pub struct PatternWrapper<P> {
    pattern: P,
    marker: SubstMarker,
    kind: PatternKind,
}



impl<P> PatternWrapper<P> {
    pub fn new_input(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper {
            pattern,
            marker,
            kind: PatternKind::Input,
        }
    }

    pub fn new_derived(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper {
            pattern,
            marker,
            kind: PatternKind::Derived,
        }
    }

    pub fn get_pattern(&self) -> &P {
        &self.pattern
    }

    pub fn get_marker(&self) -> SubstMarker {
        self.marker.clone()
    }

    pub fn get_kind(&self) -> &PatternKind {
        &self.kind
    }
}

impl<T> WithSubstMarker<T> {
    pub fn new(marker: SubstMarker, value: T) -> Self {
        WithSubstMarker { marker, value }
    }

    pub fn get_value(&self) -> &T {
        &self.value
    }
}

// pub struct InputPattern<NPA: PatternAttributeMatcher, EPA: PatternAttributeMatcher> {
//     pub parameter_nodes: Vec<SubstMarker>,
//     pub pattern_graph: Graph<WithSubstMarker<NPA::Pattern>, EPA::Pattern>,
//     subst_to_node_keys: HashMap<SubstMarker, NodeKey>,
// }
//
// pub struct OperationInput<NA, EA> {
//     pub selected_inputs: Vec<NodeKey>,
//     pub graph: Graph<NA, EA>,
// }
//
// /// A trait for graph operations.
// ///
// /// The operation requires graphs with the given node and edge attribute types.
// pub trait Operation<NPA: PatternAttributeMatcher, EPA: PatternAttributeMatcher> {
//     /// The pattern to match against the graph.
//     fn input_pattern(&self) -> InputPattern<NPA, EPA>;
//     fn apply(
//         &mut self,
//         input: &mut OperationInput<NPA::Attr, EPA::Attr>,
//         subst: &HashMap<SubstMarker, NodeKey>,
//     ) -> Result<(), String>;
// }
//
// impl<NA: Clone, EA: Clone> Graph<NA, EA> {
//     pub fn run_operation<O, NPA, EPA>(
//         &mut self,
//         selected_inputs: Vec<NodeKey>,
//         op: &mut O,
//     ) -> Result<(), String>
//     where
//         O: Operation<NPA, EPA>,
//         NPA: PatternAttributeMatcher<Attr = NA>,
//         EPA: PatternAttributeMatcher<Attr = EA>,
//     {
//         let subst = {
//             let pattern = op.input_pattern(); // TODO: rename a to pattern b to data or similar...
//             let mut nm = |a: &NodeKey, b: &NodeKey| {
//                 let a_attr = pattern.get_node_attr(*a).unwrap();
//                 let b_attr = self.get_node_attr(*b).unwrap();
//                 NPA::matches(b_attr, &a_attr.value)
//             };
//             let mut em = |a: &EdgeAttribute<EPA::Pattern>, b: &EdgeAttribute<EA>| {
//                 EPA::matches(&b.edge_attr, &a.edge_attr)
//             };
//             let Some(mut mappings) = self.match_to_pattern(&pattern, &mut nm, &mut em) else {
//                 return Err("No matching pattern found".to_string());
//             };
//             let mapping = mappings.next().ok_or("Internal Error: No mapping found")?;
//             mapping
//                 .iter()
//                 .map(|(src, target)| (pattern.get_node_attr(*src).unwrap().marker, *target))
//                 .collect::<HashMap<_, _>>()
//         };
//
//         let mut op_input = OperationInput {
//             selected_inputs,
//             // TODO: get rid of clone
//             graph: self.clone(),
//         };
//
//         op.apply(&mut op_input, &subst)?;
//         Ok(())
//     }
// }


fn main() {}
