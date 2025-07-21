import {Graphviz} from "@hpcc-js/wasm";
import {Context} from "online-syntax-js"
import * as monaco from 'monaco-editor';
import { AnsiUp } from 'ansi_up'
const ansi_up = new AnsiUp();

const graphviz = await Graphviz.load()

Context.init()

let current_res;


let output_pre = document.getElementById('output');
let input = document.getElementById('state_name');
let input_button = document.getElementById('state_name_button');
let svg_container = document.getElementById('svg-container');

input_button.onclick = (e) => {
    let dot = current_res.dotOfState(input.value)
    let svg = graphviz.dot(dot);
    svg_container.innerHTML = svg;
}

let current_content = localStorage.getItem('last_code') || "fn hello() {}";

let editor = monaco.editor.create(document.getElementById('container'), {
    // load from local storage
    value: current_content,
    language: 'rust'
});

const content_changed = () => {
    // store to local storage
    localStorage.setItem('last_code', current_content);

    try {
        let res = Context.parse(current_content);
        current_res = res;
        output_pre.innerText = "Code parsed successfully!";
    } catch (e) {
        output_pre.innerHTML = ansi_up.ansi_to_html(e.cause.toString());
        // output_pre.innerHTML = e.cause.toString();
    }
}

// first load
content_changed();

editor.onDidChangeModelContent((_change_event) => {
    let reconstructed = editor.getModel().getLinesContent().join('\n');
    current_content = reconstructed;
    content_changed();
})





Context.parse("fn hello() {}")