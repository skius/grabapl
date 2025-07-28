use error_stack::fmt::ColorMode;
use wasm_minimal_protocol::*;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

initiate_protocol!();

#[wasm_func]
pub fn dot_of_state(src: &[u8], node_av_color: &[u8], edge_str_color: &[u8]) -> Result<Vec<u8>, String> {
    error_stack::Report::set_color_mode(ColorMode::None);

    let node_av_color = String::from_utf8_lossy(node_av_color);
    let edge_str_color = String::from_utf8_lossy(edge_str_color);

    let src_str = String::from_utf8_lossy(src);
    let res = syntax::try_parse_to_op_ctx_and_map::<grabapl_template_semantics::TheSemantics>(src_str.as_ref(), false);
    if let Err(err) = res.op_ctx_and_map {
        return Err(format!("Failed to parse source: {}", err.value));
    }
    let res = res.state_map;
    let fst_state = res.values().next().ok_or_else(|| "No state found in the operation context".to_string())?;
    let dot = fst_state.dot_with_aid_table_based_with_color_names(&node_av_color, &edge_str_color);

    Ok(dot.into_bytes())
}

// too much hassle. the `layout` is just not there yet.
// #[wasm_func]
// pub fn svg_of_dot(dot: &[u8], args: &[u8]) -> Result<Vec<u8>, String> {
//     let dot_str = String::from_utf8_lossy(dot);
//     let mut parser = layout::gv::DotParser::new(&dot_str);
//     let graph = parser.process().map_err(|e| format!("Failed to parse dot: {}", e))?;
//     let mut gb = layout::gv::GraphBuilder::new();
//     gb.visit_graph(&graph);
//     let mut vg = gb.get();
//     let mut svg_writer = layout::backends::svg::SVGWriter::new();
//     let debug_mode = args[0] == b'1';
//     let disable_opt = args[1] == b'1';
//     let disable_layout = args[2] == b'1';
//     vg.do_it(debug_mode, disable_opt, disable_layout, &mut svg_writer);
//
//     Ok(svg_writer.finalize().into_bytes())
// }