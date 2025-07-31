I have been working on a statically typed, graph-based programming language with visualizable intermediate abstract states. It is written in Rust and compiles down nicely to WASM, see the playground below (it runs entirely in your browser).

For some background, I [made a post on a different subreddit](https://www.reddit.com/r/ProgrammingLanguages/comments/1me1k4j/grabapl_a_graphbased_programming_language_with/) detailing the language a bit more, the GitHub is [https://github.com/skius/grabapl](https://github.com/skius/grabapl) and the online playground is available at [https://skius.github.io/grabapl/playground/](https://skius.github.io/grabapl/playground/) .

Now, for the title of this post (which is only kind of clickbait!):

The language works on a single, global, mutable, directed graph with node and edge values. Every operation sees a statically typed (including shape) window of the graph as it will exist at runtime.

I have been working on some sample implementations of common graph algorithms, and thought about how to easily implement some extremely basic runtime debugging capabilities. Given that the program state is a graph, storing intermediate graphs (with some added metadata) was an obvious idea. Extending my interpreter to store a trace (at explicit, user-provided snapshot points) was super easy with Rust's help!

I then used the amazing [d3-graphviz](https://github.com/magjac/d3-graphviz) library to animate the snapshots together. When I saw the first visualization of a trace of a 'funny' back-and-forth bubble sort implementation I made, I was surprised at how not bad it looked as a general visualization/learning tool of the algorithm!

I wanted to share some visualizations specifically (but also share my language in general - please check out the other post linked above!), hence this post.

# Visualizations

I apologize for the terrible quality GIFs here. The [GitHub README](https://github.com/skius/grabapl) contains actual mp4s as well as links to the respective source codes which you can copy-paste into the online playground to see the operation trace (once you execute an operation) yourself, which manual stepping through!

A quick explanation of the graphs:

* Gray nodes are runtime-only. No operation (read: function) in the current call stack sees these in its static abstract window of the graph.
* Orange nodes are in-scope of some operation's static window in the current call stack, excluding the current operation (i.e., the one from which the active snapshot was taken). These behave specially in that they cannot be dynamically matched. [The other post](https://www.reddit.com/r/ProgrammingLanguages/comments/1me1k4j/grabapl_a_graphbased_programming_language_with/) has more details on why.
* White nodes with names are the nodes, including their names, of the currently active operation's static window.
* Text inside {} are node markers - dynamic matching queries can decide to skip nodes marked with specific markers.

Here is a regular bubble sort does the "optimal" n, n-1, n-2, ... chain inner iterations:

Here is the bubble sort mentioned above that goes back and forth (or up and down):

Here is an implementation of DFS:

And lastly, here is a pretty unwieldy to visualize implementation of BFS (it's so unwieldy because the queue stores "node references", which are nothing more than pointer nodes pointing via an edge ("attached") to the pointee node.

(Again, they are all as mp4s on GitHub! I hope the GIFs don't break completely at least...)