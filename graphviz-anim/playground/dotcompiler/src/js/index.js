import * as d3 from 'd3';
import { graphviz } from 'd3-graphviz';
import JSZip from 'jszip';
import { initFFmpeg } from './ffmpeg.js';
import { initResvg } from './resvg-handler.js';
import { exampleDots } from './examples.js';

// Configuration
let config = {
  frameRate: 30,
  transitionDuration: 1500,
  outputFilename: 'graph-animation',
  isRecording: false
};

// State variables
let graphvizInstance;
let svgFrames = [];
let currentDotIndex = 0;
let dotGraphs = [];
let zip = new JSZip();
let ffmpeg;
let resvg;

// DOM Elements
const dotInputEl = document.getElementById('dotInput');
const loadExampleBtn = document.getElementById('loadExample');
const renderGraphBtn = document.getElementById('renderGraph');
const animateGraphBtn = document.getElementById('animateGraph');
const startRecordingBtn = document.getElementById('startRecording');
const stopRecordingBtn = document.getElementById('stopRecording');
const convertToWebmBtn = document.getElementById('convertToWebM');
const frameRateEl = document.getElementById('frameRate');
const transitionDurationEl = document.getElementById('transitionDuration');
const outputFilenameEl = document.getElementById('outputFilename');
const progressEl = document.getElementById('progress-inner');
const statusEl = document.getElementById('status');
const outputVideoEl = document.getElementById('outputVideo');
const downloadLinkEl = document.getElementById('downloadLink');
const downloadFramesLinkEl = document.getElementById('downloadFramesLink');

// Initialize application
async function init() {
  // Initialize Graphviz
  initGraphviz();
  
  // Initialize WASM modules
  try {
    ffmpeg = await initFFmpeg();
    resvg = await initResvg();
    statusEl.textContent = 'Ready. WASM modules loaded.';
  } catch (error) {
    console.error('Failed to load WASM modules:', error);
    statusEl.textContent = 'Error: Failed to load WASM modules.';
  }

  // Set up event listeners
  setupEventListeners();
}

function initGraphviz() {
  graphvizInstance = d3.select("#graph").graphviz()
    .logEvents(true)
    .on("initEnd", () => {
      console.log("Graphviz initialized");
      statusEl.textContent = 'Graphviz initialized';
    });
}

function setupEventListeners() {
  loadExampleBtn.addEventListener('click', loadExampleDots);
  renderGraphBtn.addEventListener('click', renderCurrentGraph);
  animateGraphBtn.addEventListener('click', animateGraphSequence);
  startRecordingBtn.addEventListener('click', startRecording);
  stopRecordingBtn.addEventListener('click', stopRecording);
  convertToWebmBtn.addEventListener('click', convertToWebM);
  
  frameRateEl.addEventListener('change', (e) => {
    config.frameRate = parseInt(e.target.value);
  });
  
  transitionDurationEl.addEventListener('change', (e) => {
    config.transitionDuration = parseInt(e.target.value);
    updateGraphvizTransition();
  });
  
  outputFilenameEl.addEventListener('change', (e) => {
    config.outputFilename = e.target.value;
  });
}

function loadExampleDots() {
  dotInputEl.value = exampleDots.map(dot => dot.join('\n')).join('\n\n---\n\n');
  dotGraphs = exampleDots;
  renderCurrentGraph();
}

function parseDotInput() {
  // Split by separator or empty lines
  const dotText = dotInputEl.value;
  if (!dotText.trim()) {
    alert('Please enter DOT graph code');
    return false;
  }
  
  // Split by separator or empty lines
  dotGraphs = dotText.split(/\n\s*---\s*\n|\n{2,}/)
    .filter(graph => graph.trim())
    .map(graph => graph.trim().split('\n'));
  
  return true;
}

function renderCurrentGraph() {
  if (!parseDotInput() || dotGraphs.length === 0) return;
  
  const dotStr = dotGraphs[currentDotIndex].join('\n');
  graphvizInstance.renderDot(dotStr);
}

function updateGraphvizTransition() {
  graphvizInstance.transition(function() {
    return d3.transition("main")
      .ease(d3.easeLinear)
      .delay(500)
      .duration(config.transitionDuration);
  });
}

async function animateGraphSequence() {
  if (!parseDotInput() || dotGraphs.length === 0) return;
  
  updateGraphvizTransition();
  
  for (let i = 0; i < dotGraphs.length; i++) {
    currentDotIndex = i;
    const dotStr = dotGraphs[i].join('\n');
    
    // Wait for the transition to complete before continuing
    await new Promise(resolve => {
      graphvizInstance
        .renderDot(dotStr)
        .on("end", resolve);
    });
  }
  
  statusEl.textContent = 'Animation complete.';
}

function getSvgString() {
  const svg = document.querySelector("#graph svg");
  if (!svg) return null;
  return new XMLSerializer().serializeToString(svg);
}

async function captureFrame() {
  const svgText = getSvgString();
  if (!svgText) return;
  
  const frameNum = svgFrames.length;
  const filename = `frame_${String(frameNum).padStart(4, '0')}.svg`;
  zip.file(filename, svgText);
  svgFrames.push(svgText);
  
  return svgText;
}

async function startRecording() {
  if (config.isRecording) return;
  
  svgFrames = [];
  config.isRecording = true;
  currentDotIndex = 0;
  statusEl.textContent = 'Recording...';
  
  if (!parseDotInput() || dotGraphs.length === 0) {
    config.isRecording = false;
    statusEl.textContent = 'Nothing to record. Please enter DOT graph code.';
    return;
  }
  
  // Setup for recording frames during transitions
  const frameInterval = 1000 / config.frameRate;
  
  updateGraphvizTransition();
  
  // Record each transition
  for (let i = 0; i < dotGraphs.length; i++) {
    currentDotIndex = i;
    const dotStr = dotGraphs[i].join('\n');
    
    // Start a promise that will resolve when the transition is complete
    const transitionPromise = new Promise(resolve => {
      graphvizInstance
        .renderDot(dotStr)
        .on("end", resolve);
    });
    
    // Capture frames during the transition
    const framesPerStep = Math.ceil((config.transitionDuration + 500) / frameInterval);
    for (let j = 0; j < framesPerStep; j++) {
      await captureFrame();
      await new Promise(resolve => setTimeout(resolve, frameInterval));
    }
    
    // Wait for the transition to complete before continuing
    await transitionPromise;
  }
  
  config.isRecording = false;
  statusEl.textContent = 'Recording finished. Captured ' + svgFrames.length + ' frames.';
  downloadFramesLinkEl.style.display = 'inline-block';
  downloadFramesLinkEl.addEventListener('click', downloadSvgFrames);
}

function stopRecording() {
  config.isRecording = false;
  statusEl.textContent = 'Recording stopped. Captured ' + svgFrames.length + ' frames.';
}

async function downloadSvgFrames() {
  if (svgFrames.length === 0) {
    alert('No frames captured yet');
    return;
  }
  
  const blob = await zip.generateAsync({ type: "blob" });
  const url = URL.createObjectURL(blob);
  
  const a = document.createElement('a');
  a.href = url;
  a.download = `${config.outputFilename}-frames.zip`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

async function convertToWebM() {
  if (svgFrames.length === 0) {
    statusEl.textContent = 'No frames captured yet. Record animation first.';
    return;
  }
  
  if (!ffmpeg || !resvg) {
    statusEl.textContent = 'WASM modules not loaded. Try refreshing the page.';
    return;
  }
  
  statusEl.textContent = 'Converting SVG frames to PNG...';
  progressEl.style.width = '0%';
  
  try {
    const pngFrames = await convertSvgsToPngs(svgFrames);
    statusEl.textContent = 'Converting PNGs to WebM...';
    progressEl.style.width = '50%';
    
    const webmBlob = await convertPngsToWebM(pngFrames);
    progressEl.style.width = '100%';
    
    // Display the video
    const videoUrl = URL.createObjectURL(webmBlob);
    outputVideoEl.src = videoUrl;
    outputVideoEl.style.display = 'block';
    
    // Setup download link
    downloadLinkEl.href = videoUrl;
    downloadLinkEl.download = `${config.outputFilename}.webm`;
    downloadLinkEl.style.display = 'inline-block';
    
    statusEl.textContent = 'Conversion complete! WebM video is ready.';
  } catch (err) {
    console.error('Conversion error:', err);
    statusEl.textContent = `Error during conversion: ${err.message}`;
  }
}

async function convertSvgsToPngs(svgFrames) {
  const pngFrames = [];
  
  for (let i = 0; i < svgFrames.length; i++) {
    try {
      const pngData = await resvg.renderSvg(svgFrames[i]);
      pngFrames.push(pngData);
      
      // Update progress
      const progress = Math.floor((i / svgFrames.length) * 50);
      progressEl.style.width = `${progress}%`;
    } catch (err) {
      console.error(`Error converting SVG frame ${i}:`, err);
      throw err;
    }
  }
  
  return pngFrames;
}

async function convertPngsToWebM(pngFrames) {
  try {
    // Write PNG frames to FFmpeg virtual filesystem
    for (let i = 0; i < pngFrames.length; i++) {
      const filename = `frame_${String(i).padStart(4, '0')}.png`;
      ffmpeg.writeFile(filename, pngFrames[i]);
      
      // Update progress (50% to 75%)
      const progress = 50 + Math.floor((i / pngFrames.length) * 25);
      progressEl.style.width = `${progress}%`;
    }
    
    // Run FFmpeg to convert PNGs to WebM
    const result = await ffmpeg.runCommand([
      '-framerate', String(config.frameRate),
      '-i', 'frame_%04d.png',
      '-c:v', 'libvpx-vp9',
      '-b:v', '2M',
      '-crf', '30',
      '-pix_fmt', 'yuva420p',
      'output.webm'
    ]);
    
    // Read the resulting WebM file
    progressEl.style.width = '90%';
    const webmData = await ffmpeg.readFile('output.webm');
    
    // Create a Blob from the WebM data
    const webmBlob = new Blob([webmData], { type: 'video/webm' });
    
    return webmBlob;
  } catch (err) {
    console.error('FFmpeg error:', err);
    throw new Error('FFmpeg processing failed: ' + err.message);
  }
}

// Initialize the application
document.addEventListener('DOMContentLoaded', init);