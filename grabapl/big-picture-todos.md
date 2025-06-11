# TODOs
- [ ] "Callable" function. Takes an operation and an abstract graph and determines if the operation is callable. If not, it should state why not.
  - This works inside op builder, since we have an abstract graph there, but also concrete graphs: we just raise them to their abstract graphs first.
- [ ] Better indices/IDs (string literals) for operations in particular SubstMarker etc