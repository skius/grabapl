import {AbstractGraph, ConcreteGraph, DotCollector} from "simple-semantics";
import {Graphviz} from "@hpcc-js/wasm";

const graphviz = await Graphviz.load();

const p = document.querySelector("#main")
const svgContainer = document.querySelector("#svgContainer")
let dotCollector = DotCollector.create()
let concrete = ConcreteGraph.create()

document.querySelector("#btnReset").addEventListener("click", (event) => {
    console.log("Reset clicked!");
    p.innerHTML = "";
    dotCollector = DotCollector.create()
    concrete = ConcreteGraph.create()
})

document.querySelector("#btnDotCollector").addEventListener("click", (event) => {
    console.log("dot collector clicked!");
    p.innerHTML = dotCollector.getDot();
})

document.querySelector("#btn").addEventListener("click", (event) => {

})

document.querySelector("#btnShowCurrent").addEventListener("click", (event) => {
    let dc = DotCollector.create()
    dc.collect(concrete)
    let dot = dc.getDot()
    let svg = graphviz.dot(dot);
    svgContainer.innerHTML = svg;
})

document.querySelector("#btnAddNode").addEventListener("click", (event) => {
    let desiredInt = parseInt(prompt("Enter an integer:"));
    if (isNaN(desiredInt)) {
        alert("Please enter a valid integer.");
        return;
    }
    console.log("Adding node with value: " + desiredInt);
    concrete.addNode(desiredInt);
    dotCollector.collect(concrete)
})


const button_callback = (event) => {
    console.log("Button clicked!");

    let s = ConcreteGraph.create();
    s.addNode(42);
    // s.sayHi();

    dotCollector.collect(s)
    p.innerHTML = dotCollector.getDot()

};

