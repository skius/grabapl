import {Graphviz} from "@hpcc-js/wasm";
import {Context} from "online-syntax-js";
import * as monaco from 'monaco-editor';
import { AnsiUp } from 'ansi_up';
import * as d3 from 'd3';

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
fn foo(x: Int) -> (result: Int) {
    show_state(foo_state);
}`;

const editor = monaco.editor.create(document.getElementById('container'), {
    value: initialCode,
    language: 'rust',
    theme: 'vs-dark', // A dark theme is often preferred for code
    automaticLayout: true, // Ensures the editor resizes responsively
    minimap: { enabled: false },
    roundedSelection: false,
    scrollBeyondLastLine: false,
});

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

/**
 * This function is called whenever the content in the Monaco editor changes.
 * It attempts to compile the code and updates the UI accordingly.
 */
function onCodeChanged() {
    const current_content = editor.getValue();
    localStorage.setItem('last_code', current_content);

    try {
        // Attempt to parse/compile the code
        const res = Context.parse(current_content);
        current_res = res;

        // now check if it was parsed successfully
        let error_msg = res.errorMessage();
        if (error_msg === "") {
            // Update UI for success
            outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-green-400";
            outputPre.innerText = "Code parsed successfully!";
        } else {
            // error
            outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-red-400";
            outputPre.innerHTML = ansi_up.ansi_to_html(error_msg);
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
let simulation, svg, svgRoot, gridG, zoomableContainer;
let selectedSourceNode = null;
let currentTransform = d3.zoomIdentity;
let tempNodeCoords = { x: 0, y: 0 };
let pendingEdge = null;

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
        } else {
            if (selectedSourceNode.id !== d.id) {
                pendingEdge = { source: selectedSourceNode, target: d };
                edgeValueInput.value = "";
                edgeModal.showModal();
                edgeValueInput.focus();
            }
            selectedSourceNode = null;
        }
        updateInteractiveGraph();
    });

    // for (const node of interactiveNodes) {
    //     console.log(`Node x: ${node.x}, y: ${node.y}, name: ${node.name}, value: ${node.value}`);
    // }
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
 * This function should be called from within the D3 zoom event handler.
 *
 * @param {d3.Selection} gridG The D3 selection of the <g> element for the grid.
 * @param {number} width The width of the SVG viewport.
 * @param {number} height The height of the SVG viewport.
 * @param {d3.ZoomTransform} transform The current transform from the D3 zoom event.
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

    // Draw the vertical lines.
    gridG.selectAll(".grid-line-v")
        .data(xLines)
        .enter().append("line")
        .attr("x1", d => d)
        .attr("y1", 0)
        .attr("x2", d => d)
        .attr("y2", height);

    // Draw the horizontal lines.
    gridG.selectAll(".grid-line-h")
        .data(yLines)
        .enter().append("line")
        .attr("x1", 0)
        .attr("y1", d => d)
        .attr("x2", width)
        .attr("y2", d => d);

    // Style all the new lines. The stroke width is constant for a crisp look.
    gridG.selectAll("line")
        .attr("stroke", gridColor)
        .attr("stroke-width", 1);
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
            console.log("Adding node at:", tempNodeCoords);
            nodeNameInput.value = `node${interactiveNodes.length + 1}`;
            nodeValueInput.value = "";
            nodeModal.showModal();
            nodeNameInput.focus();
            nodeNameInput.select();
        })
        .on("click", () => {
            if (selectedSourceNode) {
                selectedSourceNode = null;
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
        drawGrid(gridG, width, height, currentTransform);
    });
    svgRoot.call(zoomBehavior).on("dblclick.zoom", null);

    drawGrid(gridG, width, height, currentTransform);

    simulation = d3.forceSimulation(interactiveNodes)
        .force("link", d3.forceLink(interactiveEdges).id(d => d.id).distance(150).strength(0))
        // .force("charge", d3.forceManyBody().strength(-100))
        // .force("center", d3.forceCenter(/*width / 2, height / 2*/).strength(1));
        // .force("positionX", d3.forceX(10).strength(0.5))
        // .force("positionY", d3.forceY(10).strength(0.5))

    updateThemeForGraph();

    console.log(simulation.toString())
}

// --- Event Handlers for UI ---

nodeForm.addEventListener('submit', () => {
    const name = nodeNameInput.value.trim();
    const value = nodeValueInput.value.trim();
    if (name && interactiveNodes.find(n => n.name === name)) {
        alert("A node with this name already exists.");
        return;
    }
    console.log("Adding node:", name, "with value:", value, "at coordinates:", tempNodeCoords);
    interactiveNodes.push({ id: crypto.randomUUID(), name, value, x: tempNodeCoords.x, y: tempNodeCoords.y });
    updateInteractiveGraph();
});
cancelNodeBtn.addEventListener('click', () => nodeModal.close());

edgeForm.addEventListener('submit', () => {
    if (!pendingEdge) return;
    interactiveEdges.push({ source: pendingEdge.source, target: pendingEdge.target, value: edgeValueInput.value.trim() });
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
});

runOperationBtn.addEventListener('click', () => {
    const operationName = operationSelector.value;
    if (!operationName) { alert("Please select an operation to run."); return; }
    const inputNodeNames = Array.from(operationInputsContainer.querySelectorAll('input')).map(input => input.value.trim()).filter(name => name !== "");
    if (inputNodeNames.length === 0) { alert("Please provide at least one input node name."); return; }
    executeOperation(operationName, inputNodeNames);
});

// --- User-Adaptable Functions ---

/**
 * ⚠️ Populates the operation selector dropdown.
 * You may need to adapt `res.listStates()` to your actual API for listing functions.
 */
function populateOperationSelector(res) {
    operationSelector.innerHTML = '';
    // ⚠️ Replace `res.listStates()` if your library has a different method for functions.
    const opNames = [...res.listStates()].map(op => op.toString());

    if (opNames.length === 0) {
        operationSelector.add(new Option('No operations found', '', true, true));
        return;
    }
    opNames.forEach(name => operationSelector.add(new Option(name, name)));
}

/**
 * ⚠️ Placeholder for your execution logic.
 * This is called when the "Run" button is clicked. You have access to `interactiveNodes` and `interactiveEdges`.
 */
function executeOperation(operationName, inputNodeNames) {
    console.log("Executing operation:", operationName, "with inputs:", inputNodeNames);
    const inputNodes = inputNodeNames.map(name => interactiveNodes.find(node => node.name === name)).filter(Boolean);

    if (inputNodes.length !== inputNodeNames.length) {
        alert("One or more input nodes could not be found in the graph.");
        return;
    }

    // --- YOUR CUSTOM LOGIC GOES HERE ---
    alert(`Running '${operationName}' with inputs: ${inputNodeNames.join(', ')}\n(See console for details. Implement your logic in 'executeOperation')`);

    // Example: Create a new result node and connect inputs to it.
    const resultNodeName = `${operationName}_result_${Date.now() % 1000}`;
    const avgX = inputNodes.reduce((sum, n) => sum + (n.x || 0), 0) / inputNodes.length;
    const avgY = inputNodes.reduce((sum, n) => sum + (n.y || 0), 0) / inputNodes.length;
    const newNode = { id: crypto.randomUUID(), name: resultNodeName, value: "✅", x: avgX + 150, y: avgY };
    console.log("Adding node at coordinates:", newNode.x, newNode.y);
    interactiveNodes.push(newNode);
    inputNodes.forEach(inputNode => {
        interactiveEdges.push({ source: inputNode, target: newNode, value: 'input' });
    });

    updateInteractiveGraph(); // Refresh the graph to show changes
}


// --- Initial Load ---
async function main() {
    // Load Graphviz WASM module
    outputPre.innerText = "Loading Graphviz WASM module...";
    graphviz = await Graphviz.load();

    // Perform initial compilation
    onCodeChanged();
    initInteractiveGraph(); // 🚀

    if (localStorage.getItem("theme")) {
        setTheme(localStorage.getItem("theme"));
    }
}

await main();
