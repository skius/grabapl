use std::collections::HashMap;
use grabapl::*;

enum EdgePattern {
    Wildcard,
    Exact(String),
}

struct EdgeMatcher;

impl PatternAttributeMatcher for EdgeMatcher {
    type Attr = String;
    type Pattern = EdgePattern;

    fn matches(attr: &Self::Attr, pattern: &Self::Pattern) -> bool {
        match pattern {
            EdgePattern::Wildcard => true,
            EdgePattern::Exact(p) => attr == p,
        }
    }
}

struct I32Matcher;
impl PatternAttributeMatcher for I32Matcher {
    type Attr = i32;
    type Pattern = ();

    fn matches(attr: &Self::Attr, pattern: &Self::Pattern) -> bool {
        true
    }
}

enum BuiltinOperation {
    AddNode,
    AppendChild,
    /// Labels nodes of a three-cycle with 1,2,3, and requires the edge between 3 and 1 to be labelled "cycle"
    IndexCycle,
}

impl Operation<I32Matcher, EdgeMatcher> for BuiltinOperation {
    fn input_pattern(&self) -> Graph<WithSubstMarker<()>, EdgePattern> {
        match self {
            BuiltinOperation::AddNode => Graph::new(),
            BuiltinOperation::AppendChild => {
                // Expects a child
                let mut g = Graph::new();
                g.add_node(WithSubstMarker::new(0, ()));
                g
            },
            BuiltinOperation::IndexCycle => {
                let mut g = Graph::new();
                let a = g.add_node(WithSubstMarker::new(0, ()));
                let b = g.add_node(WithSubstMarker::new(1, ()));
                let c = g.add_node(WithSubstMarker::new(2, ()));
                g.add_edge(a, b, EdgePattern::Wildcard);
                g.add_edge(b, c, EdgePattern::Wildcard);
                g.add_edge(c, a, EdgePattern::Exact("cycle".to_string()));
                g
            }
        }
    }
    
    fn apply(
        &mut self,
        graph: &mut Graph<i32, String>,
        subst: &HashMap<SubstMarker, NodeKey>
    ) -> Result<(), String> {
        match self {
            BuiltinOperation::AddNode => {
                graph.add_node(i32::default());
                Ok(())
            }
            BuiltinOperation::AppendChild => {
                let child = graph.add_node(i32::default());
                let parent = subst[&0];
                graph.add_edge(parent, child, "".to_string());
                Ok(())
            }
            BuiltinOperation::IndexCycle => {
                let a = subst[&0];
                let b = subst[&1];
                let c = subst[&2];
                *graph.get_mut_node_attr(a).unwrap() = 1;
                *graph.get_mut_node_attr(b).unwrap() = 2;
                *graph.get_mut_node_attr(c).unwrap() = 3;
                Ok(())
            }
        }
    }
}

#[test]
fn simple_integration() {
    let mut collector = DotCollector::new();

    let mut graph: Graph<i32, String> = Graph::new();
    collector.collect(&graph);
    
    let mut op = BuiltinOperation::AddNode;
    graph.run_operation(&mut op).unwrap();
    collector.collect(&graph);
    
    let mut op = BuiltinOperation::AddNode;
    graph.run_operation(&mut op).unwrap();
    collector.collect(&graph);
    
    let mut op = BuiltinOperation::AppendChild;
    graph.run_operation(&mut op).unwrap();

    
    // TODO: oops. Probably need to add another wrapper for the pattern that defines actual "input" nodes.
    // Run operation would then take input nodes explicitly. We could probably leverage the 'node matcher'
    // functionality to require matching inputs in the pattern to the passed inputs.
}