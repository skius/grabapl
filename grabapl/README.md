# grabapl

Check wasm compilation:
```bash
cargo check --target wasm32-unknown-unknown
```


# Next steps:
- [ ] finish interpreter, 
- [ ] finish state propagation in queries, 
- [ ] finish UserDefinedOp abstract_apply, 
  - depends on below new field for user defined op. I dont think we want to manually walk the entire op, instead just cache the abstract changes in a field.
- [ ] finish recurse call changes, 
- [x] finish getting the intermediate state for a given path so that we can actually return it.
- [x] run some tests with the op builder.. figure out why it's weird and the current thing doesnt really work
- [ ] support subtractive changes in abstract_apply. i.e., removing edges and nodes.
- [x] actually run the interpreter on every change to op builder in order to catch errors
- [x] Better show state that uses the abstract node IDs in a pretty printed way.
- [ ] finish query branch state merging

- [ ] In user defined operations, make sure that only _new_ nodes are returned abstractly. And only those that the user wants to return. Needs some new field on UserDefinedOp.
  * will need to make sure they're contravariant to the actual determined state as by the interpreter. (if a node is supposed to be a String, then the user can only return it as a String or Object, but not as eg a Number)
  
- [ ] Start some tests...