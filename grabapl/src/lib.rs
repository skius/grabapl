use std::cmp::Ordering;
use petgraph::dot::Dot;
use petgraph::prelude::{DiGraphMap, GraphMap, StableDiGraph};
use petgraph::visit::{IntoEdgesDirected, IntoNeighborsDirected, NodeRef};
use petgraph::{Direction, dot};
use std::collections::HashMap;
use std::fmt::Debug;
use std::iter;
use petgraph::algo::subgraph_isomorphisms_iter;

pub trait PatternAttribute {
    type Attr;
    type Pattern;

    fn matches(attr: &Self::Attr, pattern: &Self::Pattern) -> bool;
}

/// A marker for substitution in the graph.
///
/// Useful for programmatically defined operations to know the substitution of their input pattern.
pub type SubstMarker = u32;

pub struct WithSubstMarker<T> {
    marker: SubstMarker,
    value: T,
}

pub enum PatternKind {
    Input,
    Derived,
}

pub struct PatternWrapper<P> {
    pattern: P,
    marker: SubstMarker,
    kind: PatternKind,
}

// TODO: maybe we could have an input builder? basically we want to have one connected component per input.
// then we allow building an input graph with the builder, but the finalize method checks that we have exactly one input node
// (actually we could enforce that statically via it being the entry point) and that it is in fact weakly connected (ie ignoring edge direction)
// The input pattern for the Operation would then instead be a Vec of those input connected component patterns.

// TODO: What if two separate connected components overlap in the substitution? this leads to 'node references' to some degree.
// Probably only really bad if the 'shape' of that node changes while another reference to it expects something else. eg deleting the node or changing its type

impl<P> PatternWrapper<P> {
    pub fn new_input(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper { pattern, marker, kind: PatternKind::Input }
    }

    pub fn new_derived(pattern: P, marker: SubstMarker) -> Self {
        PatternWrapper { pattern, marker, kind: PatternKind::Derived }
    }

    pub fn get_pattern(&self) -> &P {
        &self.pattern
    }

    pub fn get_marker(&self) -> SubstMarker {
        self.marker
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

/// A trait for graph operations.
///
/// The operation requires graphs with the given node and edge attribute types.
pub trait Operation<NPA: PatternAttribute, EPA: PatternAttribute> {
    /// The pattern to match against the graph.
    fn input_pattern(&self) -> ConcreteGraph<WithSubstMarker<NPA::Pattern>, EPA::Pattern>;
    fn apply(&mut self, graph: &mut ConcreteGraph<NPA::Attr, EPA::Attr>, subst: &HashMap<SubstMarker, NodeKey>) -> Result<(), String>;
}

impl<NA, EA> ConcreteGraph<NA, EA> {
    pub fn run_operation<O, NPA, EPA>(&mut self, op: &mut O) -> Result<(), String>
    where
        O: Operation<NPA, EPA>,
        NPA: PatternAttribute<Attr = NA>,
        EPA: PatternAttribute<Attr = EA>,
    {
        let subst = {
            let pattern = op.input_pattern();
            let mut nm = |a: &NodeKey, b: &NodeKey| {
                let a_attr = pattern.get_node_attr(*a).unwrap();
                let b_attr = self.get_node_attr(*b).unwrap();
                NPA::matches(b_attr, &a_attr.value)
            };
            let mut em = |a: &EdgeAttribute<EPA::Pattern>, b: &EdgeAttribute<EA>| {
                EPA::matches(&b.edge_attr, &a.edge_attr)
            };
            let Some(mut mappings) = self.match_to_pattern(&pattern, &mut nm, &mut em) else {
                return Err("No matching pattern found".to_string());
            };
            let mapping = mappings
                .next()
                .ok_or("Internal Error: No mapping found")?;
            mapping
                .iter()
                .map(|(src, target)| (pattern.get_node_attr(*src).unwrap().marker, *target))
                .collect::<HashMap<_, _>>()
        };

        op.apply(self, &subst)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct NodeAttribute<NodeAttr> {
    pub node_attr: NodeAttr,
    // Additional attributes can be added here
}

impl<NodeAttr> NodeAttribute<NodeAttr> {
    pub fn new(node_attr: NodeAttr) -> Self {
        NodeAttribute { node_attr }
    }
}

#[derive(Debug)]
pub struct EdgeAttribute<EdgeAttr> {
    pub edge_attr: EdgeAttr,
    // Additional attributes can be added here
    /// The order of the edge as an outgoing edge from the source node
    source_out_order: EdgeOrder,
    /// The order of the edge as an incoming edge to the target node
    target_in_order: EdgeOrder,
}

impl<EdgeAttr> EdgeAttribute<EdgeAttr> {
    pub fn new(edge_attr: EdgeAttr, source_out_order: EdgeOrder, target_in_order: EdgeOrder) -> Self {
        EdgeAttribute { edge_attr, source_out_order, target_in_order }
    }
}

type EdgeOrder = i32;

pub type NodeKey = u32;
pub type EdgeKey = (NodeKey, NodeKey);

#[derive(Debug, Copy, Clone)]
pub enum EdgeInsertionOrder {
    Append,
    Prepend,
}

pub struct ConcreteGraph<NodeAttr, EdgeAttr> {
    graph: DiGraphMap<NodeKey, EdgeAttribute<EdgeAttr>>,
    max_node_key: NodeKey,
    node_attr_map: HashMap<NodeKey, NodeAttribute<NodeAttr>>,
}

impl<NodeAttr, EdgeAttr> ConcreteGraph<NodeAttr, EdgeAttr> {
    pub fn new() -> Self {
        ConcreteGraph {
            graph: GraphMap::new(),
            max_node_key: 0,
            node_attr_map: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_attr: NodeAttr) -> NodeKey {
        let node_key = self.max_node_key;
        let node_key = self.graph.add_node(node_key);
        self.node_attr_map
            .insert(node_key, NodeAttribute::new(node_attr));
        self.max_node_key += 1;
        node_key
    }

    /// Returns the old `EdgeAttr` if it exists, otherwise returns `None`.
    ///
    /// Same as `add_edge_ordered` with `Append` for both source and target.
    pub fn add_edge(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: EdgeAttr,
    ) -> Option<EdgeAttr> {
        self.add_edge_ordered(
            source,
            target,
            edge_attr,
            EdgeInsertionOrder::Append,
            EdgeInsertionOrder::Append,
        )
    }

    fn extremum_out_edge_order_key(
        &self,
        source: NodeKey,
        wanted_order: Ordering,
        direction: Direction,
    ) -> Option<EdgeOrder> {
        let mut extremum_order = None;
        for (_, _, edge_attr) in self.graph.edges_directed(source, direction) {
            let order = if direction == Direction::Outgoing {
                edge_attr.source_out_order
            } else {
                edge_attr.target_in_order
            };
            if extremum_order.is_none() || order.cmp(&extremum_order.unwrap()) == wanted_order {
                extremum_order = Some(order);
            }
        }
        extremum_order
    }

    fn max_out_edge_order_key(
        &self,
        source: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Greater, Direction::Outgoing)
    }

    fn min_out_edge_order_key(
        &self,
        source: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(source, Ordering::Less, Direction::Outgoing)
    }

    fn max_in_edge_order_key(
        &self,
        target: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Greater, Direction::Incoming)
    }

    fn min_in_edge_order_key(
        &self,
        target: NodeKey,
    ) -> Option<EdgeOrder> {
        self.extremum_out_edge_order_key(target, Ordering::Less, Direction::Incoming)
    }


    pub fn add_edge_ordered(
        &mut self,
        source: NodeKey,
        target: NodeKey,
        edge_attr: EdgeAttr,
        source_out_order: EdgeInsertionOrder,
        target_in_order: EdgeInsertionOrder,
    ) -> Option<EdgeAttr> {

        let new_out_order = match source_out_order {
            EdgeInsertionOrder::Append => {
                self.max_out_edge_order_key(source).unwrap_or(0) + 1
            }
            EdgeInsertionOrder::Prepend => {
                self.min_out_edge_order_key(source).unwrap_or(0) - 1
            }
        };
        let new_in_order = match target_in_order {
            EdgeInsertionOrder::Append => {
                self.max_in_edge_order_key(target).unwrap_or(0) + 1
            }
            EdgeInsertionOrder::Prepend => {
                self.min_in_edge_order_key(target).unwrap_or(0) - 1
            }
        };

        let old_attr = self
            .graph
            .add_edge(source, target, EdgeAttribute::new(edge_attr, new_out_order, new_in_order));
        old_attr.map(|attr| attr.edge_attr)
    }

    fn neighbors_out_ordered(
        &self,
        source: NodeKey,
    ) -> Vec<NodeKey> {
        let mut neighbors = self
            .graph
            .edges_directed(source, Direction::Outgoing)
            .collect::<Vec<_>>();
        neighbors.sort_by(|(_, _, e1), (_, _, e2)| {
            e1.source_out_order.cmp(&e2.source_out_order)
        });
        neighbors.into_iter()
            .map(|(_, target, _)| target)
            .collect()
    }

    fn neighbors_in_ordered(
        &self,
        target: NodeKey,
    ) -> Vec<NodeKey> {
        let mut neighbors = self
            .graph
            .edges_directed(target, Direction::Incoming)
            .collect::<Vec<_>>();
        neighbors.sort_by(|(_, _, e1), (_, _, e2)| {
            e1.target_in_order.cmp(&e2.target_in_order)
        });
        neighbors.into_iter()
            .map(|(source, _, _)| source)
            .collect()
    }

    pub fn next_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let outgoing_neighbors = self.neighbors_out_ordered(source);
        let curr_idx = outgoing_neighbors
            .iter()
            .position(|&target| target == curr_target)
            .unwrap_or(0);
        let next_idx = (curr_idx + 1) % outgoing_neighbors.len();
        let next_target = outgoing_neighbors[next_idx];
        (source, next_target)
    }

    pub fn prev_outgoing_edge(&self, source: NodeKey, (_, curr_target): EdgeKey) -> EdgeKey {
        let outgoing_neighbors = self.neighbors_out_ordered(source);
        let curr_idx = outgoing_neighbors
            .iter()
            .position(|&target| target == curr_target)
            .unwrap_or(0);
        let prev_idx = if curr_idx == 0 {
            outgoing_neighbors.len() - 1
        } else {
            curr_idx - 1
        };
        let prev_target = outgoing_neighbors[prev_idx];
        (source, prev_target)
    }

    pub fn remove_node(&mut self, node_key: NodeKey) -> Option<NodeAttr> {
        if let Some(node_attr) = self.node_attr_map.remove(&node_key) {
            self.graph.remove_node(node_key);
            Some(node_attr.node_attr)
        } else {
            None
        }
    }

    pub fn remove_edge_between(&mut self, source: NodeKey, target: NodeKey) -> Option<EdgeAttr> {
        self.remove_edge((source, target))
    }

    pub fn remove_edge(&mut self, (src, target): EdgeKey) -> Option<EdgeAttr> {
        self.graph
            .remove_edge(src, target)
            .map(|attr| attr.edge_attr)
    }

    pub fn get_edge_attr(&self, (src, target): EdgeKey) -> Option<&EdgeAttr> {
        self.graph
            .edge_weight(src, target)
            .map(|attr| &attr.edge_attr)
    }

    pub fn get_mut_edge_attr(&mut self, (src, target): EdgeKey) -> Option<&mut EdgeAttr> {
        self.graph
            .edge_weight_mut(src, target)
            .map(|attr| &mut attr.edge_attr)
    }

    pub fn get_node_attr(&self, node_key: NodeKey) -> Option<&NodeAttr> {
        self.node_attr_map
            .get(&node_key)
            .map(|attr| &attr.node_attr)
    }

    pub fn get_mut_node_attr(&mut self, node_key: NodeKey) -> Option<&mut NodeAttr> {
        self.node_attr_map
            .get_mut(&node_key)
            .map(|attr| &mut attr.node_attr)
    }

    pub fn dot(&self) -> String
    where
        EdgeAttr: Debug,
        NodeAttr: Debug,
    {
        format!(
            "{:?}",
            Dot::with_attr_getters(
                &self.graph,
                &[dot::Config::EdgeNoLabel, dot::Config::NodeNoLabel],
                &|g, (src, target, attr)| {
                    let dbg_attr_format = format!("{:?}", attr.edge_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    let src_order = attr.source_out_order;
                    let target_order = attr.target_in_order;
                    format!("label = \"{dbg_attr_replaced},src:{src_order},dst:{target_order}\"")
                },
                &|g, (node, _)| {
                    let node_attr = self.node_attr_map.get(&node).unwrap();
                    let dbg_attr_format = format!("{:?}", node_attr.node_attr);
                    let dbg_attr_replaced = dbg_attr_format.escape_debug();
                    format!("label = \"{node}|{dbg_attr_replaced}\"")
                }
            )
        )
    }

    /// Returns a mapping from pattern node keys to graph node keys.
    ///
    /// Order of children must be the same in the pattern.
    /// Returns `None` if no mapping is found.
    pub fn match_to_pattern<NAP, EAP, NM, EM>(&self, pattern: &ConcreteGraph<NAP, EAP>, nm: &mut NM, em: &mut EM) -> Option<impl Iterator<Item = HashMap<NodeKey, NodeKey>> + '_>
    where
        NM: FnMut(&NodeKey, &NodeKey) -> bool,
        EM: FnMut(&EdgeAttribute<EAP>, &EdgeAttribute<EdgeAttr>) -> bool,
    {
        // let mut nm = |_: &_, _: &_| true;
        // let mut em = |_: &_, _: &_| true;

        let pattern_graph = &pattern.graph;
        let graph = &self.graph;
        let mut isomorphisms = subgraph_isomorphisms_iter(&pattern_graph, &graph, nm, em)?;
        let pattern_nodes = pattern.graph.nodes().collect::<Vec<_>>();
        let self_nodes = self.graph.nodes().collect::<Vec<_>>();

        fn mapping_from_vec(
            big_nodes: &[NodeKey],
            query_nodes: &[NodeKey],
            index_mapping: &[usize],
        ) -> HashMap<NodeKey, NodeKey> {
            let mut mapping = HashMap::new();
            for (src, target) in index_mapping.into_iter().copied().enumerate() {
                let src_node = query_nodes[src];
                let target_node = big_nodes[target];
                mapping.insert(src_node, target_node);
            }
            mapping
        }
        // TODO: Can we avoid materializing all potential mappings? Would need to forward the iterator, but for that we need to be able to attach some local variables to it to avoid borrowchecker errors.
        let mappings_with_order = isomorphisms.filter_map(move |isomorphism| {
            let mapped = mapping_from_vec(
                &self_nodes,
                &pattern_nodes,
                isomorphism.as_ref(),
            );
            // return none if child order or parent order does not match up
            for pat_node in &pattern_nodes {
                let self_node = mapped[pat_node];
                let pat_children = pattern.neighbors_out_ordered(*pat_node);
                // do an ordered compare
                let mut last_order_key = None;
                for pat_child in &pat_children {
                    let self_child_key = mapped[pat_child];
                    let self_child_order = self
                        .graph
                        .edge_weight(self_node, self_child_key)
                        .unwrap()
                        .source_out_order;
                    if let Some(last_order) = last_order_key {
                        if self_child_order < last_order {
                            return None;
                        }
                    }
                    last_order_key = Some(self_child_order);
                }

                // TODO: Add option to ignore parent order?

                let pat_parents = pattern.neighbors_in_ordered(*pat_node);
                // do an ordered compare
                let mut last_order_key = None;
                for pat_parent in &pat_parents {
                    let self_parent_key = mapped[pat_parent];
                    let self_parent_order = self
                        .graph
                        .edge_weight(self_parent_key, self_node)
                        .unwrap()
                        .target_in_order;
                    if let Some(last_order) = last_order_key {
                        if self_parent_order < last_order {
                            return None;
                        }
                    }
                    last_order_key = Some(self_parent_order);
                }
            }

            Some(mapped)
        });
        let all_mappings = mappings_with_order.collect::<Vec<_>>();
        if all_mappings.is_empty() {
            return None;
        }
        Some(all_mappings.into_iter())
    }
}


pub struct DotCollector {
    dot: String,
}

impl DotCollector {
    pub fn new() -> Self {
        DotCollector {
            dot: String::new(),
        }
    }

    pub fn collect<NA: Debug, EA: Debug>(&mut self, graph: &ConcreteGraph<NA, EA>) {
        if !self.dot.is_empty() {
            self.dot.push_str("\n---\n");
        }
        self.dot.push_str(&graph.dot());
    }

    pub fn finalize(&self) -> String {
        self.dot.clone()
    }
}

#[cfg(test)]
mod tests {
    use petgraph::algo::subgraph_isomorphisms_iter;
    use super::*;

    // #[test]
    fn child_order() {
        let mut collector = DotCollector::new();
        let mut graph = ConcreteGraph::<&str, ()>::new();


        macro_rules! c {
            () => {
                collector.collect(&graph);
            };
        }

        c!();
        let a = graph.add_node("hello");
        c!();
        let b = graph.add_node("world");
        c!();

        graph.add_edge(a, b, ());
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, b));

        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, b));

        let c = graph.add_node("foo");
        c!();
        graph.add_edge(a, c, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a,c));
        assert_eq!(prev_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, c));

        let d = graph.add_node("bar");
        c!();
        graph.add_edge(a, d, ());
        c!();
        let next_edge = graph.next_outgoing_edge(a, (a,c));
        assert_eq!(next_edge, (a, d));
        let next_edge = graph.next_outgoing_edge(a, (a,d));
        assert_eq!(next_edge, (a, b));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, d));

        let before_b = graph.add_node("before_b");
        c!();
        graph.add_edge_ordered(a, before_b, (), EdgeInsertionOrder::Prepend, EdgeInsertionOrder::Append);
        c!();

        let after_d = graph.add_node("after_d");
        c!();
        graph.add_edge_ordered(a, after_d, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        c!();

        let next_edge = graph.next_outgoing_edge(a, (a,b));
        assert_eq!(next_edge, (a, c));
        let prev_edge = graph.prev_outgoing_edge(a, (a,b));
        assert_eq!(prev_edge, (a, before_b));
        let next_edge = graph.next_outgoing_edge(a, (a,d));
        assert_eq!(next_edge, (a, after_d));
        let prev_edge = graph.prev_outgoing_edge(a, (a,before_b));
        assert_eq!(prev_edge, (a, after_d));



        let dot = collector.finalize();
        println!("{}", dot);
        assert!(false);
    }

    #[test]
    fn subgraph_isomorphism_test() {
        let mut big_graph = ConcreteGraph::<&str, ()>::new();
        let a = big_graph.add_node("A");
        let b = big_graph.add_node("B");
        let c = big_graph.add_node("C");
        big_graph.remove_node(c);
        let d = big_graph.add_node("D");
        let c = big_graph.add_node("C");
        println!("c: {}", c);
        big_graph.add_edge(a, b, ());
        big_graph.add_edge(b, c, ());
        big_graph.add_edge(c, a, ());


        let mut query_graph = ConcreteGraph::<&str, ()>::new();
        let x = query_graph.add_node("X");
        let y = query_graph.add_node("Y");
        query_graph.add_edge(x, y, ());

        let query = &query_graph.graph;
        let big = &big_graph.graph;
        let mut nm = |_: &_, _: &_| true;
        let mut em = |_:&_, _:&_| true;
        let isomorphisms = subgraph_isomorphisms_iter(&query, &big, &mut nm, &mut em);
        
        let big_nodes = big_graph.graph.nodes().collect::<Vec<_>>();
        let query_nodes = query_graph.graph.nodes().collect::<Vec<_>>();
        
        fn mapping_from_vec(
            big_nodes: &[NodeKey],
            query_nodes: &[NodeKey],
            index_mapping: &[usize],
        ) -> HashMap<NodeKey, NodeKey> {
            let mut mapping = HashMap::new();
            for (src, target) in index_mapping.into_iter().copied().enumerate() {
                let src_node = query_nodes[src];
                let target_node = big_nodes[target];
                mapping.insert(src_node, target_node);
            }
            mapping
        }
        
        for isomorphism in isomorphisms.unwrap() {
            println!("Isomorphism raw: {:?}", isomorphism);
            let mapped = mapping_from_vec(
                &big_nodes,
                &query_nodes,
                isomorphism.as_ref(),
            );
            println!("Isomorphism mapped: {:?}", mapped);
            let attr_map_list = mapped.into_iter().map(|(src, target)| {
                let src_attr = query_graph.get_node_attr(src).unwrap();
                let target_attr = big_graph.get_node_attr(target).unwrap();
                (src_attr, target_attr)
            }).collect::<Vec<_>>();
            println!("Isomorphism attr map: {:?}", attr_map_list);
        }


        let mut big_graph = ConcreteGraph::<&str, ()>::new();
        let a = big_graph.add_node("A");
        let b = big_graph.add_node("B");
        let c = big_graph.add_node("C");
        let d = big_graph.add_node("D");

        big_graph.add_edge_ordered(a, b, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        big_graph.add_edge_ordered(a, c, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        big_graph.add_edge_ordered(a, d, (), EdgeInsertionOrder::Prepend, EdgeInsertionOrder::Append);

        big_graph.add_edge_ordered(d, c, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);

        // a has ordered children d,b,c

        let mut query_graph = ConcreteGraph::<&str, ()>::new();
        let x = query_graph.add_node("X");
        let y = query_graph.add_node("Y");
        let z = query_graph.add_node("Z");
        query_graph.add_edge_ordered(x, y, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        query_graph.add_edge_ordered(x, z, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);
        query_graph.add_edge_ordered(y, z, (), EdgeInsertionOrder::Append, EdgeInsertionOrder::Append);

        let big = &big_graph.graph;
        let query = &query_graph.graph;

        let isomorphisms = subgraph_isomorphisms_iter(&query, &big, &mut nm, &mut em);

        let big_nodes = big_graph.graph.nodes().collect::<Vec<_>>();
        let query_nodes = query_graph.graph.nodes().collect::<Vec<_>>();
        println!("----");
        for isomorphism in isomorphisms.unwrap() {
            println!("Isomorphism raw: {:?}", isomorphism);
            let mapped = mapping_from_vec(
                &big_nodes,
                &query_nodes,
                isomorphism.as_ref(),
            );
            println!("Isomorphism mapped: {:?}", mapped);
            let mut attr_map_list = mapped.into_iter().map(|(src, target)| {
                let src_attr = query_graph.get_node_attr(src).unwrap();
                let target_attr = big_graph.get_node_attr(target).unwrap();
                (src_attr, target_attr)
            }).collect::<Vec<_>>();
            attr_map_list.sort_by(|(src1, _), (src2, _)| src1.cmp(src2));
            println!("Isomorphism attr map: {:?}", attr_map_list);
        }

        println!("----");

        let mappings = big_graph.match_to_pattern(&query_graph, &mut nm, &mut em);
        for mapping in mappings.unwrap() {
            let mut attr_map_list = mapping.iter().map(|(src, target)| {
                let src_attr = query_graph.get_node_attr(*src).unwrap();
                let target_attr = big_graph.get_node_attr(*target).unwrap();
                (src_attr, target_attr)
            }).collect::<Vec<_>>();
            attr_map_list.sort_by(|(src1, _), (src2, _)| src1.cmp(src2));
            println!("Isomorphism attr map: {:?}", attr_map_list);
        }
        
        assert!(false);
    }
}
