use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use petgraph::algo::subgraph_isomorphisms_iter;
use petgraph::prelude::DiGraphMap;
use std::hash::RandomState;
use petgraph::algo::isomorphism::subgraph_isomorphisms_iter_with_partial_mapping;
use petgraph::data::{Build, DataMap};
use petgraph::Direction;
use petgraph::visit::{Data, EdgeCount, GetAdjacencyMatrix, GraphBase, GraphProp, GraphRef, IntoEdgeReferences, IntoEdges, IntoEdgesDirected, IntoNeighbors, IntoNeighborsDirected, NodeCompactIndexable, NodeCount, NodeIndexable};

type G = DiGraphMap<u32, (), RandomState>;

#[derive(Clone, Copy)]
struct OneNodeReindexedGraph<'a> {
    g: &'a G,
    input_node_original: usize,
    // Note: typically this will be 0.
    input_node_relabelled: usize,
}

impl<'a> OneNodeReindexedGraph<'a> {
    fn new(g: &'a G, input_node_original: usize, input_node_relabelled: usize) -> Self {
        Self {
            g,
            input_node_original,
            input_node_relabelled,
        }
    }
}

impl<'a> GraphBase for OneNodeReindexedGraph<'a> {
    type EdgeId = <G as GraphBase>::EdgeId;
    type NodeId = <G as GraphBase>::NodeId;
}

impl<'a> NodeIndexable for OneNodeReindexedGraph<'a> {
    fn node_bound(self: &Self) -> usize {
        self.g.node_bound()
    }

    fn to_index(self: &Self, a: Self::NodeId) -> usize {
        let real_idx = self.g.to_index(a);
        match real_idx {
            i if i == self.input_node_original => self.input_node_relabelled,
            i if i == self.input_node_relabelled => self.input_node_original,
            _ => real_idx,
        }
    }

    fn from_index(self: &Self, i: usize) -> Self::NodeId {
        match i {
            i if i == self.input_node_relabelled => self.g.from_index(self.input_node_original),
            i if i == self.input_node_original => self.g.from_index(self.input_node_relabelled),
            _ => self.g.from_index(i),
        }
    }
}

impl<'a> NodeCount for OneNodeReindexedGraph<'a> {
    fn node_count(self: &Self) -> usize {
        self.g.node_count()
    }
}

impl<'a> NodeCompactIndexable for OneNodeReindexedGraph<'a> {}

impl<'a> EdgeCount for OneNodeReindexedGraph<'a> {
    fn edge_count(self: &Self) -> usize {
        self.g.edge_count()
    }
}

impl<'a> Data for OneNodeReindexedGraph<'a> {
    type NodeWeight = <G as Data>::NodeWeight;
    type EdgeWeight = <G as Data>::EdgeWeight;
}

impl<'a> DataMap for OneNodeReindexedGraph<'a> {
    fn node_weight(self: &Self, a: Self::NodeId) -> Option<&Self::NodeWeight> {
        self.g.node_weight(a)
    }

    fn edge_weight(self: &Self, a: Self::EdgeId) -> Option<&Self::EdgeWeight> {
        <G as DataMap>::edge_weight(self.g, a)
    }
}

impl<'a> GetAdjacencyMatrix for OneNodeReindexedGraph<'a> {
    type AdjMatrix = <G as GetAdjacencyMatrix>::AdjMatrix;

    fn adjacency_matrix(self: &Self) -> Self::AdjMatrix {
        self.g.adjacency_matrix()
    }

    fn is_adjacent(self: &Self, matrix: &Self::AdjMatrix, a: Self::NodeId, b: Self::NodeId) -> bool {
        <G as GetAdjacencyMatrix>::is_adjacent(self.g, matrix, a, b)
    }
}

impl<'a> GraphProp for OneNodeReindexedGraph<'a> {
    type EdgeType = <G as GraphProp>::EdgeType;

    fn is_directed(&self) -> bool {
        self.g.is_directed()
    }
}


impl<'a> GraphRef for OneNodeReindexedGraph<'a> {

}

impl<'a> IntoEdgeReferences for OneNodeReindexedGraph<'a> {
    type EdgeRef = <&'a G as IntoEdgeReferences>::EdgeRef;
    type EdgeReferences = <&'a G as IntoEdgeReferences>::EdgeReferences;

    fn edge_references(self) -> Self::EdgeReferences {
        self.g.edge_references()
    }
}

impl<'a> IntoNeighbors for OneNodeReindexedGraph<'a> {
    type Neighbors = <&'a G as IntoNeighbors>::Neighbors;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.g.neighbors(a)
    }
}

impl<'a> IntoNeighborsDirected for OneNodeReindexedGraph<'a> {
    type NeighborsDirected = <&'a G as IntoNeighborsDirected>::NeighborsDirected;

    fn neighbors_directed(self, a: Self::NodeId, dir: Direction) -> Self::NeighborsDirected {
        self.g.neighbors_directed(a, dir)
    }
}

impl<'a> IntoEdges for OneNodeReindexedGraph<'a> {
    type Edges = <&'a G as IntoEdges>::Edges;

    fn edges(self, a: Self::NodeId) -> Self::Edges {
        self.g.edges(a)
    }
}

impl<'a> IntoEdgesDirected for OneNodeReindexedGraph<'a> {
    type EdgesDirected = <&'a G as IntoEdgesDirected>::EdgesDirected;

    fn edges_directed(self, a: Self::NodeId, dir: Direction) -> Self::EdgesDirected {
        self.g.edges_directed(a, dir)
    }
}

fn match_with_input_mapping(query: &G, graph: &G, query_input_idx: u32, graph_input_idx: u32) {
    let mut nm = |a: &u32, b: &u32| {
        // if *a == query_input_idx {
        //     // We only match the designed input node to the user specified graph input node
        //     *b == graph_input_idx
        // } else {
        //     true
        // }
        true
    };
    let mut em = |_a: &(), _b: &()| true;

    // let partial_mapping = [(query_input_idx, graph_input_idx)];
    // let isos = subgraph_isomorphisms_iter_with_partial_mapping(&query, &graph, &mut nm, &mut em, &partial_mapping);

    // let query_wrapped = OneNodeReindexedGraph::new(query, query_input_idx as usize, 0);
    // let graph_wrapped = OneNodeReindexedGraph::new(graph, graph_input_idx as usize, 0);
    let query_wrapped = query;
    let graph_wrapped = graph;

    let isos = subgraph_isomorphisms_iter(&query_wrapped, &graph_wrapped, &mut nm, &mut em);
    let mut isos = isos.unwrap();
    black_box(isos.next().unwrap());
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

    // uuuh do we need induced subgraphs?
    // OOPS! we do!!
    three_children_query_high_input.add_edge(0, 1, ());
    three_children_query_high_input.add_edge(1, 2, ());
    three_children_query_high_input.add_edge(2, 0, ());
    three_children_query_high_input.add_edge(0, 3, ());
    three_children_query_high_input.add_edge(1, 3, ());
    three_children_query_high_input.add_edge(2, 3, ());
    three_children_query_high_input.add_edge(0, 2, ());
    three_children_query_high_input.add_edge(2, 1, ());
    three_children_query_high_input.add_edge(1, 0, ());

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