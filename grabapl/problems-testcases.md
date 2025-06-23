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



## Recursion in operation builder
*More information in MT Report Notes gdoc*

**Core problem**: How can we support recursion in the operation builder? At some point during the builder interpretation step, we need to be able to call the operation that's being 
  built recursively.
* Positive: The built operation cannot abstractly _always_ eg add a new node, since that would be an infinite loop.
  Hence we know for sure that the operation will not abstractly add a new node.
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

[//]: # (TODO: make this better user experience.)

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