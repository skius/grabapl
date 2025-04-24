use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use petgraph::algo::subgraph_isomorphisms_iter;
use petgraph::prelude::DiGraphMap;
use std::hash::RandomState;
use petgraph::visit::GraphRef;

type G = DiGraphMap<u32, (), RandomState>;

struct OneNodeRelabelledGraph {
    graph: G,
    input_node_original: u32,
    // Note: typically this will be 0.
    input_node_relabelled: u32,
}

fn match_with_input_mapping(query: &G, graph: &G, query_input_idx: u32, graph_input_idx: u32) {
    let mut nm = |a: &u32, b: &u32| {
        if *a == query_input_idx {
            // We only match the designed input node to the user specified graph input node
            *b == graph_input_idx
        } else {
            true
        }
    };
    let mut em = |_a: &(), _b: &()| true;

    let isos = subgraph_isomorphisms_iter(&query, &graph, &mut nm, &mut em);
    let mut isos = isos.unwrap();
    black_box(isos.next());
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut single_node_query = G::new();
    single_node_query.add_node(0);

    let mut graph = G::new();
    for i in 0..100 {
        graph.add_node(i);
    }
    for i in 0..100 {
        for j in 0..100 {
            if i != j {
                graph.add_edge(i, j, ());
            }
        }
    }

    for graph_input_idx in [0, 10, 20, 50, 70, 99] {
        c.bench_with_input(
            BenchmarkId::new("match_single_node", graph_input_idx), &graph_input_idx,
            |b, i| b.iter(|| {
                match_with_input_mapping(black_box(&single_node_query), black_box(&graph), black_box(0), black_box(*i))
            }),
        );
    }


    let mut three_children_query_high_input = G::new();
    // children of the input node
    three_children_query_high_input.add_node(0);
    three_children_query_high_input.add_node(1);
    three_children_query_high_input.add_node(2);
    // the input node
    three_children_query_high_input.add_node(3);
    
    three_children_query_high_input.add_edge(3, 0, ());
    three_children_query_high_input.add_edge(3, 1, ());
    three_children_query_high_input.add_edge(3, 2, ());

    for graph_input_idx in [0, 10, 20, 50, 70, 99] {
        c.bench_with_input(
            BenchmarkId::new("match_three_children_high_query_input", graph_input_idx), &graph_input_idx,
            |b, i| b.iter(|| {
                match_with_input_mapping(black_box(&three_children_query_high_input), black_box(&graph), black_box(3), black_box(*i))
            }),
        );
    }

    let mut three_children_query_low_input = G::new();
    // the input node
    three_children_query_low_input.add_node(0);
    // children of the input node
    three_children_query_low_input.add_node(1);
    three_children_query_low_input.add_node(2);
    three_children_query_low_input.add_node(3);

    three_children_query_low_input.add_edge(0, 1, ());
    three_children_query_low_input.add_edge(0, 2, ());
    three_children_query_low_input.add_edge(0, 3, ());

    for graph_input_idx in [0, 10, 20, 50, 70, 99] {
        c.bench_with_input(
            BenchmarkId::new("match_three_children_low_query_input", graph_input_idx), &graph_input_idx,
            |b, i| b.iter(|| {
                match_with_input_mapping(black_box(&three_children_query_low_input), black_box(&graph), black_box(0), black_box(*i))
            }),
        );
    }
    
    let mut path_10_high_query_input = G::new();
    // children of the input node
    path_10_high_query_input.add_node(0);
    path_10_high_query_input.add_node(1);
    path_10_high_query_input.add_node(2);
    path_10_high_query_input.add_node(3);
    path_10_high_query_input.add_node(4);
    path_10_high_query_input.add_node(5);
    path_10_high_query_input.add_node(6);
    path_10_high_query_input.add_node(7);
    path_10_high_query_input.add_node(8);
    // the input node
    path_10_high_query_input.add_node(9);
    
    path_10_high_query_input.add_edge(0, 1, ());
    path_10_high_query_input.add_edge(1, 2, ());
    path_10_high_query_input.add_edge(2, 3, ());
    path_10_high_query_input.add_edge(3, 4, ());
    path_10_high_query_input.add_edge(4, 5, ());
    path_10_high_query_input.add_edge(5, 6, ());
    path_10_high_query_input.add_edge(6, 7, ());
    path_10_high_query_input.add_edge(7, 8, ());
    path_10_high_query_input.add_edge(8, 9, ());

    for graph_input_idx in [0, 10, 20, 50, 70, 99] {
        c.bench_with_input(
            BenchmarkId::new("match_path_10_high_query_input", graph_input_idx), &graph_input_idx,
            |b, i| b.iter(|| {
                match_with_input_mapping(black_box(&path_10_high_query_input), black_box(&graph), black_box(9), black_box(*i))
            }),
        );
    }

    let mut path_10_low_query_input = G::new();
    // the input node
    path_10_low_query_input.add_node(0);
    // children of the input node
    path_10_low_query_input.add_node(1);
    path_10_low_query_input.add_node(2);
    path_10_low_query_input.add_node(3);
    path_10_low_query_input.add_node(4);
    path_10_low_query_input.add_node(5);
    path_10_low_query_input.add_node(6);
    path_10_low_query_input.add_node(7);
    path_10_low_query_input.add_node(8);
    path_10_low_query_input.add_node(9);

    path_10_low_query_input.add_edge(1, 2, ());
    path_10_low_query_input.add_edge(2, 3, ());
    path_10_low_query_input.add_edge(3, 4, ());
    path_10_low_query_input.add_edge(4, 5, ());
    path_10_low_query_input.add_edge(5, 6, ());
    path_10_low_query_input.add_edge(6, 7, ());
    path_10_low_query_input.add_edge(7, 8, ());
    path_10_low_query_input.add_edge(8, 9, ());
    path_10_low_query_input.add_edge(9, 0, ());

    for graph_input_idx in [0, 10, 20, 50, 70, 99] {
        c.bench_with_input(
            BenchmarkId::new("match_path_10_low_query_input", graph_input_idx), &graph_input_idx,
            |b, i| b.iter(|| {
                match_with_input_mapping(black_box(&path_10_low_query_input), black_box(&graph), black_box(0), black_box(*i))
            }),
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);