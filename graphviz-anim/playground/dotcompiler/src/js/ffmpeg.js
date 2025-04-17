import { FFmpeg } from '@ffmpeg/ffmpeg';
import { toBlobURL } from '@ffmpeg/util';

// Initialize FFmpeg instance
let ffmpegInstance;

export async function initFFmpeg() {
  if (!ffmpegInstance) {
    try {
      const ffmpeg = new FFmpeg();
      
      // Load FFmpeg core
      const baseURL = 'https://unpkg.com/@ffmpeg/core@0.12.6/dist/umd';
      await ffmpeg.load({
        coreURL: await toBlobURL(`${baseURL}/ffmpeg-core.js`, 'text/javascript'),
        wasmURL: await toBlobURL(`${baseURL}/ffmpeg-core.wasm`, 'application/wasm'),
      });
      
      // Create a wrapper for FFmpeg functions
      ffmpegInstance = {
        // Write files to FFmpeg virtual filesystem
        writeFile: async (name, data) => {
          try {
            await ffmpeg.writeFile(name, new Uint8Array(data));
          } catch (error) {
            console.error(`Error writing file ${name}:`, error);
            throw error;
          }
        },
        
        // Read files from FFmpeg virtual filesystem
        readFile: async (name) => {
          try {
            return await ffmpeg.readFile(name);
          } catch (error) {
            console.error(`Error reading file ${name}:`, error);
            throw error;
          }
        },
        
        // Run FFmpeg command
        runCommand: async (args) => {
          try {
            console.log('Running FFmpeg command:', args.join(' '));
            await ffmpeg.exec(args);
            console.log('FFmpeg command completed successfully');
          } catch (error) {
            console.error('FFmpeg command failed:', error);
            throw error;
          }
        },
        
        // Delete a file from the virtual filesystem
        deleteFile: async (name) => {
          try {
            await ffmpeg.deleteFile(name);
          } catch (error) {
            console.error(`Error deleting file ${name}:`, error);
            // Don't throw here, just log the error
          }
        }
      };
      
      console.log('FFmpeg loaded successfully');
    } catch (error) {
      console.error('Failed to load FFmpeg:', error);
      throw error;
    }
  }
  
  return ffmpegInstance;
}

// Convert PNG frames to WebM video using FFmpeg
export async function convertPngFramesToWebM(pngFrames, options = {}) {
  if (!ffmpegInstance) {
    throw new Error('FFmpeg not initialized. Call initFFmpeg first.');
  }
  
  const {
    frameRate = 30,
    quality = 'medium',
    outputFilename = 'output.webm'
  } = options;
  
  try {
    console.log(`Processing ${pngFrames.length} frames at ${frameRate} fps`);
    
    // Ensure we have frames to process
    if (pngFrames.length === 0) {
      throw new Error('No frames to convert');
    }
    
    // Write PNG frames to FFmpeg virtual filesystem
    for (let i = 0; i < pngFrames.length; i++) {
      if (!pngFrames[i] || !pngFrames[i].buffer) {
        console.warn(`Frame ${i} is invalid or empty, skipping`);
        continue;
      }
      
      const filename = `frame_${String(i).padStart(4, '0')}.png`;
      console.log(`Writing frame ${i} to virtual filesystem`);
      await ffmpegInstance.writeFile(filename, pngFrames[i]);
    }
    
    // Set quality parameters
    const bitrate = quality === 'high' ? '4M' : quality === 'medium' ? '2M' : '1M';
    const crf = quality === 'high' ? '23' : quality === 'medium' ? '30' : '35';
    
    // Use a simpler FFmpeg command for better compatibility
    console.log('Starting FFmpeg conversion...');
    await ffmpegInstance.runCommand([
      '-framerate', String(frameRate),
      '-i', 'frame_%04d.png',
      '-c:v', 'vp8',  // Using vp8 instead of vp9 for better compatibility
      '-b:v', bitrate,
      '-auto-alt-ref', '0',
      '-metadata', 'title="Graph Animation"',
      outputFilename
    ]);
    
    // Read the resulting WebM file
    console.log('Reading output file...');
    const webmData = await ffmpegInstance.readFile(outputFilename);
    
    // Clean up files from virtual filesystem
    console.log('Cleaning up temporary files...');
    for (let i = 0; i < pngFrames.length; i++) {
      const filename = `frame_${String(i).padStart(4, '0')}.png`;
      await ffmpegInstance.deleteFile(filename);
    }
    await ffmpegInstance.deleteFile(outputFilename);
    
    return webmData;
  } catch (error) {
    console.error('FFmpeg conversion error:', error);
    throw new Error(`FFmpeg processing failed: ${error.message || 'Unknown error'}`);
  }
}