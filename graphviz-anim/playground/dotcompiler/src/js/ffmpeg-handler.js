// ffmpeg-handler.js
// Handles FFmpeg WASM initialization and operations

import { FFmpeg } from '@ffmpeg/ffmpeg';
import { fetchFile } from '@ffmpeg/ffmpeg/dist/esm/utils';

let ffmpeg = null;

/**
 * Initialize the FFmpeg WASM library
 */
export async function initFFmpeg() {
  try {
    ffmpeg = new FFmpeg();
    
    // Set up logging if needed
    ffmpeg.on('log', ({ message }) => {
      console.log('FFmpeg Log:', message);
    });
    
    // Load FFmpeg WASM binary
    await ffmpeg.load();
    
    console.log('FFmpeg initialized successfully');
    return true;
  } catch (error) {
    console.error('Failed to initialize FFmpeg:', error);
    return false;
  }
}

/**
 * Create a video from a sequence of image files using FFmpeg
 * @param {Array<Uint8Array>} frames - Array of image data (PNG/JPG)
 * @param {Object} options - Options for video creation
 * @returns {Promise<Uint8Array>} - WebM video data
 */
export async function createVideo(frames, options = {}) {
  if (!ffmpeg) {
    throw new Error('FFmpeg not initialized. Call initFFmpeg first.');
  }
  
  const {
    frameRate = 30,
    quality = 'medium',
    outputFilename = 'output.webm',
    onProgress
  } = options;
  
  try {
    // Get quality settings
    const crf = getQualityCRF(quality);
    
    // Write frames to virtual filesystem
    for (let i = 0; i < frames.length; i++) {
      const fileName = `frame_${String(i).padStart(4, '0')}.png`;
      await ffmpeg.writeFile(fileName, frames[i]);
      
      // Report progress for writing frames
      if (onProgress) {
        onProgress(Math.round((i / frames.length) * 50)); // First 50% for file writing
      }
    }
    
    // Create video from frames
    await ffmpeg.exec([
      '-framerate', String(frameRate),
      '-pattern_type', 'glob',
      '-i', 'frame_*.png',
      '-c:v', 'libvpx-vp9',
      '-pix_fmt', 'yuva420p',
      '-crf', String(crf),
      '-b:v', '0',
      '-deadline', 'realtime',
      '-cpu-used', '4',
      outputFilename
    ]);
    
    // Report 100% progress when encoding is complete
    if (onProgress) {
      onProgress(100);
    }
    
    // Read the output file
    const data = await ffmpeg.readFile(outputFilename);
    
    // Clean up temporary files
    for (let i = 0; i < frames.length; i++) {
      const fileName = `frame_${String(i).padStart(4, '0')}.png`;
      await ffmpeg.deleteFile(fileName);
    }
    await ffmpeg.deleteFile(outputFilename);
    
    return data;
  } catch (error) {
    console.error('Error creating video with FFmpeg:', error);
    throw error;
  }
}

/**
 * Get CRF (Constant Rate Factor) value for the specified quality
 */
function getQualityCRF(quality) {
  switch (quality) {
    case 'low': return 35;
    case 'high': return 20;
    case 'medium':
    default: return 28;
  }
}

/**
 * Get the FFmpeg instance
 * @returns {Object|null} - The FFmpeg instance or null if not initialized
 */
export function getFFmpegInstance() {
  return ffmpeg;
}