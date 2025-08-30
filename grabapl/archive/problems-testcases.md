# Problems and Test Cases to be solved

## User defined operation abstract graph shape changes
**Core problem**: How can a user defined operation change the abstract graph when applied?

The _least_ (defined as 'this always happens') change must be entirely caused by new nodes/edges.
Actually, we can support more, but it's difficult.

For example:
1. Operation `add_child_if_not_exists` with input node `a`
2. Shape query that checks if `a` has a child `child`
3. False branch of that query that adds a new node `child` to `a`

**UPDATE FROM FUTURE**: We have decided that shape queries can only match on nodes that no other operation has a handle to.
Hence, we can actually return a node from a shape query. In effect, the `add_child_if_not_exists` operation would just be a
`add_child_if_handle_already_exists_otherwise_return_handle` operation. for an input p -> c it would return a new child c2.
for an input p it would return a child c that is either new or already existed in the concrete before.
**UPDATE END**

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

**TODO**: revisit after revisiting the  connectedness semantics design 


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
   * We can make it slightly better: it just needs to happen *right after the first recursion*!


Also, we need to tell the user why something doesn't work.

[//]: # (TODO: make this better user experience.)

### Examples
See meeting notes gdoc and excalidraw.



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
   - It also doesn't work, because we may want to delete the node! Which is in effect a 'change'.
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

Also, we would need a way to "forget" a node/edge, so that we don't crash even if we actually want to delete it.

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

Also, existing Algot is pass-by-reference, so pas-by-value would be difficult to backwards-compatibly implement.

### Purity
By saying that every operation call gets a copy of the graph (connected component should be enough), we can modify abstract values
to our liking.

Insert BST would create a copy and return a node corresponding to the new root, and callers would have to make sure they are now
using the new root node.

This induces requirements on some builtin operations though: We don't want "add node" or "add edge" to be pass-by-value,
obviously, so it needs to be optional I guess?

### Exclusive Access

How difficult would it be to add a notion of "requires exclusive access to node X" to the operation signature?
Would imply exclusive access to the entire connected component of X. Since two non-connected abstract nodes may or may not
be connected, this requires us to implement connectedness analysis.

### Interpreting everything
Since shape queries are ran on the concrete graph, even if we interpret everything abstractly we will still not know
if some inner operation's shape query will change an outer operation's abstract graph
(because we don't know if the shape query's match corresponds to the abstract graph node)

The main problem is that the shape query match is not guaranteed to have some abstract graph equivalent.
If we tried to enforce that, though, edge orders would start to become a problem, since an outer operation changes the behavior
of an inner operation's shape query just by having or not having the corresponding abstract node in its graph.
Avoiding ambiguity would essentially be solving for edge orders and seeing if the outer abstract node has a potentially matching
edge order or not.

### Hiding nodes from shape queries
What if available nodes were automatically hidden from shape queries instead? So if a parent of "delete from BST" has a handle
to the child node, then the call will dynamically and concretely not match the child node from the parent operation.
Again, requires 'forgetting' nodes, if an outer operation wanted to actually delete the child.
The starting global state must have all its nodes forgotten except for the first operation's parameters, otherwise all concrete nodes
are shape-hidden, and no shape queries will match.

**I think I prefer this option the most**

If we were to pick this option, how to implement abstract value changes in this framework?

We could say that shape queries in general do not match hidden nodes. 
But this is limiting, since read-only shape queries are quite useful. Imagine a "list_length" operation not working just
because some outer operation has a handle to one of the list's elements.

Really, by read-only we mean "does not change the abstract value of the matched node". In the concrete, as long as the type remains the same,
it can be mutated just fine (assumption: pass-by-reference semantics).

Actually, we run into the problem of "even if we match an Object, we may not be allowed to set it to Object", since a String node may match
as Object, but if we then set it to Object by setting it to Integer (and then raising it to Object), then we have a problem in the outer operation
that has a handle to the node as "String".

So we do actually need to know whether or not operations set a node's value to *anything*. But we have that information for regular graphs anyway, so that's good.
Then, we have the following options:
1. We allow modification of the abstract value, but this induces a non-shape-hidden constraint on the matched node.
   We do not allow _any_ changes (even same abstract value) to the matched node if we want to match nodes that may have an outside handle.
2. We only match invariantly and allow invariant changes to the abstract value, i.e., a node that exists outside as a String can not be matched as Object.
   * It could be that op A has a handle to the node as String, calls op B which takes an Object, so now the node exists both as String and Object.
   * In this case, the node must only match as the greatest subtype of String and Object, which is String.
   * But then operations that *set* the value to String would be allowed to be called on that node.
   * Actually, we could even allow subtypes? An operation that sets the value to SubtypeOfString would be allowed to be called on that node.

Can we combine the two options?
1. Determine what we are doing with the shape matched nodes (*and edges!*)
   * Deletions: The shape query will not match on shape-hidden nodes (i.e., ones for which previous handles exist)
   * Modifications: Determine the resulting type that the node will have. Call it `ResultingT`.
2. Dynamic match behavior:
   * Neither deletions nor modifications: The shape query can match all nodes and edges.
   * Modifications but no deletions: The shape query matches only nodes for which the existing handle is a supertype of `ResultingT`.
   * Deletions: The shape query matches only nodes which are not shape-hidden, i.e., for which no previous handle exists at all.
   * **Note**: This behavior can actually be per-node/edge, i.e., if we have a shape query for two nodes `a` and `b`,
     If only `a` is modified and `b` is not touched, then `b` can be matched to anything, while `a` can only be matched according to the `ResultingT` rule above.

**TODO**: Would be nice for UX if we returned _why_ a shape query did not match anything if it in fact could have matched something.

**TODO**: Make visual examples for this.

Note: What we're saying here is it's more important that an outer operation does not get its abstract graph dynamically invalidated,
but instead may see some operations not being called if a shape query cannot match due to existing handles.

**Regarding forgetting**: Forgotten nodes need to be kept track of during the builder,
because they are distinctly not *deleted* nodes. In the concrete, forgotten nodes
need to be removed from the hidden_nodes set _of the self operation_ (but not any outer hidden_nodes set).
In other words, it does not make sense to hide parameter nodes, since they are by definition part of the outer operation.
Actually - if we are the topmost operation, then there is no outer set, so we should be able to hide parameter nodes as well.

**Regarding problems with hidden nodes**: A situation where we might want to shape-query a node that is part of an outer operation:
We're building something related to trees. We go down the tree from the root to leaves, but at each node we may want to do something
differently depending on whether it is the root or not.
Sure - it is possible to encode whether or not we're at the root node eg. with a helper operation or via a bool parameter that immediately
gets set to false, but a more intuitive way would be to be able to shape-query check if a parent node exists.
*But this is the problem*: Since when eg recursing on a child, we will have a graph like root->child, in other words the root
node is shape-hidden! and the child would not be able to see the root.

This can be fixed by reducing the restrictions on shape-hidden nodes: if we don't write/delete a shape-queried node, it may
ignore the hidden-nodes set.

TODO: test this

## Thoughts about abstract graph changes in terms of function signatures
Any static changes must be inside some signature.
* Want to write to a node? The fact that you're setting its abstract value must be specified.
* Add, delete a node? Specify it.
* Add, delete, or modify an edge? Specify it.

This is bad for coupling. We have modularity in the sense that functions with the same signature can be used interchangeably, but
the signatures are so detailed that this is unlikely to happen often.

**Signature Subtyping**: To make this slightly more bearable, we should allow signature subtyping.
That is, we need some function that checks whether a signature can be used in place of another signature.

For example, for `s1` functions to be used wherever `s2` functions are expected:
1. `s1` must define _at least_ the same set of output nodes as `s2`.
   * The output nodes must have the same name.
   * The output nodes must have the same edges.
   * The abstract values of `s1`'s output nodes and edges must be subtypes of `s2`'s output nodes.
2. The abstract value changes of `s1` must be subtypes of the abstract value changes of `s2`.
   * If `s2` says "Parameter node `a` will be set to `Object`", then `s1` can say that `a` will be set to `Object` or `String` or `Number`.
   * If `s2` says `String` however, then `s1` cannot say `Object` or `Number`, since that would be a supertype.
   * Actually, quite importantly, it must be that all changes from `s1` are present in `s2`. I.e., `s1` cannot change more things than `s2`.
3. There must be fewer deletions in `s1` than in `s2`.
   * If `s2` deletes node `a`, then `s1` can delete `a` or not delete it.
   * If `s2` deletes edge `b -> c`, then `s1` can delete that edge or not delete it.

It should be possible, in the operation builder, to manually turn the inferred signature into a supertype of the inferred signature.

### Edge changes problems

Parameter: Nodes `a`, `b`.

Operation adds an edge between `a` and `b` with type `String`.

This will be part of the "new edges" set in the signature.

Calling operation calls that operation with `a -Integer-> b`.

All good - caller will notice that a new edge gets created, and hence will know that its existing edge must be updated.

Note that this is only true because we don't allow multiple parallel edges.

## User-defined queries (UDQ) brainstorming

Would love to reuse the existing operation builder for parts of a UDQ.

Can queries modify?
* If yes, queries are just different in that their result can be used to enter two different branches.
* If no, we need to limit the modification capabilities of regular operations.

Can we somehow make the "branching capability" first-class? I.e., can we allow the user to somehow return
such a thing manually?

We could add an associated OutputType to our Semantics trait, and operations can return such a type.
That type must then implement an interface that supports our "start_query" semantics somehow.
So there's no difference between a UDQ and UDF. When a UDF is used in operation context, the outputtype is ignored,
but when it's used in place of a start_query, the outputtype is used to determine the query branches.
Actually, we need to have an option for UDFs to opt-out from being used as queries. Needs to be in the signature.

Initially, the output type should just be isomorphic to `bool` - i.e., we can pick one of two branches.

If a UDF opts-in to being used as a query, how can it actually return a true/false value?

Algot Web does this by having a "global mutable result variable" that the user can set to true/false as the program progresses.

I dislike this, as it doesn't feel as first-class as nodes and edges.
Maybe they don't return a bool, but they return a specific node, of a specific type, where the type supports a "ToBool" trait?
(for ToBool to work in our framework, the NodeConcrete value needs a ToBool -> Option<bool> method, where dynamically non-bool types just return None.)
(also: NodeAbstract needs a IsBool method that returns true on types that dynamically will return Some() for ToBool)
So, for a UDF to opt-in to being a UDQ, it would explicitly mark one of its returned nodes as the query result node. The builder asserts that the supposed type in fact supports ToBool.

What should the semantics of the bool result node be?
* If operation was called in a query context, the result node is deleted. Note: this requires it to be non-connected just to avoid confusingly deleting edges.
* If not, keep the bool node around, so we can do bool operations?


## Operation Builder brainstorming and future work
Concepts that are at odds:
1. It tries to give a representation of the current abstract state of the operation after each instruction.
   * That is useful/necessary for the visual programming paradigm.
   * However, it means there must be some "meaningful" state after each instruction.
2. It tries to ensure after every instruction, it is "more or less" valid.
   * "More or **less**": it's fine if the parameter graph has disconnected context nodes.
   * "**More** or less": it is not fine if the new instruction tries to call an operation for which the argument does not match.

The main problem that arises with this is that the user must take care to add instructions in the right order,
such that every partial state is valid.
To some degree this makes sense, since visually we want to have sensible state to show.
However, it might be a bit restrictive.

If we followed a compiler paradigm more closely, we could have the following process:
1. Build an AST from messages. This should happen more or less syntactically.
2. Type-check/abstractly interpret the AST to get rich information.

The problem with that is that in step 1 we don't really have meaningful visual state. Calling an operation
could modify the abstract state, which we need to show visually, but that is something _semantic_.

(side node: our process is essentially building the AST and validating it on the fly.)

## Match statements and why they're needed
If we wanted full interval type system support via abstract interpretation, queries
would need to be able to modify the abstract type of a node based on which branch we're inside.
(i.e.: `x: [0, 100]; if x == 100 { x: [100,100] } else { x: [0,99] }`)

That specific example would also be possible with match statements, i.e.:
```rust
match x.type {
    [100,100] => { ... }
    [0,99] => { ... }
    _ => { ... } // catch-all
}
```

Where we can decide different options for the catch-all: Either the type system provides a 'completeness' check
that would return true if all possible sub-types/cases are covered for x,
or,
we could have a 'unreachable!()' instruction that, just like in rust, returns some form of bottom type, which
can be coerced into anything.
More concretely, when merging branches, if a branch contains a unreachable!() instruction,
we just ignore it for purposes of merging. we pretend the other branch is the only one that's executed.


## Profiling results
I profiled the max heap remove operation (algot_examples/task2), and the main pain points were:
1. get_shape_query substitution, almost 50%
2. repeated concrete_to_abstract calls, somewhere between 20-30%
   * Fix: cache the abstract graph of the concrete graph. Whenever the concrete graph
   * is changed, modify the corresponding abstract graph.
   * Should be possible with a similar wrapper struct like GraphWithSubstitution.
   * Whenever we write a node/edge, we just write the concrete_to_abstract version of it.

It was a balanced 2000 node max heap graph, with 2000 remove calls.
Guessing, but each call should take around log2(heap size) query calls, so in total
around sum(1, 2000, log2(i)) ~ 19000 method calls.


## Edges maybe-writes of out of scope edges
Because edges are uniquely defined by their endpoints, all edges between parameter nodes, even if not existing in the parameter, must be returned as maybe-writes.
The reason for this is the following:

```rust
fn unsound1(a: int, b: int)[a -> b: "hello"] {
    ruin(a, b);
    show_state(); // still shows hello
}

fn ruin(a: int, b: int)
    // [a->b: string] // if this is added, above correctly shows 'string'. But without this, the connection between the two edges is not made. The write from this function is dropped.
    {
    add_edge<"bye">(a,b);
}
```

A consequence of this is that shape queries, even if read-only, could break _arbitrary edges_ of outer operations.
The problem is that _even a new edge_ causes problems, i.e., it's *not limited to modifications of existing ones*.

The current implementation of shape queries is safe, because no matched node is visible in an outer parameter.

The same goes for deleting edges!
```rust
fn unsound2(a: int, b: int)[a -> b: "hello"] {
    ruin2(a, b);
    show_state();
}

fn ruin2(a: int, b: int)
    [a->b: string]
    {
    remove_edge(a,b);
}
```



















