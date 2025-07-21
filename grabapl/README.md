# grabapl

Check wasm compilation:
```bash
cargo check --target wasm32-unknown-unknown
```


# Next steps:
- [x] finish interpreter, 
- [x] finish state propagation in queries, 
- [x] finish UserDefinedOp abstract_apply, 
  - depends on below new field for user defined op. I dont think we want to manually walk the entire op, instead just cache the abstract changes in a field.
- [ ] finish recurse call changes, 
- [x] finish getting the intermediate state for a given path so that we can actually return it.
- [x] run some tests with the op builder.. figure out why it's weird and the current thing doesnt really work
- [x] support subtractive changes in abstract_apply. i.e., removing edges and nodes.
- [x] actually run the interpreter on every change to op builder in order to catch errors
- [x] Better show state that uses the abstract node IDs in a pretty printed way.
- [x] finish query branch state merging

- [x] In user defined operations, make sure that only _new_ nodes are returned abstractly. And only those that the user wants to return. Needs some new field on UserDefinedOp.
  * will need to make sure they're contravariant to the actual determined state as by the interpreter. (if a node is supposed to be a String, then the user can only return it as a String or Object, but not as eg a Number)
  
- [x] Start some tests...
- [ ] Just add Debug constraints and simplify a lot of code?
- [x] Fix the 'static lifetime from markers and turn it into an owned String
- [x] Add the CustomName variant for AbstractNodeId?
- [ ] Temporary nodes should be first class!
  - See meeting nodes gdoc.
  - It would be nice if we could infer which node is temporary by checking
    if it is somehow connected to the existing graph, and if not it is temporary, but since we may not see an edge, this would potentially delete nodes connected in the concrete but not the abstract.
  - However! We could perform this check in the concrete, i.e., collect abstractly determined candidates, then
    check if they are connected to either explicit output nodes or parameter nodes in the concrete graph, if not, then they are temporary and can be removed.


# Final TODOs I want to do:
- [ ] Do we still need not being able to return a node from the shape query???
    - We used to need it because we thought that could cause aliasing issues, but now that shape queries cannot match already-statically-existing nodes anyway, maybe we could 
    - allow reutrning nodes again? 
    - We'd have the difference of a returned node would either be a completely newly created node, or a shape-matched node.
    - Is that a problem though? I don't think we have any asserts of concrete_size_before_op + returned_nodes == concrete_size_after. (those would break!)
- [x] online editor monaco?
  - online-syntax
  - [ ] timer for when to save state to avoid lag (1s after typing)
- [x] syntax multiple return nodes seems to be broken?
- [x] show state
  - do a map, eg, show!("name"); will return a result with mapping name => intermediatestate at that point.
  - [ ] First-class show_state? i.e., make it a token or something. Take string as arg.
- [x] Define builder behavior when running a cache-induced build() that errors (see negative_tests comment)
- [x] Recursion actually CAN add nodes! Add a test for this (if cond { return 0 } else { return recurse() })
- [x] Fix web demo (builder now needs an ID in the beginning)
- [ ] build operations from Sverrir's studies
- [ ] Shape hidden: If we exit a scope and some abstract node ID falls out of scope, then that node should not be shape-hidden anymore.
  - [ ] Add a test for this. (eg if shape [child] { .. } .. if shape [child] { .. }  should be entered for both branches if the child from the first query is not raised ('merged') to the outer scope)
- [ ] Node bloat: Scope: the pattern of let! res = .., then filling res with something in true/false is bad, since res will be overwritten
  - Delete unconnected, unreturned nodes from the concrete graph after every operation?
  - in UDOp runner, store which nodes are newly created, and in the end delete all new nodes that are either:
    - Not connected to the parameter graph
    - Not connected to a return node/a return node themselves.
- [ ] Research incremental interpreters/parsers? that show partial type info like local variables?
- [ ] Include telegram messages
- [ ] Forget instruction for shape queries
- [ ] Edge orders
- [ ] "is callable" function to determine which operations of operation context can be called!
- [ ] Temporary nodes
  - [ ] Temp nodes could be marked as temp _at result point_, i.e., after an operation is called
    - its results could be marked as temporary
- [ ] Clean up code
  - [ ] Tutorial including which tools to install for building
- [ ] Finish examples/template
- [ ] Write READMEs and doc comments! Especially on the operation builder and semantics.
- [ ] Serialization for OpBuilder
- [x] Serialization for OpCtx
- [ ] Better errors
- [ ] Scoped AIDs? write tests. What if we rename some outer AID in only one branch of a query?
- [x] bang-call support for operations that return just one value:
   * take the name of the operation and immediately rename the _single_ output node to that name.
   * Crash if the operation returns multiple nodes.
  - [ ] Add tests for bang call!
- [kinda x] In the interval type system, try a function like `foo(x) { if x >= 200 { return 200 } else { return foo(x+1) }`
  - and then make the builder actually compute a fixed point (keep constructing new stages until no more changes of signature)
  - Fixed point can maybe be checked by is_isomorphic for the signature graphs? I.e., use that to implement PartialEq on OperationSignature?
- [x] textual language
- [ ] structs in types example semantics?
- [ ] Lift restriction of not being able to return edges from shape queries. They cannot be aliased, and returning edges from there is actually useful! (eg, add_edge_if_not_exists)
- [ ] SigCtx for op builder - then we don't actually need a full user defined operation, just a signature, and that
   - would make mutual recursion easier.
- [x] syntax: propagate interpreter errors via span to parser to give pretty errors
- [ ] syntax: error when UDOp has same name as builtin.
- [ ] Debug idea: concrete call-graph joined with node mapping visualized. i.e., every call graph node
      - has the name of the operation, as well as the parts of the concrete graph that were passed to it as argument colored. context: light blue, param: dark blue
- [ ] VF2++ for faster isomorphisms? https://lemon.cs.elte.hu/trac/lemon/browser/lemon/lemon/vf2pp.h
- [ ] concrete graph dot parsing using dot_parser crate