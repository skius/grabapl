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