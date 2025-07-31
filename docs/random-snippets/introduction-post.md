I am happy to introduce the language (and -framework) I have been working on as part of my master's thesis!

Note: ^(I am posting this here to start a discussion; I don't expect anyone to use it)

Links:
* Repository: [https://github.com/skius/grabapl](https://github.com/skius/grabapl)
    * Contains more visuals and details
* Online playground: [https://skius.github.io/grabapl/playground/](https://skius.github.io/grabapl/playground/)
* Example in-place bubble sort program: [https://github.com/skius/grabapl/blob/main/example_clients/online_syntax/example_programs/tracing_normal_bubble_sort_variant_b.gbpl](https://github.com/skius/grabapl/blob/main/example_clients/online_syntax/example_programs/tracing_normal_bubble_sort_variant_b.gbpl)

Feel free to try all the examples in this post in the online playground!

**Elevator pitch**:

* **Program state is a graph**
* **Client-definable type system** for node and edge weights
* **Statically typed user-defined operations**: expected nodes and edges are guaranteed to exist at runtime, with their values being of the expected types.
    * No explicit loops: recursion only.
* **First-class node markers**: No more explicit `visited` or `seen` sets!
* **WebAssembly**: Grabapl can be compiled to WebAssembly.
* **Ships with a fully-fledged example online IDE**:
    * [https://skius.github.io/grabapl/playground/](https://skius.github.io/grabapl/playground/)
    * Interactive, visual runtime graph editor to create inputs for the program
    * Visualization of user-defined operations' abstract states
    * Automatic visualization of a runtime execution's trace
    * Text-based user-defined operations:
        * Visualize abstract states with `show_state()`
        * Capture trace snapshots with `trace()`
        * Syntax highlighting
        * Error messages

# Interesting Bits

**Client-definable type system**: The language can be used with an arbitrary "type system" for nodes and edges. Specifically, the (semi-) lattice of the subtyping relation, as well as the actual values and types, can be defined arbitrarily.

No matter the type system chosen, user defined operations should still be type-safe.

For example:

* The playground uses [the type system shown here](https://docs.rs/grabapl_template_semantics/latest/grabapl_template_semantics/), which unordinarily has actual strings as edge types ("child", "parent", anything...).
* Node values could be integers, and types can be integer intervals.
    * I.e., the framework's type checking borders on being a [abstract interpretation](https://en.wikipedia.org/wiki/Abstract_interpretation#Numerical_abstract_domains) engine on arbitrary domains

**Modifiable abstract states**: The abstract state of a user-defined operation captures every node and edge of the runtime graph that is guaranteed to exist at that point, with the nodes' and edges' respective types.

The runtime graph is a single, global graph.
This means that abstract states are always _subgraph windows_ into that single global graph.

For example, below is the state at some point in the `bubble_sort_helper` operation from the [bubble sort example program](https://github.com/skius/grabapl/blob/main/example_clients/online_syntax/example_programs/tracing_normal_bubble_sort_variant_b.gbpl) above.

BUBBLE SORT STATE

This indicates that there are two nodes in scope, connected via an edge. In particular, the nodes are named `curr` and `next` and they store a value of type `int`. The edge between them has type `*`, the top type of that type system, indicating we do not care about the specific value.

These abstract states, as mentioned, _guarantee_ existence of their nodes and edges at runtime. This implies that an operation that removes a node from some abstract state (i.e., a parameter node) needs to communicate to its caller that the passed node will no longer exist after the operation returns.

Because everything is passed by-reference and everything is mutable (due to the single, global runtime graph), we need to be careful regarding variance (think: [Java's Array covariant subtyping unsoundness](https://course.khoury.northeastern.edu/cs5500f14/Notes/ObjectOriented4/covariantArrays.html)).

**Perhaps surprisingly**, the language is covariant in node and edge value parameters (instead of invariant). We make this type-safe by adding _potential writes_ to the signature of an operation.

For example:

```rust
fn outer_outer(x: int) {
  // changes are communicated modularly - the call to outer() only looks at
  // outer's signature to typecheck, it does not recurse into its definition.
  modifies_to_string(x);
  // add_constant<5>(x); // type error
}

fn outer(x: int) {
  show_state(outer_before); // playground visualizes this state
  add_constant<5>(x); // type-checks fine - x is an int
  modifies_to_string(x);
  show_state(outer_after);
  // add_constant<5>(x); // type error: x is 'any' but integer was expected
}

fn modifies_to_string(x: int) {
  let! tmp = add_node<"hello world">();
  copy_value_from_to(tmp, x);
  remove_node(tmp);
}
```

For now, the signature only communicates "potential writes". That is, `modifies_to_string` indicates that _it may write a string_ to the parameter `x`,
not that it always does.
This implies that the final type at the call site in both `outer` and `outer_outer` is the least common supertype of `int` and `string`: `any` in this example.

Changes to edges are communicated similarly.

**Subgraph matching**: The language includes subgraph matching (an NP-complete problem in its general form, oops!) as a primitive.
Operations can indicate that they want to include some additional context graph from the caller's abstract state, which is automatically and implicitly matched at call-sites. It is required, and calls without the necessary context will fail at compile-time.

Example:

```rust
fn foo() {
  let! p = add_node<0>();
  let! c = add_node<1>();
  // copy_child_to_parent(p); // would compile-time error here, since p->c does not exist
  add_edge<"child">(p, c); // "child" is arbitrary
  copy_child_to_parent(p); // succeeds!
  if is_eq<0>(p) {
    diverge<"error: p should be 1">(); //runtime crash if we failed
  }
}


fn copy_child_to_parent(parent: int) [
  // context graph is defined inside []
  child: int, // we ask for a node of type int
  parent -> child: *, // that is connected to the parent via an edge of top type
] {
  copy_value_from_to(child, parent);
}
```

**Dynamic querying for connected components**: So far, the only nodes and edges we had in our abstract states were either created by ourselves, or passed in via the parameter.
This is equivalent to type-level programming in a regular programming language (with the entire abstract graph being the 'type' here), and includes all of its limitations.
For example, an algorithm on a dynamically sized data structure (e.g., a linked list, a tree, an arbitrary graph, ...) could only take as input one specific instance of the data structure by specifying it in its context parameter.

So, there is the notion of _shape queries_. Shape queries are like queries (conditions of if statements), except they allow searching the dynamic graph for a specific subgraph.

Example:

```rust
fn copy_child_to_parent_if_exists_else_100(p: int) {
  if shape [
    // same syntax as context parameter graphs
    c: int,
    p -> c: *,
  ] {
    copy_value_from_to(c, p);
  } else {
    let! tmp = add_node<100>();
    copy_value_from_to(tmp, p);
    remove_node(tmp);
  }
}
```

In the then-branch, we abstractly see the child node and can do whatever we want to it.

This introduces some issues: Since we can potentially delete shape-query-matched nodes and/or write to them, any operations whose abstract state already contain the matched nodes would need to "hear" the change. There are ways to do this, but my approach is to instead **hide** nodes that _already exist_ in the abstract state of any operation in the call stack. That way, we are guaranteed to be able to do whatever we want with the matched node without breaking any abstract states.

This can be made less restrictive too: if we only read from a shape-query-matched node, then it does not matter if outer abstract states have that node in scope already. We just need to make sure we do not allow returning that node, since otherwise an abstract state would see the same node twice, which we do not allow.

**First-class node markers**: with the `mark_node<"marker">(node);` operation and the `skipping ["marker"]` annotation on a shape query (which, as the name implies, skips any nodes that have the marker "marker" from being matched), node markers are supported first-class.

**Automatic Program Trace Visualization**: This is in my opinion a very cool feature that just arose naturally from all other features. Using the `trace()` instruction (see the bubble sort source for an example program utilizing it), a snapshot is taken at runtime of the entire runtime graph with all associated metadata.

This can be visualized into an animated trace of a program. Below is a (shortened) trace of the bubble sort operation, as generated by the web playground. The full trace can be found on the GitHub README.

**Legend**:
* Named, white nodes with blue outline:
    * Nodes that are part of the abstract subgraph of the currently executing operation at the time of the snapshot.
    * The names are as visible in the stack frame of the operation that took the snapshot.
* Orange nodes: Nodes that are bound to some operation in the call stack other than the currently executing operation. These are the nodes hidden from shape-queries.
* Gray nodes: Nodes that are not (yet) part of the abstract subgraph of any operation in the call stack.
* Anything in `{curly braces}`: The node markers that are currently applied to the node.

GIF

**Syntax quirks**: The syntax of the playground is just an example frontend. In general, the language tries to infer as much of an operation's signature as possible, and indeed, the syntax currently does not have support for explicitly indicating that an operation will delete a parameter node or modify its value. This is still automatically inferred by the language, it is just not expressable in text-form (yet).

# Similarities

Throughout development I've been searching for languages with similar features, i.e., any of the following:
* Graph-first
* Statically typed graphs
* Pluggable type systems
* Statically typed fnctions that can change the type of a parameter at the call-site

I've only found a few instances, namely for the functions that change parameter's types: Most similarly, there is flux-rs, refinement typing for Rust, which has "strong" references that can update the call-site refinement using a post-condition style (actually - post conditions in verification languages are pretty similar). Then there is also [Answer Refinement Modification](https://terauchi.w.waseda.jp/papers/popl24-arm.pdf), which seems to generalize the concept of functions that modify the abstract state at the call-site.

Of course on the graph side of things there are query languages like neo4j's Cypher.

I probably missed a whole bunch of languages, so I wanted to ask if there's anything in those categories that springs to mind?