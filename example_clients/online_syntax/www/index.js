import {Graphviz} from "@hpcc-js/wasm";
import {Context} from "online-syntax-js";
import * as monaco from 'monaco-editor';
import { AnsiUp } from 'ansi_up';

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
    // to issue a re-render of the SVG container
    onCodeChanged()
};

if (localStorage.getItem("theme")) {
    setTheme(localStorage.getItem("theme"));
}

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

    stateNames.forEach(name => {
        stateSelector.add(new Option(name, name));
    });

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
        current_res = res; // Store successful result

        // Update UI for success
        outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-green-400";
        outputPre.innerText = "Code parsed successfully!";

        // Populate the dropdown with discovered states
        populateStateSelector(res);

    } catch (e) {
        current_res = null; // Invalidate old results on error

        // Update UI for error
        const errorMessage = e.cause ? e.cause.toString() : e.toString();
        outputPre.className = "w-full p-4 overflow-auto whitespace-pre-wrap text-red-400";
        outputPre.innerHTML = ansi_up.ansi_to_html(errorMessage);

        // Clear the state selector and SVG preview
        stateSelector.innerHTML = '<option value="">Fix code errors to see states</option>';
        svgContainer.innerHTML = `<p class="text-gray-500">Your state graph will appear here.</p>`;
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
        const dot = current_res.dotOfState(e.target.value);
        renderGraph(dot);
    }
});

// Listener for the theme toggle button
themeToggleBtn.addEventListener('click', () => {
    const currentTheme = docElement.classList.contains('dark') ? 'dark' : 'light';
    const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
    setTheme(newTheme);
});

// --- Initial Load ---
async function main() {
    // Load Graphviz WASM module
    outputPre.innerText = "Loading Graphviz WASM module...";
    graphviz = await Graphviz.load();

    // Perform initial compilation
    onCodeChanged();
}

await main();
