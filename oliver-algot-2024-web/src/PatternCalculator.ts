import {OperationEditor, PatternEditor} from 'features/editor/editorReducer';
import {Pattern, PatternId} from './DemoSemantics';
import {Operation} from './Operation';

export function getUndirectedPatternTree(
  inputs: PatternId[],
  patterns: Record<PatternId, Pattern>
) {
  const graph: Record<PatternId, {dist: number; next: Set<PatternId>}> = {};

  const workQueue = inputs.map(pattern => ({pattern, dist: 0}));

  while (workQueue.length > 0) {
    const {pattern, dist} = workQueue.shift()!;
    if (pattern in graph) continue;

    const nextSet: Set<PatternId> = new Set();
    graph[pattern] = {dist, next: nextSet};

    const patternObj = patterns[pattern];
    [patternObj.incoming, patternObj.outgoing].forEach(patterns => {
      patterns.forEach(nextPattern => {
        workQueue.push({pattern: nextPattern, dist: dist + 1});
        if (!(nextPattern in graph)) {
          graph[pattern].next.add(nextPattern);
        }
      });
    });
  }

  return graph;
}

export function newNameForPatternEditor(editors: PatternEditor[]) {
  const names = new Set<string>(editors.map(e => e.name));
  for (let i = 1; ; i++) {
    if (!names.has(`Pattern #${i}`)) {
      return `Pattern #${i}`;
    }
  }
}

export function deletePattern(
  operation: Operation,
  editor: OperationEditor,
  pattern: PatternId
) {
  // have to find out all dependent nodes
  // 1. P := all pattern nodes that lie downstream from the pattern node (or the pattern node itself)
  // 2. O := all output nodes that depend on any of the nodes in P (from output dependency graph)

  // find all nodes in P
  const P = new Set<PatternId>();
  P.add(pattern);

  let connector: PatternId | undefined = undefined;

  // first have to find the first node in the path that connects pattern to a blue input node
  if (!operation.inputs.includes(pattern)) {
    const queue = operation.inputs.slice();
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (P.has(current)) continue;
      if (operation.patterns[current].outgoing.includes(pattern)) {
        connector = current;
        break;
      }
      if (operation.patterns[current].incoming.includes(pattern)) {
        connector = current;
        break;
      }
      queue.push(...operation.patterns[current].outgoing);
      queue.push(...operation.patterns[current].incoming);
    }
  }

  const pPattern = operation.patterns[pattern];

  // now find all dependent output nodes
  const queue = [...pPattern.incoming, ...pPattern.outgoing].filter(
    p => p !== connector
  );
  while (queue.length > 0) {
    const current = queue.shift()!;
    if (P.has(current)) continue;
    P.add(current);
    const {outgoing, incoming} = operation.patterns[current];
    queue.push(...outgoing);
    queue.push(...incoming);
  }

  if (operation.inputs.includes(pattern)) {
    const idx = operation.inputs.indexOf(pattern);
    operation.inputs.splice(idx, 1);
    operation.inputTypes.splice(idx, 1);
  }

  // TODO: fix lint error here
  // eslint-disable-next-line node/no-unsupported-features/es-builtins
  operation.patterns = Object.fromEntries(
    Object.entries(operation.patterns).filter(([id]) => !P.has(id))
  );

  Object.values(operation.patterns).forEach(p => {
    p.incoming = p.incoming.filter(id => !P.has(id));
    p.outgoing = p.outgoing.filter(id => !P.has(id));
  });

  operation.demoSemantics!.actions.forEach(
    a =>
      (a.inputs = a.inputs.map(inp => {
        if (inp.type === 'PatternMatch' && P.has(inp.pattern)) {
          return {type: 'Undefined'};
        }
        return inp;
      }))
  );

  editor.patternEditors.forEach(editor =>
    P.forEach(p => delete editor.exampleValues[p])
  );

  editor.patternEditors[editor.currentEditorIndex].actionStack = [0];
}
