// resvg-handler.js
// Handles SVG rendering using resvg-wasm

import { initWasm, Resvg } from '@resvg/resvg-wasm';

// Load the resvg-wasm core
let resvgModule;

/**
 * Initialize the resvg-wasm library
 */
export async function initResvg() {
  if (!resvgModule) {
    try {
      // Note: In production, you'd want to provide the correct path to the wasm file
      await initWasm(fetch('/node_modules/@resvg/resvg-wasm/index_bg.wasm'));
      resvgModule = {
        renderSvg: async (svgString) => {
          try {
            const resvg = new Resvg(svgString);
            const pngData = resvg.render();
            return pngData.asPng();
          } catch (error) {
            console.error('Error rendering SVG:', error);
            throw error;
          }
        }
      };
    } catch (error) {
      console.error('Failed to initialize resvg-wasm:', error);
      throw error;
    }
  }
  
  return resvgModule;
}

/**
 * Render an SVG string to PNG
 * @param {string} svgString - The SVG content to render
 * @param {Object} options - Rendering options
 * @returns {Uint8Array} - The PNG image data as a Uint8Array
 */
export function renderSvgToPng(svgString, options = {}) {
  if (!resvgModule) {
    throw new Error('Resvg not initialized. Call initResvg first.');
  }
  
  const { width, height, dpi = 96 } = options;
  
  try {
    const svgData = resvgModule.renderSvg(svgString, {
      background: 'white',
      fitTo: {
        mode: 'original',
      },
      font: {
        loadSystemFonts: true,
      },
      dpi,
      // Set width and height if provided
      ...(width && { width }),
      ...(height && { height }),
    });
    
    return svgData.asPng();
  } catch (error) {
    console.error('Error rendering SVG to PNG:', error);
    throw error;
  }
}

/**
 * Get the Resvg instance
 * @returns {Object|null} - The Resvg instance or null if not initialized
 */
export function getResvgInstance() {
  return resvgModule;
}