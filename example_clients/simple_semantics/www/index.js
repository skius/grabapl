import {AbstractGraph, ConcreteGraph, DotCollector} from "simple-semantics";
import {Graphviz} from "@hpcc-js/wasm";

const graphviz = await Graphviz.load();

const p = document.querySelector("#main")
const svgContainer = document.querySelector("#svgContainer")
let dotCollector = DotCollector.create()
let concrete = ConcreteGraph.create()

const showGraph = () => {
    let dc = DotCollector.create()
    dc.collect(concrete)
    let dot = dc.getDot()
    let svg = graphviz.dot(dot);
    svgContainer.innerHTML = svg;
}

const updateDotCollector = () => {
    dotCollector.collect(concrete)
    let dot = dotCollector.getDot()
    p.innerHTML = dot
}

const onChange = () => {
    showGraph()
    updateDotCollector()
}

document.querySelector("#btnReset").addEventListener("click", (event) => {
    console.log("Reset clicked!");
    dotCollector = DotCollector.create()
    concrete = ConcreteGraph.create()
    onChange()
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
    showGraph(dot)
})

document.querySelector("#btnAddNode").addEventListener("click", (event) => {
    let desiredInt = parseInt(prompt("Enter an integer:"));
    if (isNaN(desiredInt)) {
        alert("Please enter a valid integer.");
        return;
    }
    console.log("Adding node with value: " + desiredInt);
    concrete.addNode(desiredInt);
    onChange()
})

document.querySelector("#btnAddEdge").addEventListener("click", (event) => {
    let input = prompt("Enter an edge (start,end,value):");
    if (!input) {
        alert("Please enter a valid edge.");
        return;
    }
    let parts = input.split(",");
    if (parts.length !== 3) {
        alert("Please enter a valid edge.");
        return;
    }
    let start = parseInt(parts[0]);
    let end = parseInt(parts[1]);
    let value = parts[2];
    concrete.addEdge(start, end, value);
    onChange()
})


const button_callback = (event) => {
    console.log("Button clicked!");

    let s = ConcreteGraph.create();
    s.addNode(42);
    // s.sayHi();

    dotCollector.collect(s)
    p.innerHTML = dotCollector.getDot()

};

