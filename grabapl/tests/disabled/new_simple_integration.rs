use std::collections::HashMap;
use grabapl::graph::semantics::{AbstractGraph, AbstractMatcher, AnyMatcher, ConcreteGraph, ConcreteToAbstract, Semantics};
use grabapl::{DotCollector, EdgeInsertionOrder, OperationContext, WithSubstMarker};
use grabapl::graph::operation::run_operation;
use grabapl::graph::pattern::{OperationArgument, OperationParameter, ParameterSubstition};

struct SampleSemantics;

enum EdgePattern {
    Wildcard,
    Exact(String),
}

struct EdgeMatcher;
impl AbstractMatcher for EdgeMatcher {
    type Abstract = EdgePattern;
    fn matches(arg: &Self::Abstract, parameter: &Self::Abstract) -> bool {
        match (arg, parameter) {
            (_, EdgePattern::Wildcard) => true,
            (EdgePattern::Exact(a), EdgePattern::Exact(b)) => a == b,
            (_, _) => false,
        }
    }
}

struct NodeConcreteToAbstract;
struct EdgeConcreteToAbstract;

impl ConcreteToAbstract for NodeConcreteToAbstract {
    type Concrete = i32;
    type Abstract = ();

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        ()
    }
}

impl ConcreteToAbstract for EdgeConcreteToAbstract {
    type Concrete = String;
    type Abstract = EdgePattern;

    fn concrete_to_abstract(c: &Self::Concrete) -> Self::Abstract {
        EdgePattern::Exact(c.clone())
    }
}

impl Semantics for SampleSemantics {
    type NodeConcrete = i32;
    type NodeAbstract = ();
    type EdgeConcrete = String;
    type EdgeAbstract = EdgePattern;
    type NodeMatcher = AnyMatcher<()>;
    type EdgeMatcher = EdgeMatcher;

    type NodeConcreteToAbstract = NodeConcreteToAbstract;
    type EdgeConcreteToAbstract = EdgeConcreteToAbstract;

    type BuiltinOperation = BuiltinOperation;
}

enum BuiltinOperation {
    AddNode,
    AppendChild,
    /// Labels nodes of a three-cycle with 1,2,3, and requires the edge between 3 and 1 to be labelled "cycle"
    /// Only the first node is used as explicit input, the others are inferred.
    IndexCycle,
}

impl grabapl::graph::operation::BuiltinOperation for BuiltinOperation {
    type S = SampleSemantics;

    fn parameter(&self) -> OperationParameter<Self::S> {
        match self {
            BuiltinOperation::AddNode => {
                let mut g = grabapl::graph::Graph::new();
                OperationParameter {
                    explicit_input_nodes: vec![],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::new(),
                }
            }
            BuiltinOperation::AppendChild => {
                // Expects a child
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    // TODO: this is scary, because NodeKeys are not a newtype yet, and neither are SubstMarkers.
                    subst_to_node_keys: HashMap::from([(0, a)]),
                }
            }
            BuiltinOperation::IndexCycle => {
                let mut g = grabapl::graph::Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                let b = g.add_node(WithSubstMarker::new(1, ()));
                let c = g.add_node(WithSubstMarker::new(2, ()));
                g.add_edge(a, b, EdgePattern::Wildcard);
                g.add_edge(b, c, EdgePattern::Wildcard);
                g.add_edge(c, a, EdgePattern::Exact("cycle".to_string()));
                OperationParameter {
                    explicit_input_nodes: vec![0],
                    parameter_graph: g,
                    subst_to_node_keys: HashMap::from([(0, a), (1, b), (2, c)]),
                }
            }
        }
    }

    fn apply_abstract(&self, g: &mut AbstractGraph<Self::S>, argument: OperationArgument, substitution: &ParameterSubstition) {
        match self {
            BuiltinOperation::AddNode => {
                g.add_node(());
            }
            BuiltinOperation::AppendChild => {
                let parent = substitution.mapping[&0];
                let child = g.add_node(());
                // TODO: this EdgePattern is weird.
                //  On the one hand, we know for a fact this is an exact "" that will be added, so in type-theory, we correctly add the most precise type (Exact instead of Wildcard)
                //  But if this ever used as a _pattern_ (parameter), it is a *decision* we're making here. Exact will permit fewer matches.
                //  Realistically this is not a problem, because we don't run builtin operations on parameters. But we should be careful.
                g.add_edge_ordered(parent, child, EdgePattern::Exact("".to_string()), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
            }
            BuiltinOperation::IndexCycle => {
                // Nothing happens abstractly. Dynamically values change, but the abstract graph stays.
            }
        }
    }

    fn apply(
        &self,
        graph: &mut ConcreteGraph<SampleSemantics>,
        argument: OperationArgument,
        substitution: &grabapl::graph::pattern::ParameterSubstition,
    ) {
        match self {
            BuiltinOperation::AddNode => {
                graph.add_node(0);
            }
            BuiltinOperation::AppendChild => {
                let parent = substitution.mapping[&0];
                let child = graph.add_node(0);
                graph.add_edge_ordered(parent, child, "".to_string(), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
            }
            BuiltinOperation::IndexCycle => {
                let a = substitution.mapping[&0];
                let b = substitution.mapping[&1];
                let c = substitution.mapping[&2];
                *graph.get_mut_node_attr(a).unwrap() = 1;
                *graph.get_mut_node_attr(b).unwrap() = 2;
                *graph.get_mut_node_attr(c).unwrap() = 3;
            }
        }
    }
}

#[test]
fn new_simple_integration() {
    let operation_ctx = HashMap::from([
        (0, BuiltinOperation::AddNode),
        (1, BuiltinOperation::AppendChild),
        (2, BuiltinOperation::IndexCycle),
    ]);
    let operation_ctx = OperationContext::from_builtins(operation_ctx);

    let mut dot_collector = DotCollector::new();

    let mut g = SampleSemantics::new_concrete_graph();
    dot_collector.collect(&g);
    let a = g.add_node(1);
    dot_collector.collect(&g);
    let b = g.add_node(2);
    dot_collector.collect(&g);
    g.add_edge(a, b, "edge".to_string());
    dot_collector.collect(&g);

    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 1, vec![2]).unwrap();
    dot_collector.collect(&g);

    // add 3 new nodes
    // 4
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    // 5
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);
    // 6
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 0, vec![]).unwrap();
    dot_collector.collect(&g);

    // add cycle
    g.add_edge(6, 4, "cycle".to_string());
    dot_collector.collect(&g);
    // add edge
    g.add_edge(4, 5, "anything1".to_string());
    dot_collector.collect(&g);
    // add edge
    g.add_edge(5, 6, "anything2".to_string());
    dot_collector.collect(&g);
    
    // add other children to 4 that are ignored
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 1, vec![4]).unwrap();
    dot_collector.collect(&g);
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 1, vec![4]).unwrap();
    dot_collector.collect(&g);
    
    // run cycle operation
    run_operation::<SampleSemantics>(&mut g, &operation_ctx, 2, vec![4]).unwrap();
    dot_collector.collect(&g);



    println!("{}", dot_collector.finalize());

    assert!(false);
}
