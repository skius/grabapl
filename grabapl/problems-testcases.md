# Problems and Test Cases to be solved

## User defined operation abstract graph shape changes
**Core problem**: How can a user defined operation change the abstract graph when applied?

The _least_ (defined as 'this always happens') change must be entirely caused by new nodes/edges.
Actually, we can support more, but it's difficult.

For example:
1. Operation `add_child_if_not_exists` with input node `a`
2. Shape query that checks if `a` has a child `child`
3. False branch of that query that adds a new node `child` to `a`

One might say that the least change is that `a` now has a child `child`.
This is true only for the builder - we can call operations on that child.
However, the abstract graph is not changed by this operation.
Imagine if that was not the case:


1. Outside of that op, we have an abstract graph `a -> child`.
2. When running `add_child_if_not_exists(a)`, what should the result be?

Realistically, we don't want the abstract graph to change. We want it to somehow know that the
new node `child` inside the operation's abstract graph is the same as the `child` in the argument
graph.

Similarly, if the argument did not have `child`, we would expect the abstract graph to change to `a -> child` for some
new node `child`.

ACTUALLY! Since we define shape queries to run on the _concrete_ graph, the child from the outer abstract graph
_might not even be the same child that is matched inside_.

So, the only way we could support changing the abstract graph for a shape query node, is if we had aliasing
nodes. Then we could unconditionally add a new node `child` that may or may not alias the other child node.
But this is very tricky and we don't want to do that.

Hence: Abstract graph changes due to shape queries are not supported.

This would work, however:
1. Operation `add_child` with input node `a`
2. non-shape-query on eg `a` (eg Eq0)
3. True branch: add child `child` to `a`
4. False branch: add child `child` to `a`
5. After query: reconciled abstract graph is `a -> child`, both for the operation builder and also for any
   calling code.

### Additive changes in the shape query true branch
Inside the shape query true branch, we may want to make some _additive_ changes to
the abstract graph that can be returned to the caller.

For example, if we _add_ a new node in the true branch and also have a new node in the false branch,
those can be reconciled.

However, edges cannot be reconciled if one of their endpoints is a shape-matched node.

What happens in the following case?

```
// state: a
if a (-> b)? {
  add node c;
  add node d;
  add edge b -> c;
  add edge c -> d;
  // a -> b -> c -> d, with "-> b" being a shape match
} else {
  add node b, c, d;
  add edge a -> b;
  add edge b -> c;
  add edge c -> d;
  // a -> b -> c -> d, with b,c,d new
}
// what is the reconciled state here? (**that can be returned to the caller**, not just the state visible in the op builder)
// Options:
// 1. a (we ignore everything attached to b since that was a shape match)
// 2. a, c -> d (we ignore all edges from b, but include the new nodes)
```

Option 1 is the easiest and clearly the most sound.

Option 2 carries the question if there is any aliasing risk now?
Basically, the two connected components a and c->d may or may not be connected in the concrete.
So, if we were doing any "connectedness-analysis", we'd have to say they were "potentially connected".

If we don't do a connectedness-analysis, is it a problem to have both graphs?

**TODO**: revisit after revisiting the connectedness semantics design 


### Output node names
We could make it so the user defines the names of abstract output nodes (and which ones are actually returned)
by a "post-processing" step at the end of the op builder, where the user is asked
for every automatically determined abstract output node
(a UI could eg have a checkbox) whether it should be returned, and if so,
what name it should have.

## Recursion in operation builder
*More information in MT Report Notes gdoc*

**Core problem**: How can we support recursion in the operation builder? At some point during the builder interpretation step, we need to be able to call the operation that's being 
  built recursively.
* Positive: The built operation cannot abstractly _always_ eg add a new node, since that would be an infinite loop.
  Hence we know for sure that the operation will not abstractly add a new node.
  Removing a node should also not be possible in every branch simultaneously.
* But: How do we know the new _abstract values_ to set in the abstract graph after recursing?

For user defined operations, we could make the user say which (sound) changes they want to be visible.
Because we know that the operation will abstractly not add a new node, the abstract graph will be the same
as the parameter graph.

Hence, we can make the user say which abstract node and edge values will change to what values.

Caveats:
1. This must be sound. That is, the user can only change the abstract value to something that is a *supertype* of the
   abstract value at the end of the operation. We will need to check this.
2. This needs to happen _at the beginning_ of the operation, because if we're partially building an operation, and then
   recurse, we must already know the abstract values that will change.
   * This is bad user experience, but let's try it for now.


Also, we need to tell the user why something doesn't work.

[//]: # (TODO: make this better user experience.)

### Examples
See meeting notes gdoc.



## Merging query branch abstract states after the query - Edge Order
*Assuming we make edge order a thing*, we must consider the specific edge order of a common
node in two branches of a query.

For example, if we have a shape query that checks if node `a` has a child `b` with edge `"child"`,
and it checks if that edge is the **first** edge, and the false branch creates a child with edge `"child"`,
then the merged abstract graph should know that `a` does in fact have a child `b` with edge `"child"`,
**however**, the specific edge order must be the *least common ancestor* in the edge order lattice of
"the last child" (since we just added the edge - actually, what is the edge order of a newly added edge?) and
the "first child".

That means in the resulting abstract graph, the edge order should be some Top element probably, i.e.,
`Any`, probably.

## Edge Orders

### Representation
*More information: in MT meeting notes gdoc as well as some types here in the Rust project*

We should probably have both concrete edges and abstract edges.

Abstract edges should be elements of some lattice.

But! What should the abstract edge order of an "add edge" operation be?

At that point we know it's the "last" edge, but as soon as we add another edge, it's not the last
anymore. So how can we represent the fact that it's only last for a given time period?

Perhaps there should be some "last_invisible" marker that refers to the last edge that the
abstract graph does not see, and then the new edge would be "last_invisible + 1", another edge after that
"last_invisible + 2", etc.

### Customizability
What if we supported custom edge orders? `Semantics` would additionally need to provide 
types for the concrete and abstract edge orders.

How would this be stored in the graph? We could provide the user two possibilities:
1. Storing additional data on the node
2. Storing additional data on an edge

(not mutually exclusive). 

And this would be different data for concrete and abstract.

For matching an abstract parameter graph to an abstract argument graph, we would then ask the user for
every node pair (param, arg), together with an iterator over all edges from param and arg,
whether arg can be given to param.

Because both are potentially abstract, matching needs to happen (like for node/edge values) on
abstract values. So, e.g., the arg might say "this is edge `binding`+1",
and the param might say "this is the last edge", and then the matching algorithm would need
to determine if this matches. 
Personally, in this example I think it should not necessarly match, since we have no guarantee
that binding+1 is the last edge (assuming the semantics say that edge orders are based on the concrete).

Because this affects the actual graph, the Graph API needs to actually support
receiving concrete/abstract edge orders on edge adding.

This is easy for orders stored in edges, since when we add an edge we can give it the static edge order data,
but for orders stored in nodes, this requires some modification to data in both node endpoints.

Maybe turn this into some sort of builder?

```rust
// assume `graph` is a concrete graph
let mut edge_builder = graph.add_edge(a, b);
edge_builder.set_edge_order(ConcreteEdgeOrderStoredInEdge::...); 
let a_data: &mut ConcreteEdgeOrderStoredInNode = edge_builder.get_node_a_data();
// modify a_data, which contains other edges as well
let b_data: &mut ConcreteEdgeOrderStoredInNode = edge_builder.get_node_b_data();
// modify b_data
// commit can be called at any time - the edge orders need to implement `Default`
// Drop also commits.
edge_builder.commit(); 
```

## Prettier Operation Builder
The current approach mirrors parsing a stream of tokens via a recursive descent parser with multiple passes through
different structs.

This is quite complex, not very efficient, and not easy to modify.

What is keeping us from using a state machine with an explicit stack?

Essentially, all local variables from the current recursive descent methods need to be stored in the stack.
Since they appear at different points of parsing, they need to be made optional in the explicit stack since we may have not
reached the relevant point yet.

==> There is a lot of overhead.

**Alternative**: Perhaps it is enough to just make the existing operation builder a bit prettier.

It uses a lot of tuples, introducing structs could make it more readable.

Really, if we could just have a Debug constraint on the BuiltinOperation and BuiltinQuery types, the actual IntermediateState
that we can print would potentially be more helpful.

## Node formatting
Maybe it would be helpful to have a "where this comes from" string for abstract nodes.

This would be an additional mapping from AID -> String, and it would be computed by every operation.
For example, the "add(a,b,c)" operation would have have a format function that takes
debug representation for all arguments (in order) and returns a string, e.g., `{i1} <- {i2} + {i3}`.

## Changing abstract values in shape query nodes
**Core problem**: Because we match shape query nodes against the *concrete*, changes to their abstract values cannot
really be reasoned about abstractly.

Imagine this:
1. Helper operation with input node `p0`
2. Shape query that checks if `p0` has a child `c:Object`
2a. True branch: set `c` to `String`
2b. False branch: add a new node `c` with type `Integer` and make it a child of `p0`
3. After the query, the abstract graph is `p0 -> c`, where `c` is a Object.

Now this helper operation is called somewhere:
1. input graph `a -> child:Integer`
2. call helper with `a` as `p0`
3. What abstract value should `child` have?

The problem is that depending on which node is selected concretely, `child` may or may not be a `String`, since
it may be the child that is found by the shape query in the helper.

The problem is exacerbated in case the helper is hidden behind another helper, i.e., the caller does not even directly
see a shape query and would instead have to traverse the entire call stack to find out.

**Potential fixes**:
1. Shape queries are not allowed to change the abstract values of their matched nodes.
   - This is the simplest solution, but it is also quite limiting.
2. Shape queries match invariantly, i.e., if some abstract value `t_s` is expected, then concrete values only
   match that query if their type is exactly `t_s`.
   - Subsequent modifications would be allowed to change the abstract value to something more precise, i.e. a subtype of `t_s`,
    but not to something more general.
3. Specify potential changes to shape nodes and what they look like, and then over-approximatingly change
   **all** abstract values in the entire call stack upwards.
   - I.e., if some operation has a shape query that does something to that node, that must be stored in its signature,
   and if some operation calls that operation, that signature must also keep track of the shape node change, etc.
   - Very overkill.

In all cases, special care has to be taken when passing the matched node to an inner operation.
In particular, the inner operation must not change the abstract value of the matched node.
==> We must have some static guarantees that an operation does not change the abstract value of a parameter node.

**Next problem**: How to delete a shape query node? It almost seems like deleting a shape query node is not possible
unless we use potential fix #3 from above.

[//]: # (TODO: add tests for these problems)

And deleting a node from a shape query probably something we want to support?

Yeah: imagine removing a node from a binary search tree.

How can we do that if not by deleting a shape query node?

[//]: # (TODO: Visualize this in excalidraw.)

**Side note**: This problem appears with edges as well. Imagine a shape query edge being removed - that's a critical
issue as well.

### Root cause of the problem
The root cause is that we may have two handles to the same node with at least one being mutable. I.e., we violate aliasing XOR mutability.
For example, in the "remove child if exists" example, the problem appears if some outer operation abstract graph
has a handle to the child node, and then calls the chain of operations that eventually shapes query for the child and then
removes it.

In Rust this is roughly equivalent to:
```rust
let p = Child { child: 42 };
let child_ref = &p.child;
delete_child_if_exists(&mut p); //internally acquires a mutable reference to `p.child` and deletes it
// child_ref is now dangling.
```

Which is disallowed by the Rust borrow checker.

This is very difficult to fix without a borrow checker.

Actually, Rust provides one workaround: `RefCell`.

What if we decided this was the same in grabapl? We keep track of concretely deleted nodes, pass that to callers,
and after each call the caller checks if something was deleted that we expected to still be around. If so, we crash.

This would require knowledge of what we expect to still be around (i.e., the abstract graph) at concrete execution time, but that's fine.

### Pass-by-reference vs pass-by-value
Fundamentally the above is a problem because we pass arguments by reference, i.e., changes inside the operation are visible
by the caller.

However, something like a `insert_bst_node` *needs* pass-by-reference, because otherwise how would it modify the BST?
(ignoring the fact we could be pure/functional and return a new BST).

What if nodes could be selected to be pass-by-value/pass-by-reference?
We could locally run shape queries on pass-by-value parameters, since there won't be any side-effects outside of the operation.
But this is limiting. I want to have a helper operation that takes some node by reference and then conditionally removes
a shape queried node.

What if we had proper "nesting-graphs" support? E.g., we may have a type that is an entire BST.
1. Accept a BST stored in a node as pass-by-reference parameter.
2. Call some "unpacking" operation on that BST node to get a handle on eg the actual root node of the BST.
3. Hmm. Would still like to be allowed to call helper operations on that root node. But how to uphold aliasing XOR mutability?

## Thoughts about abstract graph changes in terms of function signatures
Any static changes must be inside some signature.
* Want to write to a node? The fact that you're setting its abstract value must be specified.
* Add, delete a node? Specify it.
* Add, delete, or modify an edge? Specify it.