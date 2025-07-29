#import "@preview/diagraph:0.3.5": render
// diagraph does not support record shapes :(
  // => use html tables

#import "@preview/codly:1.3.0": *
#import "@preview/codly-languages:0.1.1": *
// #show: codly-init.with()

#let grabapl-init(body) = {
    codly(languages: codly-languages)
    codly(
      languages: (
        grabapl: (name: "Grabapl", color: rgb("#0E412B"))
      )
    )
    codly(
      zebra-fill: rgb("#f1f1f1"),
      reference-sep: ":"
    )
    codly-init(body)
}

#show: grabapl-init

#let grabapl_plugin = plugin("typst_plugin.wasm")
#let type_color = "brown3"
#let edge_str_color = "forestgreen"
#let dot_of_state(a, state_name) = str(grabapl_plugin.dot_of_state(bytes(a), bytes(state_name), bytes(type_color), bytes(edge_str_color)))

#let render_dot_diagraph(dot) = {
  render(dot)
}

#let render_dot(dot) = {
  render_dot_diagraph(dot)
}

// in the input, find the lines and start indices of all occurrences of `show_state()`
#let find-show-state-lines-better(input) = {
  // Split the input into lines
  let lines = input.split("\n")
  // Enumerate lines with 1-based index and filter those containing "show_state()"
  let matching = lines.enumerate().filter(
    ((_, line)) => line.contains("show_state()")
  )
  // then search for the start index for each line
  let highlights_indices = matching.map(
    ((i, line)) => {
      let start_index = line.position("show_state()")
      if start_index == none {
        return none
      } else {
        return (i + 1, start_index) // 1-based line number and start index
      }
    }
  )
  // Filter out any `none` values
  highlights_indices.filter((x) => x != none).map(
    (x) => {
      let (line_num, start_index) = x
      return (line: line_num, start: start_index + 1) // return as a tuple with line number and start index
    }
  )
}

#let find-show-state-lines(input) = {
  // Split the input into lines
  let lines = input.split("\n")
  // Enumerate lines with 1-based index and filter those containing "show_state"
  let matching = lines.enumerate().filter(
    ((_, line)) => line.contains("show_state")
  )
  // Return the list of matching line numbers (1-based)
  matching.map(((i, _)) => i + 1)
}

#let side_by_side(block, ..image) = {
  grid(
    columns: (auto, auto),
    grid.cell(colspan: 2, block),
    row-gutter: 10pt,
    column-gutter: 20pt,
    align: (horizon + center),
    ..image,
  )
}

#let grabapl(raw_content, code-label: none, code-caption: none, image-label: none, image-caption: none) = {
  let raw_src = raw_content.text

  // replace in src every show_state() with a show_state(state1), show_state(state2), etc.
  let highlights_indices = find-show-state-lines-better(raw_src)

  let show_state_count = highlights_indices.len()
  let src = raw_src
  let states = ()
  for i in range(show_state_count) {
    src = src.replace("show_state()", "show_state(state" + str(i) + ")", count: 1)
    states = states + (("state" + str(i), highlights_indices.at(i).line),)
  }

  // [#highlights_indices]


  codly(
    highlights: (
      highlights_indices.map(
        (elt) => (line: elt.line, start: elt.start, fill: green)
      )
    ),
    radius: 0.5em,
    stroke: 1pt + black,
  )
  let code_block = raw(
    raw_src,
    lang: "grabapl",
    syntaxes: "Grabapl.sublime-syntax",
    block: true
  )
  // turn code block into figure with a link and caption
  let code-label = if code-label != none {
    code-label
  } else {
    src
  }
  let code-label-instruction = if code-label != none {
    label(code-label)
  } else {
    none
  }
  let code-caption = if code-caption != none {
    code-caption
  } else {
    "Grabapl code block"
  }
  let code_block = [#figure(
    code_block,
    caption: code-caption,
  )  #code-label-instruction]

  // for every state, get the dot and add a new image
  let images = ()
  for (state_name, line_number) in states {
    let dot = dot_of_state(src, state_name)
    let graph_image = render_dot(dot)
    let graph_label_instruction = label(code-label + "_" + state_name)
    let image-caption = if image-caption != none {
      image-caption
    } else {
      [Visualized abstract state at #ref(label(code-label + ":" + str(line_number)))]
    }

    let graph_image = block(
        pad(graph_image, left: 1em, right: 1em, top: 1em, bottom: 1em),
        stroke: 0.5pt + rgb("#dddddd"),
        radius: 1em,
    );

    let graph_image = [#figure(
      graph_image,
      caption: image-caption,
    ) #graph_label_instruction]
    images = images + (graph_image,)
  }


  side_by_side(code_block, ..images)
}

