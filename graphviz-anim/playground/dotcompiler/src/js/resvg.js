import { initWasm, Resvg } from '@resvg/resvg-wasm';

// Load the resvg-wasm core
let resvgModule;

export async function initResvg() {
  if (!resvgModule) {
    try {
      // Note: In production, you'd want to provide the correct path to the wasm file
      await initWasm(fetch('/node_modules/@resvg/resvg-wasm/index_bg.wasm'));
      resvgModule = {
        renderSvg: async (svgString) => {
          try {
            // Convert text elements to paths to ensure text is always visible
            // svgString = await inlineTextAsPaths(svgString);
            
            // Create Resvg instance with improved font options
            const resvg = new Resvg(svgString, {
              font: {
                loadSystemFonts: true,  // Try to load system fonts
                fontFiles: [],          // Additional font files if needed
                fontDirs: [],           // Additional font directories if needed
                defaultFontFamily: 'Times, Arial, Helvetica, sans-serif',
                defaultFontSize: 12,
              },
              imageRendering: 0,       // High quality
              shapeRendering: 1,       // Optimized for quality
              textRendering: 1,        // Optimized for quality
              fitTo: {
                mode: 'original',
              },
              dpi: 96,
              background: 'white',     // Ensure white background
            });
            
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
 * Convert SVG text elements to path elements to ensure they render correctly
 * This uses a virtual DOM to parse and manipulate the SVG
 * @param {string} svgString - The SVG content as a string
 * @returns {string} - Modified SVG with text converted to paths or embedded with inline styles
 */
async function inlineTextAsPaths(svgString) {
  try {
    // Create a DOM parser to modify the SVG
    const parser = new DOMParser();
    const serializer = new XMLSerializer();
    const svgDoc = parser.parseFromString(svgString, 'image/svg+xml');
    
    // Find all text elements
    const textElements = svgDoc.querySelectorAll('text');
    
    // Process each text element
    textElements.forEach(textEl => {
        // console.log('Processing text element:', textEl);
      // Ensure text has font attributes
      if (!textEl.hasAttribute('font-family')) {
        textEl.setAttribute('font-family', 'Arial, Helvetica, sans-serif');
      }
      
      if (!textEl.hasAttribute('font-size')) {
        textEl.setAttribute('font-size', '12px');
      }
      
      // Make sure text is visible by giving it a fill if it doesn't have one
      if (!textEl.hasAttribute('fill') || textEl.getAttribute('fill') === 'none') {
        textEl.setAttribute('fill', 'black');
      }
      
      // Ensure the text has a position
      if (!textEl.hasAttribute('x')) {
        textEl.setAttribute('x', '0');
      }
      
      if (!textEl.hasAttribute('y')) {
        textEl.setAttribute('y', '0');
      }
    });
    
    // Convert back to string
    return serializer.serializeToString(svgDoc);
  } catch (error) {
    console.error('Error processing text in SVG:', error);
    // Return original SVG if there was an error
    return svgString;
  }
}