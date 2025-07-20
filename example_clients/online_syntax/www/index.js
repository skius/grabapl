import {Graphviz} from "@hpcc-js/wasm";
import {Context} from "online-syntax-js"
import * as monaco from 'monaco-editor';
import { AnsiUp } from 'ansi_up'
const ansi_up = new AnsiUp();

Context.init()

let output_pre = document.getElementById('output');

let editor = monaco.editor.create(document.getElementById('container'), {
    value: 'fn main() {}',
    language: 'rust'
});

editor.onDidChangeModelContent((_change_event) => {
    let reconstructed = editor.getModel().getLinesContent().join('\n');
    try {
        let res = Context.parse(reconstructed);
        output_pre.innerText = "Code parsed successfully!";
    } catch (e) {
        output_pre.innerHTML = ansi_up.ansi_to_html(e.cause.toString());
        // output_pre.innerHTML = e.cause.toString();
    }
})



Context.parse("fn hello() {}")