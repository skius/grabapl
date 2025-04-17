// video-converter.js
// Handles conversion of SVG frames to WebM video using resvg and ffmpeg

import { renderSvgToPng } from './resvg-handler';
import { createVideo } from './ffmpeg-handler';

export function initVideoConverter() {
  /**
   * Convert an array of SVG frames to a WebM video
   * @param {Array<string>} svgFrames - Array of SVG strings
   * @param {number} frameRate - Frames per second for the output video
   * @param {Function} onProgress - Progress callback
   * @returns {Promise<Blob>} - WebM video as a Blob
   */
  async function convertToWebM(svgFrames, frameRate = 30, onProgress) {
    if (!svgFrames || svgFrames.length === 0) {
      throw new Error('No SVG frames provided');
    }
    
    try {
      // Step 1: Convert all SVG frames to PNG
      const pngFrames = [];
      for (let i = 0; i < svgFrames.length; i++) {
        const svg = svgFrames[i];
        const png = await renderSvgToPng(svg);
        pngFrames.push(png);
        
        // Report progress for PNG conversion
        if (onProgress) {
          onProgress(Math.round((i / svgFrames.length) * 40)); // First 40% for PNG conversion
        }
      }
      
      // Step 2: Convert PNG frames to WebM video
      const videoData = await createVideo(pngFrames, {
        frameRate,
        quality: 'medium', 
        outputFilename: 'output.webm',
        onProgress: progress => {
          if (onProgress) {
            // Scale progress to remaining 60%
            onProgress(40 + Math.round(progress * 0.6));
          }
        }
      });
      
      // Step 3: Convert Uint8Array to Blob
      return new Blob([videoData], { type: 'video/webm' });
      
    } catch (error) {
      console.error('Error converting SVGs to WebM:', error);
      throw error;
    }
  }
  
  return {
    convertToWebM
  };
}