# TODOs
- [ ] "Callable" function. Takes an operation and an abstract graph and determines if the operation is callable. If not, it should state why not.
  - This works inside op builder, since we have an abstract graph there, but also concrete graphs: we just raise them to their abstract graphs first.
- [ ] Better indices/IDs (string literals) for operations in particular SubstMarker etc
- [ ] Query branch merging: we need to find the greatest common subgraph of both query branches.
  - Example: if the query is a shape query for has child, and the other branch adds a child, then the merged
   point after the query should have a child statically. the child is the least-common-ancestor of the lattice
   for the edge weight and the node weight.
- [ ] User-defined queries
- [ ] Nested types?


# Next TODOs
- [ ] Work with abstract graph
- [ ] Think about how to keep mapping from abstract graph in tact for concrete execution
  - When calling an operation, the implicitly matched context nodes are important.
  - We need the determined mapping at typecheck-time.
  - Ah! This mapping will use nodes of the "abstract abstract" graph, (i.e., AbstractNodeId)
  - So we just store the implicitly matched AbstractNodeIds explicitly and pass them when
    running operations. This needs some restructuring of the run_operation, since it expect to need to match
    context nodes dynamically.
  - ==> UserDefinedOperations should store the context nodes explicitly for every call. They
    should be stored in the form of AbstractNodeId.