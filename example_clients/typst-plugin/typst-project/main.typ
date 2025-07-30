#import "./grabapl-package/lib.typ": *
#import "@preview/codly:1.3.0": *
#show: grabapl-init

#codly(
    header: [*Example syntax*],
    header-cell-args: (align: center, )
)
    #codly(
      zebra-fill: rgb("#f6f6f6"),
      reference-sep: ":"
    )
#grabapl(
  code-caption: "Supported syntax features",
  code-label: "first-code",
```rust
// Example function
fn concat_string_siblings(
  // explicit node parameters are inside ()
  parent: any
) [
  // implicitly matched context graph is inside []
  child: string,
  // edges are defined in the context graph
  parent -> child: *,
] -> (concatenated: string) // named return values 
{
  let! result = add_node<"">();
  // shape queries can be used to dynamically match on the shape
  // of the graph at runtime
  if shape [
    // ask for a sibling that stores a string
    sibling: string,
    // ask for the sibling to be a child of the parent
    parent -> sibling: *,
  ] {
    // we see the sibling here!
    show_state();
    // now we can copy the values of child and sibling to result
    append_snd_to_fst(result, child);
    append_snd_to_fst(result, sibling);
  }
  // we don't see the sibling here
  show_state();

  // return values are by-name, so we indicate that the return name 
  // `concatenated` should receive the value of `result`
  return (concatenated: result);
}
```)


