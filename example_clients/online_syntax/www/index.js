import {Graphviz} from "@hpcc-js/wasm";
import {ConcreteGraph, Context} from "online-syntax-js";
// import * as monaco from 'monaco-editor';
// import this to only import the features we requested from webpack.config.js:
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
// import 'monaco-editor/esm/vs/basic-languages/rust/rust.contribution';
import { AnsiUp } from 'ansi_up';
import * as d3 from 'd3';
import {d3Graphviz} from 'd3-graphviz'

// --- Initialization ---
const ansi_up = new AnsiUp();
let graphviz;
let current_res; // To store the result of a successful compilation

// --- DOM Element References ---
const outputPre = document.getElementById('output');
const svgContainer = document.getElementById('svg-container');
const stateSelector = document.getElementById('state-selector');

// --- Monaco Editor Setup ---
const initialCode = localStorage.getItem('last_code') ||
    `// Welcome! Type your Grabapl code here.
// You can find example programs here: https://github.com/skius/grabapl/tree/main/example_clients/online_syntax/example_programs
// Maybe try "tracing_normal_bubble_sort_variant_b.gbpl"?

fn foo(x: int) -> (result: int) {
    show_state(initial_state);
    // try returning a node!
    // let! new_node = add_node<1>();
    // show_state(after_adding_node_state);
    // return (result: new_node);
}`;
// TODO: add drop down with example code snippets

const editor = monaco.editor.create(document.getElementById('container'), {
    value: initialCode,
    language: 'rust',
    theme: 'vs-light',
    automaticLayout: true, // Ensures the editor resizes responsively
    minimap: { enabled: false },
    roundedSelection: true,
    scrollBeyondLastLine: true,
});
window.editor = editor; // Expose editor for resizer script

let editorDecorations = editor.createDecorationsCollection();

// --- Theme Toggle Logic ---
const themeToggleBtn = document.getElementById('theme-toggle');
const themeToggleDarkIcon = document.getElementById('theme-toggle-dark-icon');
const themeToggleLightIcon = document.getElementById('theme-toggle-light-icon');
const docElement = document.documentElement;

/**
 * Sets the application theme (light/dark) and updates the UI accordingly.
 * @param {'light' | 'dark'} theme The theme to set.
 */
const setTheme = (theme) => {
    if (theme === 'dark') {
        docElement.classList.add('dark');
        themeToggleLightIcon.classList.remove('hidden');
        themeToggleDarkIcon.classList.add('hidden');
        monaco.editor.setTheme('vs-dark');
        localStorage.setItem('theme', 'dark');
    } else {
        docElement.classList.remove('dark');
        themeToggleLightIcon.classList.add('hidden');
        themeToggleDarkIcon.classList.remove('hidden');
        monaco.editor.setTheme('vs-light'); // Use the light theme for Monaco
        localStorage.setItem('theme', 'light');
    }
    // Re-render graphs with the new theme
    if (current_res && stateSelector.value) {
        const dot = current_res.dotOfState(stateSelector.value);
        renderGraph(dot);
    }
    updateThemeForGraph();
};

// --- Core Functions ---

/**
 * Renders a DOT string into an SVG and displays it in the preview container.
 * @param {string} dotString The DOT graph notation string.
 */
function renderGraph(dotString) {
    if (!graphviz || !dotString) {
        svgContainer.innerHTML = `<p class="text-gray-500">Could not generate graph.</p>`;
        return;
    }
    try {
        let svg = graphviz.dot(dotString);
        // replace fill="white" with fill="none" to avoid white background
        svg = svg.replace(/fill="white"/g, 'fill="none"');
        svg = svg.replace(/<text /g, '<text fill="white" ');
        // if theme is dark, set 'stroke="black"' to 'stroke="white"'
        if (docElement.classList.contains('dark')) {
            svg = svg.replace(/stroke="black"/g, 'stroke="white"');
            svg = svg.replace(/<text fill="black"/g, '<text fill="white"');
        } else {
            svg = svg.replace(/stroke="white"/g, 'stroke="black"');
            svg = svg.replace(/<text fill="white"/g, '<text fill="black"');
        }
        svgContainer.innerHTML = svg;
    } catch (error) {
        console.error("Graphviz rendering error:", error);
        svgContainer.innerHTML = `<p class="text-red-500">Error rendering graph. Check console for details.</p>`;
    }
}

/**
 * Populates the state selector dropdown with names from the compilation result.
 * @param {any} res The successful compilation result from Context.parse.
 */
function populateStateSelector(res) {
    stateSelector.innerHTML = ''; // Clear previous options

    const stateNames = [...res.listStates()].map(state => state.toString());

    if (stateNames.length === 0) {
        const option = new Option('No states found in code', '');
        option.disabled = true;
        stateSelector.add(option);
        svgContainer.innerHTML = `<p class="text-gray-500">No states were found to visualize.</p>`;
        return;
    }

    stateNames.forEach(name => stateSelector.add(new Option(name, name)));

    // Automatically render the graph for the first state
    if (stateNames.length > 0) {
        const dot = res.dotOfState(stateNames[0]);
        renderGraph(dot);
    }
}

function highlightError(line, column) {

    // Highlight the error in the editor
    editor.revealLineInCenter(line);
    editor.focus();
    editor.setPosition({ lineNumber: line, column: column });

    // show decoration
}

let MS_BETWEEN_CODE_CHANGES = 200; // Throttle code change events to avoid excessive parsing

/**
 * Wrapper around onCodeChangedInner to throttle calls.
 */
function onCodeChanged() {
    if (onCodeChanged.timeout) {
        clearTimeout(onCodeChanged.timeout);
    }
    onCodeChanged.timeout = setTimeout(onCodeChangedInner, MS_BETWEEN_CODE_CHANGES);
}

/**
 * This function is called whenever the content in the Monaco editor changes.
 * It attempts to compile the code and updates the UI accordingly.
 */
function onCodeChangedInner() {
    const current_content = editor.getValue();
    localStorage.setItem('last_code', current_content);

    try {
        // Attempt to parse/compile the code
        const res = Context.parse(current_content);
        current_res = res;

        // reset the error on the editor
        editorDecorations.clear()
        // now check if it was parsed successfully
        let error_msg_raw = res.errorMessage();
        if (error_msg_raw === "") {
            // Update UI for success
            outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-green-400";
            outputPre.innerText = "Code compiled successfully!";
        } else {
            // error
            outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-red-400";

            let error_msg = ansi_up.ansi_to_html(error_msg_raw);

            // raw error_msg text:
            // const tempDiv = document.createElement('div');
            // tempDiv.innerHTML = error_msg;
            // let error_msg_text_form = tempDiv.innerText; // Get the raw text without HTML tags
            // // delete
            // tempDiv.remove();
            //
            // let line_arr = error_msg_text_form.split("\n").map((line) => {return {value: line}});


            // for every actual span returned in the error, add a decoration
            for (const span of res.errorSpans()) {
                // console.log("Highlighting error span:", span);
                // create a range for the decoration
                const range = new monaco.Range(span.lineStart, span.colStart, span.lineEnd, span.colEnd);
                editorDecorations.append([
                    {
                        range: range,
                        options: {
                            className: 'error-highlight',
                            isWholeLine: false,
                            hoverMessage: [
                                {
                                    value: "error here",
                                },
                            ],
                            // TODO: would be cool if every errorSpan had was more 'rich' in the sense that it also contained a
                            //  specific message to show. 
                            // glyphMarginHoverMessage: "glyph haha",
                        }
                    }
                ]);
            }

            // find the portion of the error message that contains the error, it's of the form input:line:column
            outputPre.innerHTML = error_msg;
            const errorMatches = error_msg.matchAll(/input:(\d+):(\d+)/g);
            let i = 0;
            const added_ids_and_spans = [];
            for (const errorMatch of errorMatches) {
                // add a link to the editor position
                const line = parseInt(errorMatch[1], 10);
                const column = parseInt(errorMatch[2], 10);

                // console.log("Replacing for match: ", errorMatch[0], "at line:", line, "column:", column);

                // replace the match in the error_msg to be a link that invokes the following function on click:
                outputPre.innerHTML = outputPre.innerHTML.replace(errorMatch[0], `<a href="#" id="error-span-link-${i}" class="text-red-400 underline">input:${line}:${column}</a>`);
                added_ids_and_spans.push({ id: `error-span-link-${i}`, line: line, column: column });
                i = i + 1;
            }
            // now attach event listeners
            for (const id_and_span of added_ids_and_spans) {
                const line = id_and_span.line;
                const column = id_and_span.column;
                // console.log(`Attaching click handler for error at line ${line}, column ${column}`);
                // find and attach a click handler to the error span
                const errorSpanLink = document.getElementById(id_and_span.id);
                i = i + 1;
                if (errorSpanLink) {
                    // console.log(`IF line ${line}, column ${column}`);

                    errorSpanLink.addEventListener('click', (e) => {
                        // console.log(`Highlighting error at line ${line}, column ${column}`);
                        e.preventDefault();
                        highlightError(line, column);
                    });
                }
            }

        }


        // Populate the dropdown with discovered states
        populateStateSelector(res);
        populateOperationSelector(res); // 🚀
    } catch (e) {
        current_res = null; // Invalidate old results on crash

        // Update UI for error
        const errorMessage = e.cause ? e.cause.toString() : e.toString();
        outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap font-mono text-sm text-red-400";
        outputPre.innerHTML = ansi_up.ansi_to_html(errorMessage);

        // Clear the state selector and SVG preview
        stateSelector.innerHTML = '<option value="">Fix code errors to see states</option>';
        svgContainer.innerHTML = `<p class="text-gray-500">Your state graph will appear here.</p>`;
        operationSelector.innerHTML = '<option value="">Fix code errors to see operations</option>'; // 🚀
    }
}

// --- Event Listeners ---

// Listen for changes in the editor model
editor.onDidChangeModelContent(() => {
    onCodeChanged();
});

// Listen for changes in the state selector dropdown
stateSelector.addEventListener('change', (e) => {
    if (current_res && e.target.value) {
        renderGraph(current_res.dotOfState(e.target.value));
    }
});
themeToggleBtn.addEventListener('click', () => {
    const newTheme = docElement.classList.contains('dark') ? 'light' : 'dark';
    setTheme(newTheme);
});

// 🚀 --- DYNAMIC GRAPH BUILDER --- 🚀 //

// --- State and Variables ---
let interactiveNodes = [];
let interactiveEdges = [];
let concreteGraph = ConcreteGraph.new_()
let simulation, svg, svgRoot, gridG, zoomableContainer;
let selectedSourceNode = null;
let currentTransform = d3.zoomIdentity;
let tempNodeCoords = { x: 0, y: 0 };
let pendingEdge = null;
let currentTraceDots = [];
let rawCurrentTraceDots = [];
let currentTraceIndex = 0;
let traceGraphviz;

// --- DOM References ---
const operationSelector = document.getElementById('operation-selector');
const operationInputsContainer = document.getElementById('operation-inputs');
const addNodeInputBtn = document.getElementById('add-node-input-btn');
const runOperationBtn = document.getElementById('run-operation-btn');
const interactiveGraphContainer = document.getElementById('interactive-graph-container');
const nodeModal = document.getElementById('node-modal');
const nodeForm = document.getElementById('node-form');
const cancelNodeBtn = document.getElementById('cancel-node-btn');
const nodeNameInput = document.getElementById('node-name');
const nodeValueInput = document.getElementById('node-value');
const edgeModal = document.getElementById('edge-modal');
const edgeForm = document.getElementById('edge-form');
const cancelEdgeBtn = document.getElementById('cancel-edge-btn');
const edgeValueInput = document.getElementById('edge-value');
edgeValueInput.required = false;

const traceBox = document.getElementById('trace-box');
const prevTraceBtn = document.getElementById('prev-trace-btn');
const nextTraceBtn = document.getElementById('next-trace-btn');
const traceStepIndicator = document.getElementById('trace-step-indicator');
const traceGraphContainer = document.getElementById('trace-graph-container');
let isTraceRendering = false;

const runSimulationBtn = document.getElementById('run-simulation-btn');

// --- Delete Node Elements and Logic ---
const deleteNodeBtn = document.getElementById('delete-node-btn');
const deleteConfirmModal = document.getElementById('delete-confirm-modal');
const cancelDeleteBtn = document.getElementById('cancel-delete-btn');
const confirmDeleteBtn = document.getElementById('confirm-delete-btn');


// --- auto compile input and logic

const autoCompileInput = document.getElementById('auto-compile-timeout');
autoCompileInput.addEventListener('change', (e) => {
    const newValue = parseInt(e.target.value, 10);
    if (isNaN(newValue) || newValue < 0) {
        alert("Please enter a valid positive number for auto-compile timeout.");
        return;
    }
    MS_BETWEEN_CODE_CHANGES = newValue;
    localStorage.setItem('auto_compile_timeout', newValue);
});

// --- skip duplicate traces input
const skipDuplicateTracesInput = document.getElementById('skip-duplicate-traces');
// it's a check button
skipDuplicateTracesInput.addEventListener('change', (e) => {
    const skipDuplicates = e.target.checked;
    localStorage.setItem('skip_duplicate_traces', skipDuplicates);
    // change the label text accordingly
    if (skipDuplicates) {
        currentTraceDots = rawCurrentTraceDots.filter((dot, index, arr) => index === 0 || dot !== arr[index - 1]);
    } else {
        currentTraceDots = rawCurrentTraceDots; // Reset to raw if unchecked
    }
    currentTraceIndex = Math.min(currentTraceIndex, currentTraceDots.length - 1); // dont go out of bounds
    // render the trace graph again
    renderTraceGraph();
});

/**
 * Deletes the currently selected node and its connected edges.
 */
function deleteSelectedNode() {
    if (!selectedSourceNode) return;

    // Remove from the underlying graph representation
    concreteGraph.deleteNode(selectedSourceNode.nodeKey);

    // Filter out the node and its edges from the D3 data arrays
    interactiveNodes = interactiveNodes.filter(n => n.id !== selectedSourceNode.id);
    interactiveEdges = interactiveEdges.filter(e => e.source.id !== selectedSourceNode.id && e.target.id !== selectedSourceNode.id);

    // Reset selection state
    selectedSourceNode = null;
    deleteNodeBtn.classList.add('hidden');

    // Re-render the graph and close the modal
    updateInteractiveGraph();
    deleteConfirmModal.close();
}


// --- Core D3 Functions ---

function updateInteractiveGraph() {
    if (!svg) return;
    const isDark = docElement.classList.contains('dark');
    const nodeStrokeColor = isDark ? "#A0AEC0" : "#4A5568";
    const nodeTextColor = isDark ? "#F7FAFC" : "#1A202C";
    const edgeColor = isDark ? "#718096" : "#4A5568";

    // Edges
    const edge = svg.select(".links").selectAll("g.edge-group").data(interactiveEdges, d => `${d.source.id}-${d.target.id}`);
    edge.exit().remove();
    const edgeEnter = edge.enter().append("g").attr("class", "edge-group");
    edgeEnter.append("line").attr("stroke-width", 2).attr("marker-end", "url(#arrow)");
    edgeEnter.append("text").attr("class", "edge-label").attr("text-anchor", "middle").attr("dy", -5).style("font-size", "12px").style("pointer-events", "none");
    const edgeUpdate = edgeEnter.merge(edge);
    edgeUpdate.select("line").attr("stroke", edgeColor);
    edgeUpdate.select("text").text(d => d.value).attr("fill", nodeTextColor);

    // Nodes
    const node = svg.select(".nodes").selectAll("g.node").data(interactiveNodes, d => d.id);
    node.exit().remove();
    const nodeEnter = node.enter().append("g").attr("class", "node").call(d3.drag().on("start", dragstarted).on("drag", dragged).on("end", dragended));
    nodeEnter.append("circle").attr("r", 25).attr("stroke-width", 3);
    nodeEnter.append("text").attr("class", "node-value").attr("text-anchor", "middle").attr("dy", ".3em").style("font-size", "14px").style("font-weight", "bold").style("pointer-events", "none");
    nodeEnter.append("text").attr("class", "node-name").attr("text-anchor", "middle").attr("y", 40).style("font-size", "12px").style("pointer-events", "none");
    const nodeUpdate = nodeEnter.merge(node);
    nodeUpdate.select("circle").attr("fill", d => d.id === selectedSourceNode?.id ? "#63B3ED" : (isDark ? "#2D3748" : "#E2E8F0")).attr("stroke", nodeStrokeColor);
    nodeUpdate.select(".node-value").text(d => d.value).attr("fill", nodeTextColor);
    nodeUpdate.select(".node-name").text(d => d.name).attr("fill", nodeTextColor);
    nodeUpdate.on("click", (event, d) => {
        event.stopPropagation();
        if (!selectedSourceNode) {
            selectedSourceNode = d;
            deleteNodeBtn.classList.remove('hidden'); // Show delete button
        } else {
            if (selectedSourceNode.id !== d.id) {
                pendingEdge = { source: selectedSourceNode, target: d };
                edgeValueInput.value = "";
                edgeModal.showModal();
                edgeValueInput.focus();
            }
            selectedSourceNode = null;
            deleteNodeBtn.classList.add('hidden'); // Hide delete button
        }
        updateInteractiveGraph();
    });

    simulation.nodes(interactiveNodes).on("tick", ticked);
    simulation.force("link").links(interactiveEdges).id(d => d.id);
    simulation.alpha(0.3).restart();

    function ticked() {
        edgeUpdate.select("line").attr("x1", d => d.source.x).attr("y1", d => d.source.y).attr("x2", d => d.target.x).attr("y2", d => d.target.y);
        edgeUpdate.select("text").attr("x", d => (d.source.x + d.target.x) / 2).attr("y", d => (d.source.y + d.target.y) / 2);
        nodeUpdate.attr("transform", d => `translate(${d.x},${d.y})`);
    }
    function dragstarted(event, d) { if (!event.active) simulation.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; }
    function dragged(event, d) { d.fx = event.x; d.fy = event.y; }
    function dragended(event, d) { if (!event.active) simulation.alphaTarget(0); d.fx = null; d.fy = null; }
}

/**
 * Draws a smooth, pannable, and zoomable grid background.
 */
function drawGrid(gridG, width, height, transform) {
    const isDark = document.documentElement.classList.contains('dark');
    const gridColor = isDark ? "rgba(255, 255, 255, 0.1)" : "rgba(0, 0, 0, 0.1)";
    const gridSpacing = 50;

    // Calculate the spacing and offset for lines based on the current zoom level.
    // This creates the visual effect of an infinite grid.
    const lineSpacing = gridSpacing * transform.k;
    const offsetX = transform.x % lineSpacing;
    const offsetY = transform.y % lineSpacing;

    // Clear any previous grid lines to redraw them.
    gridG.selectAll("*").remove();

    // Generate the data for the new line positions.
    const xLines = d3.range(offsetX, width + 1, lineSpacing);
    const yLines = d3.range(offsetY, height + 1, lineSpacing);
    gridG.selectAll(".grid-line-v").data(xLines).enter().append("line").attr("x1", d => d).attr("y1", 0).attr("x2", d => d).attr("y2", height);
    gridG.selectAll(".grid-line-h").data(yLines).enter().append("line").attr("x1", 0).attr("y1", d => d).attr("x2", width).attr("y2", d => d);
    gridG.selectAll("line").attr("stroke", gridColor).attr("stroke-width", 1);
}

function handleResize() {
    if (!interactiveGraphContainer || !svgRoot || !gridG) return;

    const width = interactiveGraphContainer.clientWidth;
    const height = interactiveGraphContainer.clientHeight;

    svgRoot.attr("width", width).attr("height", height);
    drawGrid(gridG, width, height, currentTransform);
}

function updateThemeForGraph() {
    if (!svg) return;
    const isDark = docElement.classList.contains('dark');
    const arrowheadColor = isDark ? "#718096" : "#4A5568";
    d3.select("#arrow path").style("fill", arrowheadColor);
    updateInteractiveGraph();
    const width = interactiveGraphContainer.clientWidth;
    const height = interactiveGraphContainer.clientHeight;
    drawGrid(gridG, width, height, currentTransform);
}

function initInteractiveGraph() {
    const width = interactiveGraphContainer.clientWidth;
    const height = interactiveGraphContainer.clientHeight;

    svgRoot = d3.select(interactiveGraphContainer).append("svg").attr("width", width).attr("height", height)
        .on("dblclick", (event) => {
            event.preventDefault();
            const [mx, my] = d3.pointer(event);
            const tempNodeCoordsArr = currentTransform.invert([mx, my]);
            tempNodeCoords.x = tempNodeCoordsArr[0];
            tempNodeCoords.y = tempNodeCoordsArr[1];
            nodeNameInput.value = `node${interactiveNodes.length + 1}`;
            nodeValueInput.value = "";
            nodeModal.showModal();
            nodeValueInput.focus();
            nodeValueInput.select();
            // disallow browser dropdown for node values
            nodeValueInput.setAttribute("autocomplete", "off");
        })
        .on("click", () => {
            if (selectedSourceNode) {
                selectedSourceNode = null;
                deleteNodeBtn.classList.add('hidden'); // Hide on deselect
                updateInteractiveGraph();
            }
        });

    svgRoot.append("defs").append("marker").attr("id", "arrow").attr("viewBox", "0 -5 10 10").attr("refX", 33).attr("refY", 0).attr("markerWidth", 6).attr("markerHeight", 6).attr("orient", "auto").append("path").attr("d", "M0,-5L10,0L0,5").attr("class", "arrowhead");

    gridG = svgRoot.append("g").attr("class", "grid");

    zoomableContainer = svgRoot.append("g");
    svg = zoomableContainer; // Main container for nodes and links
    svg.append("g").attr("class", "links");
    svg.append("g").attr("class", "nodes");

    const zoomBehavior = d3.zoom().scaleExtent([0.1, 8]).on("zoom", (event) => {
        currentTransform = event.transform;
        zoomableContainer.attr("transform", currentTransform);
        drawGrid(gridG, interactiveGraphContainer.clientWidth, interactiveGraphContainer.clientHeight, currentTransform);
    });
    svgRoot.call(zoomBehavior).on("dblclick.zoom", null);

    drawGrid(gridG, width, height, currentTransform);

    simulation = d3.forceSimulation(interactiveNodes)
        .force("link", d3.forceLink(interactiveEdges).id(d => d.id).distance(150).strength(0.6))
        .force("charge", d3.forceManyBody().strength(-800))
        .force("positionX", d3.forceX(300))
        .force("positionY", d3.forceY(300));

    updateThemeForGraph();
}

// --- Event Handlers for UI ---

nodeForm.addEventListener('submit', () => {
    const name = nodeNameInput.value.trim();
    const value = nodeValueInput.value.trim();
    if (name && interactiveNodes.find(n => n.name === name)) {
        alert("A node with this name already exists.");
        return;
    }
    let nodeKey = concreteGraph.addNode(value);
    const newNode = { id: crypto.randomUUID(), name, value, x: tempNodeCoords.x, y: tempNodeCoords.y, nodeKey: nodeKey, fx: tempNodeCoords.x, fy: tempNodeCoords.y };
    interactiveNodes.push(newNode);
    updateInteractiveGraph();
    setTimeout(() => { newNode.fx = null; newNode.fy = null; }, 150);
});
cancelNodeBtn.addEventListener('click', () => nodeModal.close());

edgeForm.addEventListener('submit', () => {
    if (!pendingEdge) return;
    // if interactiveEdges already contains this edge, remove it first
    const existingEdgeIndex = interactiveEdges.findIndex(e => e.source.id === pendingEdge.source.id && e.target.id === pendingEdge.target.id);
    if (existingEdgeIndex !== -1) {
        interactiveEdges.splice(existingEdgeIndex, 1);
    }
    interactiveEdges.push({ source: pendingEdge.source, target: pendingEdge.target, value: edgeValueInput.value.trim() });
    concreteGraph.addEdge(pendingEdge.source.nodeKey, pendingEdge.target.nodeKey, edgeValueInput.value.trim());
    pendingEdge = null;
    updateInteractiveGraph();
});
cancelEdgeBtn.addEventListener('click', () => { pendingEdge = null; edgeModal.close(); });

addNodeInputBtn.addEventListener('click', () => {
    const inputCount = operationInputsContainer.children.length;
    const inputWrapper = document.createElement('div');
    inputWrapper.className = 'relative';
    const newInput = document.createElement('input');
    newInput.type = 'text';
    newInput.placeholder = `Input ${inputCount + 1}`;
    newInput.className = 'bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md p-2 pl-3 w-32 focus:ring-2 focus:ring-blue-500 focus:outline-none';
    const removeBtn = document.createElement('button');
    removeBtn.innerHTML = '&times;';
    removeBtn.className = 'absolute right-1 top-1/2 -translate-y-1/2 text-gray-400 hover:text-red-500 font-bold p-1 leading-none';
    removeBtn.onclick = () => {
        inputWrapper.remove();
        Array.from(operationInputsContainer.querySelectorAll('input')).forEach((inp, i) => { inp.placeholder = `Input ${i + 1}`; });
    };
    inputWrapper.appendChild(newInput);
    inputWrapper.appendChild(removeBtn);
    operationInputsContainer.appendChild(inputWrapper);
    return removeBtn.onclick;
});

runOperationBtn.addEventListener('click', () => {
    const operationName = operationSelector.value;
    if (!operationName) { alert("Please select an operation to run."); return; }
    const inputNodeNames = Array.from(operationInputsContainer.querySelectorAll('input')).map(input => input.value.trim()).filter(name => name !== "");
    executeOperation(operationName, inputNodeNames);
});

runSimulationBtn.addEventListener('click', () => {
    simulation.tick(100);
    updateInteractiveGraph();
})

// --- Delete Node Event Listeners ---
deleteNodeBtn.addEventListener('click', () => {
    if (selectedSourceNode) {
        deleteConfirmModal.showModal();
    }
});
cancelDeleteBtn.addEventListener('click', () => deleteConfirmModal.close());
confirmDeleteBtn.addEventListener('click', deleteSelectedNode);

window.addEventListener('keydown', (e) => {
    if ((e.key === 'Delete' || e.key === 'Backspace') && selectedSourceNode) {
        // Prevent typing 'Backspace' in an input field from triggering deletion
        if (document.activeElement.tagName === 'INPUT' || document.activeElement.tagName === 'TEXTAREA') {
            return;
        }
        e.preventDefault();
        deleteConfirmModal.showModal();
    }
});
window.addEventListener('resize', handleResize);


// --- User-Adaptable Functions ---

/**
 * Populates the operation selector dropdown.
 */
function populateOperationSelector(res) {
    operationSelector.innerHTML = '';
    const opNames = [...res.listOperations()].map(op => op.toString());

    if (opNames.length === 0) {
        operationSelector.add(new Option('No operations found', '', true, true));
        return;
    }
    opNames.forEach(name => operationSelector.add(new Option(name, name)));
}

/**
 * This is called when the "Run" button is clicked. You have access to `interactiveNodes` and `interactiveEdges`.
 */
function executeOperation(operationName, inputNodeNames) {
    console.log("Executing operation:", operationName, "with inputs:", inputNodeNames);
    const inputNodes = inputNodeNames.map(name => interactiveNodes.find(node => node.name === name)).filter(Boolean);

    if (inputNodes.length !== inputNodeNames.length) {
        alert("One or more input nodes could not be found in the graph.");
        return;
    }

    let inputKeys = inputNodes.map(node => node.nodeKey);

    try {
        let res = current_res.runOperation(concreteGraph, operationName, inputKeys);
        let traceDotsRaw = res.chainedDotTrace();

        if (traceDotsRaw && traceDotsRaw.trim() !== "") {
            currentTraceDots = traceDotsRaw.split('---').map(dot => dot.trim()).filter(dot => dot);
            rawCurrentTraceDots = currentTraceDots; // Keep the raw trace dots for reference
            if (skipDuplicateTracesInput.checked) {
                // deduplicate consecutive identical trace dots
                currentTraceDots = currentTraceDots.filter((dot, index, arr) => index === 0 || dot !== arr[index - 1]);
            }
            // deduplicate consecutive identical trace dots
            // note: we're deduping now with skipDuplicateTracesInput
            // currentTraceDots = currentTraceDots.filter((dot, index, arr) => index === 0 || dot !== arr[index - 1]);
            currentTraceIndex = 0;
            traceBox.style.display = 'block';
            renderTraceGraph();
            traceBox.scrollIntoView({ behavior: 'smooth' });
        } else {
            currentTraceDots = [];
            traceBox.style.display = 'none';
        }

        const avgX = inputNodes.reduce((sum, n) => sum + (n.x || 0), 0) / inputNodes.length;
        const avgY = inputNodes.reduce((sum, n) => sum + (n.y || 0), 0) / inputNodes.length;

        // now make the current interactiveEdges and interactiveNodes synchronized with concreteGraph
        let newInteractiveNodes = [];
        let newInteractiveEdges = [];

        const addNode = (key, name, value) => {
            let thisX = avgX + Math.random() * 50; // Randomly offset to avoid overlap
            let thisY = avgY + Math.random() * 50; // Randomly offset to avoid overlap
            // check if the name already exists, if so, append a number to it
            let foundName = interactiveNodes.find(n => n.name === name);
            while (foundName) {
                const parts = name.match(/^(.*?)(\d+)?$/);
                const baseName = parts[1];
                const num = parts[2] ? parseInt(parts[2]) + 1 : 1;
                name = `${baseName}${num}`;
                foundName = interactiveNodes.find(n => n.name === name);
            }
            newInteractiveNodes.push({ id: crypto.randomUUID(), name, value, x: thisX, y: thisY, nodeKey: key });
        }

        for (const knownNewNode of res) {
            let nodeKey = knownNewNode.key();
            let name = knownNewNode.name();
            let value = knownNewNode.value();
            // if "new node" is actually an existing, shape matched node, then we need to just update its name
            // NOTE: this is a special case we only need to handle here, in the _concrete_ setting,
            //  since abstractly a returned node can never already exist in the abstract graph.
            let existingNode = interactiveNodes.find(n => n.nodeKey === nodeKey);
            if (existingNode) {
                existingNode.name = name; //Update name if it exists
                existingNode.value = value; // Update value if it exists
                newInteractiveNodes.push(existingNode); // Keep existing nodes
            } else {
                addNode(nodeKey, name, value);
            }
        }


        for (const node of concreteGraph.getNodes()) {
            let key = node.key();
            let value = node.value();

            let alreadyExists = newInteractiveNodes.find(n => n.nodeKey === key);
            if (alreadyExists) {
                // we added this already with a proper name
                continue;
            }

            let existingNode = interactiveNodes.find(n => n.nodeKey === key);
            if (!existingNode) {
                addNode(key, key, value);
            } else {
                existingNode.value = value; // Update value if it exists
                newInteractiveNodes.push(existingNode); // Keep existing nodes
            }
        }
        for (const edge of concreteGraph.getEdges()) {
            let sourceKey = edge.src();
            let targetKey = edge.dst();
            let value = edge.weight();

            let sourceNode = newInteractiveNodes.find(n => n.nodeKey === sourceKey);
            let targetNode = newInteractiveNodes.find(n => n.nodeKey === targetKey);

            if (sourceNode && targetNode) {
                // Check if the edge already exists
                let existingEdge = interactiveEdges.find(e => e.source.nodeKey === sourceKey && e.target.nodeKey === targetKey);
                if (!existingEdge) {
                    newInteractiveEdges.push({ source: sourceNode, target: targetNode, value });
                } else {
                    // Update the value of the existing edge if it already exists
                    existingEdge.value = value;
                    // also update the source and target nodes
                    existingEdge.source = sourceNode;
                    existingEdge.target = targetNode;
                    newInteractiveEdges.push(existingEdge);
                }
            }
        }
        // update the interactive nodes and edges
        interactiveNodes = newInteractiveNodes;
        interactiveEdges = newInteractiveEdges;
    } catch (e) {
        let errorMessage = e.cause ? e.cause.toString() : e.toString();
        alert(errorMessage);
        console.error("Error running operation:", e);
    }

    updateInteractiveGraph(); // Refresh the graph to show changes
}

function renderTraceGraph() {
    if (currentTraceDots.length === 0) {
        traceBox.style.display = 'none';
        return;
    }

    traceBox.style.display = 'block';
    const dot = currentTraceDots[currentTraceIndex];
    console.log("Rendering trace graph for index:", currentTraceIndex, "with DOT:\n", dot);

    // Apply theme adjustments to the DOT string
    // let themedDot = dot.replace(/fill="white"/g, 'fill="none"');
    // themedDot = themedDot.replace(/<text /g, '<text fill="white" ');
    // if (docElement.classList.contains('dark')) {
    //     themedDot = themedDot.replace(/stroke="black"/g, 'stroke="white"');
    //     themedDot = themedDot.replace(/<text fill="black"/g, '<text fill="white"');
    // } else {
    //     themedDot = themedDot.replace(/stroke="white"/g, 'stroke="black"');
    //     themedDot = themedDot.replace(/<text fill="white"/g, '<text fill="black"');
    // }

    let themedDot = dot;
    isTraceRendering = true; // Prevent further interactions during rendering
    traceGraphviz
        .transition(() => d3.transition().duration(300).ease(d3.easeLinear))
        .renderDot(themedDot)
        .on("end", function () {
            // // adjust the svg inside trace-graph-container to be full width and height:
            // const traceGraphSvg = document.querySelector("#trace-graph-container svg");
            // if (traceGraphSvg) {
            //     // traceGraphSvg.style.width = "100%";
            //     // traceGraphSvg.style.height = "100%";
            // }
            // scroll to the bottom of the trace box
            // traceBox.scrollIntoView({ behavior: 'smooth' });
            isTraceRendering = false; // Allow further interactions after rendering
        });

    let traceContainer = document.querySelector("#trace-graph-container");
    // disable moving around via drag and drop
    if (traceContainer) {
        traceContainer.style.pointerEvents = 'none'; // Disable pointer events to prevent dragging
    }


    traceStepIndicator.textContent = `${currentTraceIndex + 1} / ${currentTraceDots.length}`;
    prevTraceBtn.disabled = currentTraceIndex === 0;
    nextTraceBtn.disabled = currentTraceIndex === currentTraceDots.length - 1;
}


// --- Initial Load ---
async function main() {
    // Initialize panic hooks
    Context.init();
    // Load Graphviz WASM module
    outputPre.innerText = "Loading Graphviz WASM module...";
    graphviz = await Graphviz.load();

    if (localStorage.getItem("theme")) {
        setTheme(localStorage.getItem("theme"));
    }
    if (localStorage.getItem("auto_compile_timeout")) {
        MS_BETWEEN_CODE_CHANGES = parseInt(localStorage.getItem("auto_compile_timeout"), 10);
        autoCompileInput.value = MS_BETWEEN_CODE_CHANGES;
    }
    if (localStorage.getItem("skip_duplicate_traces")) {
        skipDuplicateTracesInput.checked = localStorage.getItem("skip_duplicate_traces") === 'true';
    }

    // Perform initial compilation
    onCodeChanged();
    initInteractiveGraph(); // 🚀
    initTraceViewer();

    // add a Ctrl-S event listener to the monaco editor that immediately runs onCodeChangedInner()
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
        onCodeChangedInner();
    });
}

/// Advances the current trace index by respecting the value of
function advanceTrace() {
    currentTraceIndex++;
    return;
    // note: we're handling with rawCurrentTraceDots now
    // let doSkip = skipDuplicateTracesInput.checked;
    // if (doSkip) {
    //     // skip to the next unique trace dot
    //     while (currentTraceIndex < currentTraceDots.length - 1 && currentTraceDots[currentTraceIndex] === currentTraceDots[currentTraceIndex + 1]) {
    //         currentTraceIndex++;
    //     }
    // } else {
    //     // just increment the index
    //     if (currentTraceIndex < currentTraceDots.length - 1) {
    //         currentTraceIndex++;
    //     }
    // }
}

function previousTrace() {
    currentTraceIndex--;
    return;
    // note: we're handling with rawCurrentTraceDots now
    // let doSkip = skipDuplicateTracesInput.checked;
    // if (doSkip) {
    //     // skip to the previous unique trace dot
    //     while (currentTraceIndex > 0 && currentTraceDots[currentTraceIndex] === currentTraceDots[currentTraceIndex - 1]) {
    //         currentTraceIndex--;
    //     }
    // } else {
    //     // just decrement the index
    //     if (currentTraceIndex > 0) {
    //         currentTraceIndex--;
    //     }
    // }
}

function initTraceViewer() {
    traceGraphviz = d3.select("#trace-graph-container").graphviz({
        useWorker: false,
        fit: true,
    });

    // add child to traceGraphContainer to provide some buffer
    if (traceGraphContainer) {
        let divChild = document.createElement('div');
        divChild.className = "h-[100vh] bg-white";
        traceGraphContainer.appendChild(divChild);
    }

    prevTraceBtn.addEventListener('click', () => {
        if (isTraceRendering) return; // Prevent navigation while rendering
        if (currentTraceIndex > 0) {
            previousTrace();
            renderTraceGraph();
        }
    });

    nextTraceBtn.addEventListener('click', () => {
        if (isTraceRendering) return; // Prevent navigation while rendering
        if (currentTraceIndex < currentTraceDots.length - 1) {
            advanceTrace();
            renderTraceGraph();
        }
    });

    // add left and right arrow key listeners to navigate the trace
    traceBox.addEventListener('keydown', (e) => {
        if (isTraceRendering) return; // Prevent navigation while rendering
        if (e.key === 'ArrowLeft' && currentTraceIndex > 0) {
            previousTrace();
            renderTraceGraph();
        } else if (e.key === 'ArrowRight' && currentTraceIndex < currentTraceDots.length - 1) {
            advanceTrace();
            renderTraceGraph();
        }
    });
}

await main();
