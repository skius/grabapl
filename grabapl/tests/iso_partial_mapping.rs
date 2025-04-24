use std::hash::RandomState;
use petgraph::algo::isomorphism::subgraph_isomorphisms_iter_with_partial_mapping;
use petgraph::graphmap::DiGraphMap;

type G = DiGraphMap<u32, (), RandomState>;

#[test]
fn iso_partial_mapping() {
    let mut query = G::new();
    query.add_node(0);
    query.add_node(1);
    query.add_node(2);
    // the "forced mapping" node
    query.add_node(3);

    query.add_edge(3, 0, ());
    query.add_edge(3, 1, ());
    query.add_edge(3, 2, ());

    let mut target = G::new();
    for i in 0..100 {
        target.add_node(i);
    }
    for i in 0..100 {
        for j in 0..100 {
            if i != j {
                target.add_edge(i, j, ());
            }
        }
    }
    
    // We want node '3' in query to map to '50' in target.
    let mut partial_mapping = [(3, 50)];
    
    let mut nm = |_:&_, _:&_| {
        true
    };
    let mut em = |_:&_, _:&_| {
        true
    };
    
    let query_ref = &query;
    let target_ref = &target;
    
    let isos = subgraph_isomorphisms_iter_with_partial_mapping(&query_ref, &target_ref, &mut nm, &mut em, &partial_mapping);
    let mut isos = isos.unwrap();
    let first = isos.next();
    let first = first.unwrap();
    // mapping 3 to 50
    // note this is only true since we know the keys of our graph map are exactly the indices. In a general graph map that is not true, since it depends on insertion order.
    assert_eq!(first[3], 50);
}