import * as d3 from 'd3';
import { graphviz } from 'd3-graphviz';
import JSZip from 'jszip';
import { initFFmpeg } from './ffmpeg.js';
import { initResvg } from './resvg.js';
import { exampleDots } from './examples.js';

// Configuration
let config = {
  frameRate: 30,
  transitionDuration: 333,
  outputFilename: 'graph-animation',
  isRecording: false,
  videoWidth: 1280,  // Higher resolution default width
  videoHeight: 720,  // Higher resolution default height
  preserveAspectRatio: true  // Maintain aspect ratio of SVGs
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
    .logEvents(false)
    .width(1000)
    .height(800)
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
  
  // Add event listeners for video size inputs
  const videoWidthEl = document.getElementById('videoWidth');
  const videoHeightEl = document.getElementById('videoHeight');
  const preserveAspectRatioEl = document.getElementById('preserveAspectRatio');
  
  if (videoWidthEl) {
    videoWidthEl.addEventListener('change', (e) => {
      config.videoWidth = parseInt(e.target.value);
    });
  }
  
  if (videoHeightEl) {
    videoHeightEl.addEventListener('change', (e) => {
      config.videoHeight = parseInt(e.target.value);
    });
  }
  
  if (preserveAspectRatioEl) {
    preserveAspectRatioEl.addEventListener('change', (e) => {
      config.preserveAspectRatio = e.target.checked;
    });
  }
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
    console.log(`Starting conversion of ${svgFrames.length} SVG frames to PNG...`);
    const pngFrames = await convertSvgsToPngs(svgFrames);
    console.log(`Successfully converted ${pngFrames.length} frames to PNG`);
    
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
  let validFrameCount = 0;
  
  for (let i = 0; i < svgFrames.length; i++) {
    try {
      // Make sure we have a valid SVG
      if (!svgFrames[i] || !svgFrames[i].includes('<svg')) {
        console.warn(`Frame ${i} is not a valid SVG, skipping`);
        continue;
      }
      
      let svgString = svgFrames[i];
      
      // Process the SVG to ensure text is preserved
      const parser = new DOMParser();
      const serializer = new XMLSerializer();
      const doc = parser.parseFromString(svgString, 'image/svg+xml');
      
      // Add proper viewBox if missing
      const svgElement = doc.querySelector('svg');
      if (!svgElement.hasAttribute('viewBox')) {
        const width = svgElement.getAttribute('width');
        const height = svgElement.getAttribute('height');
        if (width && height) {
          svgElement.setAttribute('viewBox', `0 0 ${parseFloat(width)} ${parseFloat(height)}`);
        }
      }
      
      // Add a white background if needed
      const hasBackground = Array.from(svgElement.children).some(
        child => child.tagName === 'rect' && child.getAttribute('width') === '100%' && child.getAttribute('height') === '100%'
      );
      
      if (!hasBackground) {
        const rect = doc.createElementNS('http://www.w3.org/2000/svg', 'rect');
        rect.setAttribute('width', '100%');
        rect.setAttribute('height', '100%');
        rect.setAttribute('fill', 'white');
        svgElement.insertBefore(rect, svgElement.firstChild);
      }
      
      // Process text elements to ensure they'll be visible
    //   const textElements = doc.querySelectorAll('text');
    //   textElements.forEach(text => {
    //     // Ensure text has proper attributes
    //     if (!text.hasAttribute('font-family')) {
    //       text.setAttribute('font-family', 'Arial, sans-serif');
    //     }
    //     if (!text.hasAttribute('font-size')) {
    //       text.setAttribute('font-size', '14px');
    //     }
    //     if (!text.hasAttribute('fill') || text.getAttribute('fill') === 'none') {
    //       text.setAttribute('fill', 'black');
    //     }
        
    //     // Create a rectangle behind the text to improve visibility
    //     const bbox = text.getBBox?.() || {x: 0, y: 0, width: 0, height: 0};
    //     const padding = 1;
    //     const textBg = doc.createElementNS('http://www.w3.org/2000/svg', 'rect');
    //     textBg.setAttribute('x', String(bbox.x - padding));
    //     textBg.setAttribute('y', String(bbox.y - padding));
    //     textBg.setAttribute('width', String(bbox.width + 2*padding));
    //     textBg.setAttribute('height', String(bbox.height + 2*padding));
    //     textBg.setAttribute('fill', 'white');
    //     textBg.setAttribute('fill-opacity', '0.7');
    //     text.parentNode.insertBefore(textBg, text);
    //   });
      
      // Convert back to string
      svgString = serializer.serializeToString(doc);
      
      // Additional fix: Add XHTML namespace for better text handling
      if (!svgString.includes('xmlns:xlink="http://www.w3.org/1999/xlink"')) {
        svgString = svgString.replace('<svg', '<svg xmlns:xlink="http://www.w3.org/1999/xlink"');
      }
      
      console.log(`Converting SVG frame ${i}`);
      const pngData = await resvg.renderSvg(svgString);
      
      if (pngData && pngData.buffer) {
        pngFrames.push(pngData);
        validFrameCount++;
      } else {
        console.warn(`Frame ${i} rendered to invalid PNG, skipping`);
      }
      
      // Update progress
      const progress = Math.floor((i / svgFrames.length) * 50);
      progressEl.style.width = `${progress}%`;
    } catch (err) {
      console.error(`Error converting SVG frame ${i}:`, err);
      console.log(`Problematic SVG:`, svgFrames[i].substring(0, 100) + '...');
      // Skip this frame but continue with others
    }
  }
  
  console.log(`Successfully converted ${validFrameCount} out of ${svgFrames.length} frames`);
  
  if (pngFrames.length === 0) {
    throw new Error('Failed to convert any SVG frames to PNG');
  }
  
  return pngFrames;
}

async function convertPngsToWebM(pngFrames) {
  try {
    // Make sure we have frames to convert
    if (pngFrames.length === 0) {
      throw new Error('No PNG frames to convert');
    }
    
    console.log(`Starting WebM conversion with ${pngFrames.length} PNG frames`);
    
    // Write PNG frames to FFmpeg virtual filesystem
    for (let i = 0; i < pngFrames.length; i++) {
      const filename = `frame_${String(i).padStart(4, '0')}.png`;
      await ffmpeg.writeFile(filename, pngFrames[i]);
      
      // Update progress (50% to 75%)
      const progress = 50 + Math.floor((i / pngFrames.length) * 25);
      progressEl.style.width = `${progress}%`;
    }
    
    // Run FFmpeg to convert PNGs to WebM
    // Using simpler vp8 codec instead of vp9 for better compatibility
    console.log('Running FFmpeg command to generate WebM video...');
    await ffmpeg.runCommand([
      '-framerate', String(config.frameRate),
      '-i', 'frame_%04d.png',
      '-c:v', 'vp8',
      '-b:v', '2M',
      '-auto-alt-ref', '0',
      'output.webm'
    ]);
    
    // Read the resulting WebM file
    progressEl.style.width = '90%';
    console.log('Reading output WebM file...');
    const webmData = await ffmpeg.readFile('output.webm');
    console.log('Successfully read WebM file, size:', webmData.byteLength);
    
    // Create a Blob from the WebM data
    const webmBlob = new Blob([webmData], { type: 'video/webm' });
    
    return webmBlob;
  } catch (err) {
    console.error('FFmpeg error:', err);
    throw new Error(`FFmpeg processing failed: ${err.message || 'Unknown error'}`);
  }
}

// Initialize the application
document.addEventListener('DOMContentLoaded', init);