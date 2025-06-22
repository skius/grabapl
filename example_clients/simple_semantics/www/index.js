import {AbstractGraph, AbstractNodeId, ConcreteGraph, DotCollector, OpCtx, Runner} from "simple-semantics";
import {Graphviz} from "@hpcc-js/wasm";
import {getOpCtx} from "./op_builder";

const graphviz = await Graphviz.load();

const p = document.querySelector("#main")
const svgContainer = document.querySelector("#svgContainer")
let dotCollector = DotCollector.create()
let concrete = ConcreteGraph.create()


const runner = Runner.create();
let opCtx = getOpCtx();


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

document.querySelector("#btn").addEventListener("click", (event) => {

})

document.querySelector("#btnRunOp").addEventListener("click", (event) => {
    let input = prompt("Enter an operation input (id,inp1,inp2,...):");
    if (!input) {
        alert("Please enter a valid operation input.");
        return;
    }
    let parts = input.split(",");
    if (parts.length < 1) {
        alert("Please enter a valid operation input.");
        return;
    }
    let id = parseInt(parts[0]);
    let inputs = parts.slice(1).map(x => parseInt(x));
    console.log("Adding operation with id: " + id + " and inputs: " + inputs);

    try {
        runner.run(concrete, opCtx, id, inputs)
    } catch (e) {
        console.error("Error running operation:", e);
        alert("Error running operation: " + e.message);
        return;
    }
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

// make the btnInsertNode button use the nodeVal input label
document.querySelector("#btnInsertNode").addEventListener("click", (event) => {
    let nodeVal = document.querySelector("#nodeVal").value;
    if (nodeVal === "") {
        alert("Please enter a value for the node.");
        return;
    }
    console.log("Inserting node with value: " + nodeVal);
    concrete.addNode(nodeVal);
    onChange()
});

// make btnCallBstInsert use the bstInsertVal and bstRootId inputs to call operation 5
document.querySelector("#btnCallBstInsert").addEventListener("click", (event) => {
    let bstInsertVal = document.querySelector("#bstInsertVal").value;
    let bstRootId = document.querySelector("#bstRootId").value;
    if (bstInsertVal === "" || bstRootId === "") {
        alert("Please enter a value for the node and the root id.");
        return;
    }
    console.log("Calling BST insert with value: " + bstInsertVal + " and root id: " + bstRootId);
    let value_node_key = concrete.addNode(parseInt(bstInsertVal));
    runner.run(concrete, opCtx, 5, [parseInt(bstRootId), value_node_key]);
    onChange()
});




// tab handling
let playground_button = document.querySelector("#playground-tab-button");
let operation_builder_button = document.querySelector("#operation-builder-tab-button");
playground_button.addEventListener("click", (event) => {
    openTab(event, "playground-tab");
});

operation_builder_button.addEventListener("click", (event) => {
    openTab(event, "operation-builder-tab");
});


function openTab(evt, tabId) {
    // Declare all variables
    var i, tabcontent, tablinks;

    // Get all elements with class="tabcontent" and hide them
    tabcontent = document.getElementsByClassName("tabcontent");
    for (i = 0; i < tabcontent.length; i++) {
        tabcontent[i].style.display = "none";
    }

    // // Get all elements with class="tablinks" and remove the class "active"
    // tablinks = document.getElementsByClassName("tablinks");
    // for (i = 0; i < tablinks.length; i++) {
    //     tablinks[i].className = tablinks[i].className.replace(" active", "");
    // }

    // Show the current tab, and add an "active" class to the button that opened the tab
    document.getElementById(tabId).style.display = "block";
    // evt.currentTarget.className += " active";
}


/// Below here: the operation builder tab

