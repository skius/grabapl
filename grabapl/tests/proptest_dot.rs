use grabapl::Graph;
use petgraph::dot::Dot;
use petgraph::prelude::DiGraphMap;
use proptest::collection::vec;
use proptest::prelude::*;
use std::fmt::Debug;
use std::hash::RandomState;

#[derive(Clone)]
struct MyGraph<N, E>(Graph<N, E>);

impl<N: Debug, E: Debug> Debug for MyGraph<N, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.dot())
    }
}

#[derive(Default)]
struct MyGraphArbParams<N: Debug, E: Debug> {
    max_nodes: usize,
    node_strategy: Option<BoxedStrategy<N>>,
    edge_strategy: Option<BoxedStrategy<E>>,
}

impl<N, E> Arbitrary for MyGraph<N, E>
where
    N: Clone + std::fmt::Debug + 'static + Default,
    E: Clone + std::fmt::Debug + 'static + Default,
{
    type Parameters = MyGraphArbParams<N, E>;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(
        MyGraphArbParams {
            max_nodes,
            node_strategy,
            edge_strategy,
        }: Self::Parameters,
    ) -> Self::Strategy {
        (0..=max_nodes)
            .prop_flat_map(move |nodes| {
                let node_vec = vec(node_strategy.clone().unwrap(), nodes as usize);
                (node_vec, Just(nodes))
            })
            .prop_flat_map(move |(node_values, nodes)| {
                let mut graph = Graph::new();
                for value in node_values {
                    graph.add_node(value);
                }
                let mut edges = vec![];
                for i in 0..nodes {
                    for j in 0..nodes {
                        if i != j && rand::random::<bool>() {
                            let edge_value = edge_strategy.clone().unwrap();
                            edges.push((Just(i), Just(j), edge_value));
                        }
                    }
                }
                (Just(graph), edges)
            })
            .prop_map(move |(mut graph, edges)| {
                for (src, dst, edge_value) in edges {
                    graph.add_edge(src as u32, dst as u32, edge_value);
                }
                MyGraph(graph)
            })
            .boxed()
    }
}

prop_compose! {
    fn arb_digraph(max_node: u32)(nodes in 0..=max_node)
        -> (usize, DiGraphMap<u32, (), RandomState>) {
        let mut graph = DiGraphMap::new();
        for i in 0..nodes {
            graph.add_node(i);
        }
        for i in 0..nodes {
            for j in 0..nodes {
                if i != j && rand::random::<bool>() {
                    graph.add_edge(i, j, ());
                }
            }
        }
        (nodes as usize, graph)
    }
}

prop_compose! {
    fn arb_grabapl_graph(max_node: u32)((nodes, graph) in arb_digraph(max_node))(val_vec in vec(any::<i32>(), nodes..=nodes), graph in Just(graph)) -> Graph<i32, ()> {
        let mut grabapl_graph = Graph::new();
        for (_, val) in graph.nodes().zip(val_vec) {
            grabapl_graph.add_node(val);
        }
        for (src, dst, _) in graph.all_edges() {
            grabapl_graph.add_edge(src, dst, ());
        }
        grabapl_graph
    }
}

proptest! {
    #[test]
    fn test_graph_dot_format(graph in arb_grabapl_graph(10)) {
        let dot_string = graph.dot();
        println!("{}", dot_string);

        assert!(false);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_time: 100000,
        max_shrink_iters: 2000000,
        ..ProptestConfig::default()
    })]
    #[test]
    fn test_arb_impl_graph(graph in any_with::<MyGraph<(), ()>>(MyGraphArbParams {
        max_nodes: 10,
        node_strategy: Some(any::<()>().boxed()),
        edge_strategy: Some(Just(()).boxed()),
    })) {
        let dot_string = graph.0.dot();
        println!("{}", dot_string);

        assert!(dot_string.contains("->"));

    }
}
