# grabapl

Check wasm compilation:
```bash
cargo check --target wasm32-unknown-unknown
```


# Next steps:
- [ ] finish interpreter, 
- [ ] finish state propagation in queries, 
- [ ] finish UserDefinedOp abstract_apply, 
- [ ] finish recurse call changes, 
- [ ] finish getting the intermediate state for a given path so that we can actually return it.
- [ ] run some tests with the op builder.. figure out why it's weird and the current thing doesnt really work
- [ ] support subtractive changes in abstract_apply. i.e., removing edges and nodes.
- [ ] actually run the interpreter on every change to op builder in order to catch errors
- [ ] Better show state that uses the abstract node IDs in a pretty printed way.
- [ ] finish query branch state merging

- [ ] In user defined operations, make sure that only _new_ nodes are returned abstractly. And only those that the user wants to return. Needs some new field on UserDefinedOp.
  * will need to make sure they're contravariant to the actual determined state as by the interpreter. (if a node is supposed to be a String, then the user can only return it as a String or Object, but not as eg a Number)