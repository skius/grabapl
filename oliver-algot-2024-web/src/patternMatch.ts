/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {PatternMatches} from 'src/DemoSemanticsInterpreter';
import {Operation} from 'src/Operation';
import {PatternId} from 'src/DemoSemantics';
import {GraphNode} from 'src/GraphAPI';

function pivotAt<T>(arr: T[], val: T): [T[], T[]] {
  const indexToSplit = arr.indexOf(val);
  return [arr.slice(0, indexToSplit), arr.slice(indexToSplit + 1)];
}

export default function patternMatch<Node extends GraphNode<Node>>(
  operation: Operation,
  inputs: Node[]
) {
  const matches: PatternMatches<Node> = {};
  Object.keys(operation.patterns).forEach(pattern => (matches[pattern] = null));

  const visited = new Set<Node>();

  function walk(
    pattern: PatternId,
    node: Node,
    sourcePattern: PatternId | null,
    source: Node | null,
    sourceForward: boolean
  ) {
    if ((source !== null) !== (sourcePattern !== null))
      throw 'Invalid arguments. Either both source pattern and node must be provided or neither.';

    if (visited.has(node)) return;
    visited.add(node);

    const sourcePresent = source !== null && sourcePattern !== null;

    matches[pattern] = node;

    function matchSide(
      patterns: PatternId[],
      neighbors: Node[],
      forward: boolean
    ) {
      patterns.forEach((neighborPattern, index) => {
        if (neighborPattern === sourcePattern)
          throw 'Invariant violated. We should have pivoted and pivoting should have removed this pattern!';

        if (neighbors.length <= index) return;
        walk(neighborPattern, neighbors[index], pattern, node, forward);
      });
    }

    function match(patterns: PatternId[], neighbors: Node[], forward: boolean) {
      if (sourcePresent && forward === !sourceForward) {
        const [patternLeft, patternRight] = pivotAt(patterns, sourcePattern);
        const [concreteLeft, concreteRight] = pivotAt(neighbors, source);
        matchSide(patternLeft, concreteLeft, forward);
        matchSide(patternRight, concreteRight, forward);
      } else {
        matchSide(patterns, neighbors, forward);
      }
    }

    match(operation.patterns[pattern].outgoing, node.neighbors, true);
    match(
      operation.patterns[pattern].incoming,
      node.nodesWithIncomingEdges,
      false
    );
  }

  operation.inputs.forEach((input, i) =>
    walk(input, inputs[i], null, null, true)
  );

  Object.values(operation.patterns).forEach(pattern => {
    if (pattern.required && matches[pattern.id] === null)
      throw `Pattern ${pattern.name} is required but not matched.`;
  });

  return matches;
}
