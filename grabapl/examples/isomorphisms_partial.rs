use criterion::black_box;
use petgraph::algo::general_subgraph_monomorphisms_iter;
use petgraph::algo::isomorphism::general_subgraph_monomorphisms_iter_with_partial_mapping;
use petgraph::graphmap::DiGraphMap;
use std::hash::RandomState;

type G = DiGraphMap<u32, (), RandomState>;

fn match_with_input_mapping<'a>(
    query: &'a G,
    graph: &'a G,
    query_input_idx: u32,
    graph_input_idx: u32,
    gen_all: bool,
    partial: bool,
) {
    let mut nm = move |a: &u32, b: &u32| {
        if *a == query_input_idx {
            // We only match the designed input node to the user specified graph input node
            *b == graph_input_idx
        } else {
            true
        }
    };
    let mut em = |_a: &(), _b: &()| true;

    let partial_mapping = [(query_input_idx, graph_input_idx)];
    let query_wrapped = query;
    let graph_wrapped = graph;

    macro_rules! handle_iter {
        ($iter:expr) => {
            let mut isos = $iter;
            if gen_all {
                let all = isos.collect::<Vec<_>>();
                assert!(all.len() > 0);
                println!("Mappings ({}):\n{:?}", all.len(), all);
                black_box(all);
            } else {
                let first = isos.next().unwrap();
                println!("First mapping:\n{:?}", first);
                black_box(first);
            }
        };
    }

    if partial {
        let isos = general_subgraph_monomorphisms_iter_with_partial_mapping(
            &query_wrapped,
            &graph_wrapped,
            &mut nm,
            &mut em,
            &partial_mapping,
        );
        handle_iter!(isos.unwrap());
    } else {
        let isos =
            general_subgraph_monomorphisms_iter(&query_wrapped, &graph_wrapped, &mut nm, &mut em);
        handle_iter!(isos.unwrap());
    };
}

fn main() {
    let (query_high, query_high_idx) = {
        let mut query_high = G::new();
        // children of the input node
        query_high.add_node(0);
        query_high.add_node(1);
        query_high.add_node(2);
        // the input node
        query_high.add_node(3);

        query_high.add_edge(3, 0, ());
        query_high.add_edge(3, 1, ());
        query_high.add_edge(3, 2, ());
        (query_high, 3)
    };
    let (query_low, query_low_idx) = {
        let mut query_low = G::new();
        // the input node
        query_low.add_node(0);
        // children of the input node
        query_low.add_node(1);
        query_low.add_node(2);
        query_low.add_node(3);

        query_low.add_edge(0, 1, ());
        query_low.add_edge(0, 2, ());
        query_low.add_edge(0, 3, ());
        (query_low, 0)
    };

    for (qg, qi, qkind) in [(query_low, query_low_idx, "low")] {
        for partial in [false, true] {
            for gen_all in [true] {
                for graph_input_idx in [99] {
                    println!(
                        "Running: {},gidx:{},p:{},ga:{}",
                        qkind, graph_input_idx, partial, gen_all
                    );
                    // Generate the data graph. It is a complete graph, except for the input node which only has the desired amount of children.
                    // That means the number of expected output mappings should just be the number of permutations on 3 children.

                    let num_nodes = 1000;

                    let mut g = G::new();
                    for i in 0..num_nodes {
                        g.add_node(i);
                    }
                    for i in 0..num_nodes {
                        for j in 0..num_nodes {
                            if i == j {
                                continue;
                            }
                            if i == graph_input_idx {
                                // only add three outgoing edges
                                if graph_input_idx == 0 {
                                    if j > 3 || j == 0 {
                                        // 0 has children 1,2,3
                                        continue;
                                    }
                                } else {
                                    if j > 2 {
                                        // non 0 has children 0,1,2
                                        continue;
                                    }
                                }
                            }
                            g.add_edge(i, j, ());
                        }
                    }

                    if graph_input_idx == 99 {
                        assert!(g.edge_weight(99, 3).is_none());
                    }

                    match_with_input_mapping(
                        black_box(&qg),
                        black_box(&g),
                        black_box(qi),
                        black_box(graph_input_idx),
                        gen_all,
                        partial,
                    );
                }
            }
        }
    }
}
