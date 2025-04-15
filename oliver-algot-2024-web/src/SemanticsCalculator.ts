import {
  EditorState,
  PatternEditor,
  fromActionIdStack,
  fromActionStack,
  getCurrentEditor,
  toActionStack,
} from 'features/editor/editorReducer';
import {Operation, OperationId} from './Operation';
import {BaseOperation, isBaseOperation} from './BaseOperations';
import {keyForAbstractNode} from './AbstractNodeUtils';
import {
  reconstructApproximateGraphFrom,
  ApproximateGraphNode,
} from './ApproximateGraphAPI';
import {
  PatternId,
  AbstractNodeDescriptor,
  fromOutputKey,
  toOutputKey,
  DemoSemantics,
  ActionId,
  Action,
} from './DemoSemantics';
import {OperationResolver, approximate} from './interpreters';
import patternMatch from './patternMatch';
import resolveOperation from './resolveOperation';
import {ArraySet} from './ArraySet';

export function getCurrentOperation(
  actionStack: number[],
  operations: EditorState['operations'],
  selectedOperation: OperationId
): {
  operation: Operation | BaseOperation;
  action: number;
} {
  const res = {
    ...actionStack.slice(0, -1).reduce(
      ({operation}, idx) => {
        const op = resolveOperation(
          operations,
          operation.demoSemantics!.actions[idx].operation
        );
        return {
          operation: op,
        };
      },
      {
        operation: operations[selectedOperation],
      }
    ),
    action: actionStack.at(-1)!,
  };
  return res;
}

function matchOneStep(
  operations: Record<OperationId, Operation>,
  selectedOperation: OperationId,
  actionStack: number[],
  storedGraphs: PatternEditor['approximateGraphs'],
  current: Record<PatternId, AbstractNodeDescriptor>
): Record<PatternId, AbstractNodeDescriptor> {
  const {operation, action} = getCurrentOperation(
    actionStack,
    operations,
    selectedOperation
  );

  const currentGraph = reconstructApproximateGraphFrom(
    storedGraphs[fromActionStack(actionStack)].graph
  );

  const action2 = operation.demoSemantics!.actions[action];
  const operation2 = resolveOperation(operations, action2.operation);

  const inputs2 = action2.inputs.map(node => {
    switch (node.type) {
      case 'PatternMatch':
        return current[node.pattern];
      case 'OperationOutput': {
        const names = fromOutputKey(node.id);
        return {
          ...node,
          id: toOutputKey([
            ...actionStack.slice(0, -1),
            ...names,
          ] as typeof names),
        };
      }
      default:
        return node;
    }
  });
  const inputs = inputs2.map(node =>
    currentGraph.nodes
      .filter(
        n => keyForAbstractNode(n.abstractNode) === keyForAbstractNode(node)
      )
      .at(0)
  );

  if (!inputs.every(i => i !== undefined)) return {};

  const patternMatches = patternMatch(
    operation2,
    inputs as ApproximateGraphNode[]
  );

  const res: Record<PatternId, AbstractNodeDescriptor> = {};
  Object.entries(patternMatches).forEach(([p, e]) => {
    if (e) {
      res[p] = e.abstractNode;
    }
  });

  return res;
}

export class ApproximationError extends Error {
  constructor(public error: string) {
    super(error);
  }
}

function approximateAllPathActions(
  operation: Operation,
  operations: EditorState['operations'],
  patternEditor: PatternEditor
) {
  const approxResult = approximate(
    operation,
    id => resolveOperation(operations, id),
    patternEditor.exampleValues
  );

  if ('error' in approxResult) {
    throw new ApproximationError(approxResult.error);
  } else {
    return approxResult.graphData;
  }
}

export function forwardInEditor(state: EditorState) {
  function isExecutionStep(paths: string[], start: number): boolean {
    const from = paths[start];
    const to = paths[start + 1];

    return (
      from.length > to.length ||
      to.substring(0, to.lastIndexOf('.')) ===
        from.substring(0, from.lastIndexOf('.'))
    );
  }

  function findNextIndex(paths: string[], idx: number): number {
    // find the first step that performs a basic operation
    for (let i = idx; i < paths.length - 1; i++) {
      if (isExecutionStep(paths, i)) {
        return i + 1;
      }
    }
    return paths.length;
  }

  const operation = state.operations[state.selectedOperation!];

  const currentEditor = getCurrentEditor(state);

  const paths = currentEditor.openPaths;

  const idx =
    fromActionStack(currentEditor.actionStack) ===
    fromActionStack([operation.demoSemantics!.actions.length])
      ? -1
      : paths.indexOf(fromActionStack(currentEditor.actionStack));

  if (idx >= 0) {
    if (idx === paths.length - 1) {
      currentEditor.actionStack = [operation.demoSemantics!.actions.length];
    } else {
      const nextIndex = findNextIndex(paths, idx);
      if (nextIndex === paths.length) {
        currentEditor.actionStack = [operation.demoSemantics!.actions.length];
      } else {
        currentEditor.actionStack = toActionStack(paths[nextIndex]);
      }
    }
  }
}

export function expandInEditor(
  state: EditorState,
  path: number[],
  idx: number
): 'Expanded' | 'Contracted' | 'NotExandable' {
  const fullPath = fromActionStack([...path, idx]);
  const fullIdPath = fromActionIdStack(
    getActionIdStack(
      [...path, idx],
      state.operations[state.selectedOperation!],
      id => resolveOperation(state.operations, id)
    )
  );

  const currentEditor = getCurrentEditor(state);

  const {operation, action} = getCurrentOperation(
    [...path, idx],
    state.operations,
    state.selectedOperation!
  );

  if (operation.demoSemantics!.actions.length <= idx) {
    return 'NotExandable';
  }

  if (
    isBaseOperation(
      resolveOperation(
        state.operations,
        operation.demoSemantics!.actions[action].operation
      )
    )
  ) {
    return 'NotExandable';
  }

  if (ArraySet.has(currentEditor.expandedActions, fullIdPath)) {
    const pathStr = fromActionStack([...path, idx]);
    const actionStack = currentEditor.actionStack;
    while (
      fromActionStack(actionStack).startsWith(pathStr) &&
      actionStack.length > path.length + 1
    ) {
      actionStack.pop();
    }
    ArraySet.remove(currentEditor.expandedActions, fullIdPath);
    updateApproximations(state);
    return 'Contracted';
  }

  const {queryResults} = currentEditor.approximateGraphs[fullPath].graph;

  // this check can technically be omitted.
  // the check is already done in SemanticsList.tsx
  if (Object.entries(queryResults).every(([, b]) => b)) {
    ArraySet.add(currentEditor.expandedActions, fullIdPath);
    updateApproximations(state);
  }

  return 'Expanded';
}

export function deleteSpecificAction(
  state: EditorState,
  operationId: OperationId,
  action: number
) {
  const operation = state.operations[operationId];
  const currentEditor = getCurrentEditor(state);

  const actionToDelete = operation.demoSemantics!.actions[action];

  if (
    currentEditor.actionStack.length > 1 &&
    currentEditor.actionStack[0] === action
  ) {
    currentEditor.actionStack = [action];
  } else {
    const callStack = getCallStack(currentEditor.actionStack, operation, id =>
      resolveOperation(state.operations, id)
    );

    for (let i = 0; i < callStack.length; i++) {
      if (callStack[i] === operation.id) {
        if (currentEditor.actionStack[i] === action) {
          currentEditor.actionStack = currentEditor.actionStack.slice(0, i + 1);
          break;
        } else if (currentEditor.actionStack[i] > action) {
          currentEditor.actionStack[i]--;
        }
      }
    }
  }

  const dg = calculateOutputDependencyGraph(operation.demoSemantics!);
  Object.entries(dg).forEach(([a_id, deps]) => {
    deps.forEach(({id, value}) => {
      if (id === actionToDelete.id) {
        const action = operation.demoSemantics!.actions.find(
          a => a.id === a_id
        );
        if (action) {
          action.inputs[value] = {type: 'Undefined'};
        }
      }
    });
  });

  operation.demoSemantics?.actions.splice(action, 1);
}

function sortPaths(paths: string[]) {
  const stacks = paths.map(toActionStack);
  stacks.sort((a, b) => {
    for (let i = 0; i < Math.min(a.length, b.length); i++) {
      if (a[i] !== b[i]) {
        return a[i] - b[i];
      }
    }
    return a.length - b.length;
  });
  return stacks.map(fromActionStack);
}

export function updateApproximations(state: EditorState): boolean {
  const operation = state.selectedOperation;
  if (!operation) return false;

  const currentEditor = getCurrentEditor(state);

  let updates: ReturnType<typeof approximateAllPathActions> = {};
  try {
    updates = approximateAllPathActions(
      state.operations[operation],
      state.operations,
      currentEditor
    );
  } catch (e) {
    if (e instanceof ApproximationError) {
      state.errorString = e.message;
      return false;
    }
  }

  const patternMatches: PatternEditor['patternMatches'] = {};
  const naivePaths = calculateOpenPaths(
    updates,
    currentEditor.expandedActions,
    id => resolveOperation(state.operations, id),
    state.operations[operation]
  ).filter(p => updates[p].nextStep.nextStep !== 'Noinput');

  currentEditor.openPaths = naivePaths;
  currentEditor.reachablePaths = sortPaths(Object.keys(updates));

  state.operations[operation].demoSemantics!.actions.forEach((_, i) => {
    patternMatches[fromActionStack([i])] = {};
    Object.keys(state.operations[operation].patterns).forEach(pattern => {
      patternMatches[fromActionStack([i])][pattern] = {
        type: 'PatternMatch',
        pattern,
      };
    });
  });

  for (let i = 0; i < naivePaths.length - 1; i++) {
    if (naivePaths[i + 1].startsWith(naivePaths[i])) {
      patternMatches[naivePaths[i + 1]] = matchOneStep(
        state.operations,
        operation,
        toActionStack(naivePaths[i]),
        updates,
        patternMatches[naivePaths[i]]
      );
      for (
        let j = i + 2;
        j < naivePaths.length && naivePaths[j].startsWith(naivePaths[i]);
        j++
      ) {
        patternMatches[naivePaths[j]] = patternMatches[naivePaths[i + 1]];
      }
    }
  }

  currentEditor.patternMatches = patternMatches;
  currentEditor.approximateGraphs = {};
  // currentEditor.openPaths = currentEditor.openPaths.filter(
  //   path => updates[path].nextStep.nextStep !== 'Noinput'
  // );
  if (currentEditor.openPaths.length === 0)
    currentEditor.openPaths.push(fromActionStack([0]));

  currentEditor.openPaths.forEach(path => {
    currentEditor.approximateGraphs[path] = updates[path];
  });

  return true;
}

export function updateExpandedActions(
  expandedActions: string[],
  approximateGraphs: PatternEditor['approximateGraphs'],
  resolveOperation: OperationResolver,
  actions: Action[],
  path: number[] = [],
  actionIdPath: ActionId[] = []
) {
  actions.forEach((a, i) => {
    const newPath = fromActionStack([...path, i]);
    const newIdPath = fromActionIdStack([...actionIdPath, a.id]);
    if (!(newPath in approximateGraphs)) {
      ArraySet.remove(expandedActions, newIdPath);
    } else {
      const queryResults = Object.values(
        approximateGraphs[newPath].graph.queryResults
      );
      if (queryResults.includes(null)) {
        ArraySet.remove(expandedActions, newIdPath);
      } else if (queryResults.includes(false)) {
        ArraySet.add(expandedActions, newIdPath);
      } else if (ArraySet.has(expandedActions, newIdPath)) {
        const operation = resolveOperation(a.operation);
        if (operation.demoSemantics) {
          updateExpandedActions(
            expandedActions,
            approximateGraphs,
            resolveOperation,
            operation.demoSemantics.actions,
            [...path, i],
            [...actionIdPath, a.id]
          );
        }
      }
    }
  });
}

function getExploredPaths(
  expandedActions: string[],
  resolveOperation: OperationResolver,
  operation: Operation,
  path: number[] = [],
  actionIdPath: ActionId[] = []
): string[] {
  if (!operation.demoSemantics) return [];

  const res: string[] = [];
  operation.demoSemantics.actions.forEach((a, i) => {
    const nextActionStack = fromActionStack([...path, i]);
    const nextActionIdStack = fromActionIdStack([...actionIdPath, a.id]);
    res.push(nextActionStack);
    if (ArraySet.has(expandedActions, nextActionIdStack)) {
      res.push(
        ...getExploredPaths(
          expandedActions,
          resolveOperation,
          resolveOperation(a.operation),
          [...path, i],
          [...actionIdPath, a.id]
        )
      );
    }
  });

  return res;
}

export function calculateOpenPaths(
  approximateGraphs: PatternEditor['approximateGraphs'],
  expandedActions: string[],
  resolveOperation: OperationResolver,
  operation: Operation
) {
  const approximatedGraphs = Object.keys(approximateGraphs).sort((a, b) => {
    const aSplit = a.split('.');
    const bSplit = b.split('.');
    for (let i = 0; i < Math.min(aSplit.length, bSplit.length); i++) {
      if (aSplit[i] !== bSplit[i])
        return Number.parseInt(aSplit[i]) - Number.parseInt(bSplit[i]);
    }
    return 0;
  });
  const exploredGraphs = getExploredPaths(
    expandedActions,
    resolveOperation,
    operation
  );

  const res = approximatedGraphs.filter(
    x =>
      exploredGraphs.includes(x) ||
      x === fromActionStack([operation.demoSemantics!.actions.length])
  );
  return res;
}

export function newActionId(semantics: DemoSemantics): ActionId {
  const ids = semantics.actions.map(a => a.id).sort();
  for (let i = 0; ; i++) {
    const i_str = i.toString();
    if (!ids.includes(i_str)) return i_str;
  }
}

type DependencyGraph<T extends string | number | symbol, U> = Record<
  T,
  {id: T; value: U}[]
>;

// res[i] = [{id, value}]: action i depends on output generated by action id, input is index into input
export function calculateOutputDependencyGraph(
  semantics: DemoSemantics
): DependencyGraph<ActionId, number> {
  const res: DependencyGraph<ActionId, number> = {};
  semantics.actions.forEach(a => (res[a.id] = []));

  semantics.actions.forEach(a => {
    a.inputs.forEach((input, j) => {
      if (input.type === 'OperationOutput') {
        const split = fromOutputKey(input.id);
        res[a.id].push({id: split[0] as ActionId, value: j});
      }
    });
  });

  return findTransitiveClosure(res);
}

function findTransitiveClosure(graph: DependencyGraph<ActionId, number>) {
  const res: typeof graph = {};
  Object.keys(graph).forEach(k => (res[k] = []));

  Object.entries(graph).forEach(([k, v]) => {
    const queue = v.slice();
    while (queue.length > 0) {
      const next = queue.pop()!;
      if (res[k].map(x => x.id).includes(next.id)) continue;
      res[k].push(next);
      queue.push(...graph[next.id]);
    }
  });

  return res;
}

export function getCallStack(
  actionStack: number[],
  operation: Operation,
  resolveOperation: OperationResolver
): OperationId[] {
  const res = [operation.id];
  let current = operation;
  actionStack.slice(0, -1).forEach(i => {
    current = resolveOperation(current.demoSemantics!.actions[i].operation);
    res.push(current.id);
  });
  return res;
}

export function getActionIdStack(
  actionStack: number[],
  operation: Operation,
  resolveOperation: OperationResolver
): ActionId[] {
  if (
    actionStack.length === 1 &&
    actionStack[0] === operation.demoSemantics!.actions.length
  ) {
    return ['-1'];
  }
  const res = [] as ActionId[];
  let current = operation;
  actionStack.forEach(i => {
    const action = current.demoSemantics!.actions[i];
    res.push(action.id);
    current = resolveOperation(action.operation);
  });
  return res;
}

export function getLastPosition(state: EditorState) {
  return [
    state.operations[state.selectedOperation!].demoSemantics!.actions.length,
  ];
}
