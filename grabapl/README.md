# grabapl

<div align="center">

[![Crate Badge]][Crate] [![Docs Badge]][Docs]

</div>

A library for **gra**ph-**ba**sed **p**rogramming **l**anguages with static analysis.

Playground: [https://skius.github.io/grabapl/](https://skius.github.io/grabapl/)

Docs: [https://docs.rs/grabapl/latest/grabapl/](https://docs.rs/grabapl/latest/grabapl/)

## Main Features
* The program state is a single global, directed graph.
* The type system is a shape-based type system (i.e., existence and absence of nodes and edges) composed
  with an arbitrary client-defined type system for node and edge values.
    * Nodes and edges can hold arbitrary values of arbitrary types.
    * See [`grabapl_template_semantics`] for an example client.
* No explicit loops, only recursion.
* Statically visible nodes and edges are guaranteed to exist at runtime. No nulls.
* Frontend-agnostic with a focus on intermediate abstract states:
    * The fundamental building blocks of programs are "instructions" that can stem from any source.
    * For example, a frontend may decide to be visual-first by visualizing intermediate states and
      turning interactive actions into instructions.
    * A text-based frontend is provided with [`grabapl_syntax`],
      supporting a Rust-like syntax with pluggable client-defined parsing rules.
* Builds to WebAssembly
    * See [`grabapl_template_ffi`] for an example FFI library that exposes functionality to JavaScript.

## Example
Using the [`grabapl_syntax`] frontend as example with the example node and edge type system from
[`grabapl_template_semantics`], here is an implementation of in-place bubble sort on a linked list
(feel free to copy-paste this code into the playground and play around with it!):

```rust ,ignore
// Jumpstarts the helper function
fn bubble_sort_wrapper(head: int) {
    let! direction = add_node<0>();
    bubble_sort(head, direction);
    remove_node(direction);
}

// General approach:
// 1. Depending on `direction`, go down the linked list, pulling the maximum/minimum 
//    element with us.
// 2. Once we reach the end/beginning of the *remaining* list, we know that
//    that element is now in its final position (it was either the maximum or 
//    the minimum remaining element)
// 3. Hence we mark that node as 'fixed', and switch direction.
// 4. Once there are no more non-'fixed' nodes, we are done.
//
// In other words, the call stack will look roughly like this, with 
// 'fixed' mark actions indicated by an 'x':
//
// Linked list: a   ->   b   ->   c   ->   d
//              1.
//                       2.
//                                3.
//                                         4.
//                                5.       x
//                       6.                x
//              7.                         x
//              x        8.                x
//              x                  9.      x
//              x        10.       x       x
//              x        x         x       x
//
// 10. can't go down the list anymore, hence the entire list has been sorted.
fn bubble_sort(curr_elt: int, direction: int) {
    // direction == 0: down
    // direction == 1: up
    if is_eq<0>(direction) {
        // we go down the list, pulling the max element with us
        if shape [
            next: int,
            curr_elt -> next: *,
        ] skipping ["fixed"] {
            if fst_lt_snd(curr_elt, next) {
                // already in order
            } else {
                // need to swap values then continue
                swap_values(curr_elt, next);
            }
            // just continue to next, while making sure to forget our node
            hide_node(curr_elt);
            bubble_sort(next, direction);
        } else {
            // we have reached the end. Since we were going down, that means `curr_elt` is now in the right position.
            mark_node<"fixed">(curr_elt);
            // now we need to check if we can go back up again
            add_constant<1>(direction);
            if shape [
                prev: int,
                prev -> curr_elt: *,
            ] skipping ["fixed"] {
                // if so, just recurse on prev
                bubble_sort(prev, direction);
            } else {
                // we're done!
            }
        }
    } else {
        // we go up the list, pulling the min element with us
        if shape [
            prev: int,
            prev -> curr_elt: *,
        ] skipping ["fixed"] {
            if fst_lt_snd(prev, curr_elt) {
                // already in order
            } else {
                // need to swap
                swap_values(prev, curr_elt);
            }
            hide_node(curr_elt);
            bubble_sort(prev, direction);
        } else {
            // we have reached the top. hence we must be the min node and can fix ourselves.
            mark_node<"fixed">(curr_elt);
            // now we need to go back down
            add_constant<-1>(direction);
            if shape [
                next: int,
                curr_elt -> next: *,
            ] skipping ["fixed"] {
                // we can go back down, so let's recurse
                bubble_sort(next, direction);
            } else {
                // we're done!
            }
        }
    }
}

fn swap_values(a: int, b: int) {
    let! temp = add_node<0>();
    copy_value_from_to(a, temp);
    copy_value_from_to(b, a);
    copy_value_from_to(temp, b);
    remove_node(temp);
}

// Hides a node from the abstract state in order to release it for future dynamic shape-query matching.
fn hide_node(node: any) {
    let! one = add_node<1>();
    if is_eq<0>(one) {
        // statically 'maybe' delete the node in order to hide the node.
        // in practice this is never executed.
        remove_node(node);
    }
    remove_node(one);
}
```

[`grabapl_template_semantics`]: https://crates.io/crates/grabapl_template_semantics
[`grabapl_template_ffi`]: https://crates.io/crates/grabapl_template_ffi
[`grabapl_syntax`]: https://crates.io/crates/grabapl_syntax
[Docs]: https://docs.rs/grabapl/latest/grabapl/
[Crate]: https://crates.io/crates/grabapl
[Crate Badge]: https://img.shields.io/crates/v/grabapl?logo=rust&style=flat-square&color=E05D44
[Docs Badge]: https://img.shields.io/badge/docs-grabapl-1370D3?style=flat-square&logo=rust