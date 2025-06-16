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