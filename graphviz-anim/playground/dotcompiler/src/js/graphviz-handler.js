// graphviz-handler.js
// Handles d3-graphviz initialization and operations

export function initGraphviz() {
  // Initialize d3-graphviz
  const graphviz = d3.select("#graph").graphviz()
    .logEvents(false)
    .fade(false);  // Can be enabled for smoother transitions
  
  let graphvizInitialized = false;
  let currentAnimationPromise = Promise.resolve();
  let isCapturing = false;
  let shouldStopCapturing = false;
  
  // Set up onload event for d3-graphviz
  graphviz.on("initEnd", () => {
    graphvizInitialized = true;
    console.log("Graphviz initialized");
  });
  
  /**
   * Renders a single DOT graph
   */
  function renderDot(dotCode) {
    if (!graphvizInitialized) {
      console.warn("Graphviz not yet initialized, trying anyway...");
    }
    
    try {
      graphviz.renderDot(dotCode);
      return true;
    } catch (error) {
      console.error("Error rendering DOT:", error);
      return false;
    }
  }
  
  /**
   * Animates through a sequence of DOT graphs
   */
  function animateGraph(dots, transitionDuration = 1500) {
    if (!dots || !dots.length) return;
    
    // Cancel any previous animation
    currentAnimationPromise = new Promise(async (resolve) => {
      // Configure transition
      graphviz
        .transition(function() {
          return d3.transition("main")
            .ease(d3.easeLinear)
            .duration(transitionDuration);
        });
      
      // Render initial graph if not already rendered
      if (!d3.select("#graph svg").node()) {
        graphviz.renderDot(dots[0].join('\n'));
        await new Promise(r => setTimeout(r, 100)); // Small delay to ensure rendering
      }
      
      // Animate through the sequence
      for (let i = 0; i < dots.length; i++) {
        const dot = dots[i].join('\n');
        try {
          await new Promise(r => {
            graphviz
              .renderDot(dot)
              .on("end", r);
          });
        } catch (error) {
          console.error(`Error rendering dot #${i}:`, error);
        }
      }
      
      resolve();
    });
    
    return currentAnimationPromise;
  }
  
  /**
   * Captures frames during DOT animation
   */
  async function captureFrames(dots, transitionDuration = 1500, frameRate = 30, onProgress) {
    if (!dots || !dots.length) return [];
    
    const frameInterval = 1000 / frameRate;
    const framesPerTransition = Math.ceil((transitionDuration + 500) / frameInterval); // Add buffer
    const totalExpectedFrames = dots.length * framesPerTransition;
    let capturedFrames = [];
    
    isCapturing = true;
    shouldStopCapturing = false;
    
    // Set up d3-graphviz transition
    graphviz
      .transition(function() {
        return d3.transition("main")
          .ease(d3.easeLinear)
          .duration(transitionDuration);
      });
    
    // Render initial graph if not already rendered
    if (!d3.select("#graph svg").node()) {
      graphviz.renderDot(dots[0].join('\n'));
      await new Promise(r => setTimeout(r, 100)); // Small delay to ensure rendering
    }
    
    // First capture initial state
    capturedFrames.push(getSvgString());
    
    // Then animate and capture frames
    for (let dotIndex = 0; dotIndex < dots.length; dotIndex++) {
      if (shouldStopCapturing) break;
      
      const dot = dots[dotIndex].join('\n');
      
      // Start the transition
      const transitionPromise = new Promise(resolve => {
        graphviz.renderDot(dot).on("end", resolve);
      });
      
      // Capture frames during transition
      const startTime = performance.now();
      let frameTime = startTime;
      let frameCount = 0;
      
      while (performance.now() - startTime < transitionDuration + 200) { // Add a buffer
        if (shouldStopCapturing) break;
        
        // Calculate how much time has passed since the last frame
        const now = performance.now();
        const elapsed = now - frameTime;
        
        if (elapsed >= frameInterval) {
          // Time to capture a new frame
          capturedFrames.push(getSvgString());
          frameCount++;
          frameTime = now;
          
          // Update progress
          const progress = Math.min(
            100, 
            Math.round((capturedFrames.length / totalExpectedFrames) * 100)
          );
          onProgress && onProgress(progress);
          
          // Throttle frame capture to avoid overwhelming the browser
          await new Promise(r => setTimeout(r, 0));
        } else {
          // Wait a bit before checking again
          await new Promise(r => setTimeout(r, 5));
        }
      }
      
      // Wait for the transition to complete before moving to the next one
      await transitionPromise;
    }
    
    isCapturing = false;
    
    return capturedFrames;
  }
  
  /**
   * Helper function to get the SVG string
   */
  function getSvgString() {
    const svg = document.querySelector("#graph svg");
    if (!svg) return '';
    return new XMLSerializer().serializeToString(svg);
  }
  
  /**
   * Stops an ongoing frame capture
   */
  function stopCapture() {
    shouldStopCapturing = true;
  }
  
  return {
    renderDot,
    animateGraph,
    captureFrames,
    stopCapture
  };
}