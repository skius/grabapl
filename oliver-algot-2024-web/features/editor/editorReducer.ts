/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {createSlice, current, Draft, PayloadAction} from '@reduxjs/toolkit';
import {Operation, OperationId} from 'src/Operation';
import {
  AbstractNodeDescriptor,
  AbstractNodeKey,
  ActionCondition,
  ActionId,
  DemoSemantics,
  fromOutputKey,
  Pattern,
  PatternId,
  QueryApplicationId,
} from 'src/DemoSemantics';
import {ANY_TYPE_ID} from 'src/ConcreteValue';
import {fetchWorkspace} from 'src/fetchWorkspace';
import {
  AbstractNodeArgument,
  isAbstractNodeArgument,
  LiteralArgument,
  NodeArgumentArgument,
} from '../tools/Tool';
import {keyForAbstractNode} from 'src/AbstractNodeUtils';
import {selectTool} from 'features/tools/toolsReducer';
import resolveOperation from 'src/resolveOperation';
import {ApproximateGraphData} from 'src/ApproximateGraphAPI';
import {mapNode} from './SemanticsList';
import {
  calculateOutputDependencyGraph,
  expandInEditor,
  forwardInEditor,
  getActionIdStack,
  getCallStack,
  getCurrentOperation,
  getLastPosition,
  newActionId,
  updateApproximations,
  updateExpandedActions,
} from 'src/SemanticsCalculator';
import {deletePattern, newNameForPatternEditor} from 'src/PatternCalculator';

export type Stage = 'Query' | 'Action' | 'Pattern';

export interface NextStepSummary {
  nextStep: 'Run' | 'QFalse' | 'Noinput' | 'UnknownInput' | 'Noop';
  expandable: boolean;
}

export interface PatternEditor {
  name: string;
  exampleValues: Record<PatternId, number | string>;

  actionStack: number[];
  expandedActions: string[];
  openPaths: string[];
  reachablePaths: string[];

  approximateGraphs: Record<
    string,
    {
      graph: ApproximateGraphData;
      nextStep: NextStepSummary;
    }
  >;

  patternMatches: Record<string, Record<PatternId, AbstractNodeDescriptor>>;
}

export interface OperationEditor {
  activeConditions: ActionCondition[];
  undoSnapshots: {
    operation: Operation;
    editor: Omit<OperationEditor, 'undoSnapshots'>;
  }[];

  // Pattern Editor
  patternEditors: PatternEditor[];
  currentEditorIndex: number;
}

export function getCurrentEditor(
  editor: OperationEditor | EditorState,
  operation?: OperationId
): PatternEditor {
  if ('patternEditors' in editor) {
    return editor.patternEditors[editor.currentEditorIndex];
  } else {
    const opEditor =
      editor.operationsEditor[operation || editor.selectedOperation!];
    return getCurrentEditor(opEditor);
  }
}

export function fromActionStack(actionStack: number[]): string {
  return actionStack.join('.');
}

export function toActionStack(str: string): number[] {
  return str.split('.').map(x => parseInt(x));
}

// technically the same functions as the ones above
// but just to highlight difference
// first for application on actionStack, second on expandedActions elements

export function fromActionIdStack(actionIdStack: ActionId[]): string {
  return actionIdStack.join('.');
}

export function toActionIdStack(str: string): ActionId[] {
  return str.split('.');
}

const letters = 'abcdefghijklmnopqrstuvwxyz';

export interface EditorState {
  nextId: number;
  name: string;
  saveTimestamp: number;
  selectedOperation: OperationId | null;
  operationsEditor: Record<OperationId, OperationEditor>;
  loading: boolean;
  tutorialOpen: boolean;
  showHidden: boolean;
  drawArrows: boolean;
  drawEdges: boolean;
  drawEdgeIndex: boolean;
  operations: Record<OperationId, Operation>;

  highlightedNodes?: Record<AbstractNodeKey, string[]>;

  selectedNodeforEdit?: {action: number; idx: number};

  errorString: string | undefined;
}

export const initialEditorState: EditorState = {
  drawArrows: true,
  nextId: 0,
  name: 'My Workspace',
  saveTimestamp: 0,
  drawEdges: true,
  drawEdgeIndex: false,
  operationsEditor: {},
  operations: {},
  selectedOperation: null,
  loading: true,
  tutorialOpen: true,
  showHidden: false,
  errorString: undefined,
};

function updateSemantics(
  state: Draft<EditorState>,
  f: (semantics: DemoSemantics, editor: OperationEditor) => void,
  undo: boolean = true
) {
  const op = state.selectedOperation;
  if (!op) return state;
  takeUndoSnapshot(state);
  const semantics = state.operations[op].demoSemantics;
  if (!semantics) return state;
  f(semantics, state.operationsEditor[op]);
  Object.values(state.operationsEditor).forEach(
    editor => (getCurrentEditor(editor).approximateGraphs = {})
  );

  if (undo) {
    if (!updateApproximations(state)) {
      undoStep(state);
    }
  } else {
    updateApproximations(state);
  }
}

function takeUndoSnapshot(state: Draft<EditorState>) {
  const op = state.selectedOperation!;
  const editor = state.operationsEditor[op];
  editor.undoSnapshots.push({
    editor: current(editor),
    operation: current(state.operations[op]),
  });
}

function makeId(state: Draft<EditorState>) {
  return (state.nextId++).toString();
}

export function findPatternName(operation: Operation) {
  const usedLetters = new Set();
  Object.values(operation.patterns).forEach(pattern =>
    usedLetters.add(pattern.name)
  );
  for (const x of letters) {
    if (!usedLetters.has(x)) {
      return x;
    }
  }
  return 'newPattern';
}

function nameForNewOperation(
  operations: EditorState['operations'],
  isQuery: boolean
) {
  const preliminaryName = isQuery ? 'My Query' : 'My Operation';
  const names = new Set(Object.values(operations).map(op => op.name));
  if (!names.has(preliminaryName)) return preliminaryName;

  for (let i = 2; ; i++) {
    const newName = `${preliminaryName} (${i})`;
    if (!names.has(newName)) return newName;
  }
}

function makeNewOutputNodeName(names: string[]): string {
  function charToNumber(c: string): number {
    return c.charCodeAt(0) - 'A'.charCodeAt(0) + 1;
  }
  function stringToNumber(s: string): number {
    let res = 0;
    for (let i = s.length - 1, base = 1; i >= 0; i--, base *= 26) {
      res += charToNumber(s[i]) * base;
    }
    return res;
  }
  function numberToString(n: number): string {
    let res = '';
    while (n > 0) {
      res = String.fromCharCode(((n - 1) % 26) + 'A'.charCodeAt(0)) + res;
      n = Math.floor((n - 1) / 26);
    }
    return res;
  }
  const numbers = names.map(stringToNumber);
  numbers.sort((a, b) => a - b);
  let i = 1;
  while (numbers.includes(i)) i++;
  return numberToString(i);
}

function undoStep(state: EditorState) {
  const op = state.selectedOperation;
  if (!op) return;
  const opEditor = state.operationsEditor[op];
  if (opEditor.undoSnapshots.length === 0) return;
  const {operation, editor} = opEditor.undoSnapshots.pop()!;
  state.operations[op] = operation;
  state.operationsEditor[op] = {
    undoSnapshots: state.operationsEditor[op].undoSnapshots,
    ...editor,
  };
}

const editorSlice = createSlice({
  name: 'dataStructure',
  initialState: initialEditorState,
  reducers: {
    createOperation: (state, action: PayloadAction<boolean>) => {
      const isQuery = action.payload;
      const id = makeId(state);
      const initialEditor: PatternEditor = {
        name: 'Pattern #1',
        actionStack: [0],
        expandedActions: [],
        approximateGraphs: {},
        openPaths: ['0'],
        reachablePaths: ['0'],
        patternMatches: {},
        exampleValues: {},
      };
      state.operations = {
        ...state.operations,
        [id]: {
          type: 'Operation',
          id,
          name: nameForNewOperation(state.operations, isQuery),
          icon: isQuery ? 'quiz' : 'auto_fix_normal',
          inputs: [],
          inputTypes: [],
          patterns: {},
          isQuery: isQuery,
          isUserDefined: true,
        },
      };
      state.operationsEditor[id] = {
        activeConditions: [],
        undoSnapshots: [],
        patternEditors: [initialEditor],
        currentEditorIndex: 0,
      };
    },
    changeOperationName: (
      state,
      action: PayloadAction<{id: OperationId; name: string}>
    ) => {
      state.operations[action.payload.id].name = action.payload.name;
    },
    changeOperationIcon: (
      state,
      action: PayloadAction<{id: OperationId; icon: string}>
    ) => {
      state.operations[action.payload.id].icon = action.payload.icon;
    },
    changeOperationDocumentation: (
      state,
      action: PayloadAction<{id: OperationId; documentation: string}>
    ) => {
      state.operations[action.payload.id].documentation =
        action.payload.documentation;
    },
    deleteOperation: (state, {payload: id}: PayloadAction<OperationId>) => {
      if (
        Object.values(state.operations).some(
          op =>
            op.id !== id &&
            op.demoSemantics!.actions.some(a => a.operation === id)
        )
      ) {
        state.errorString =
          'Cannot delete operation that is used as input in another operation';
        return;
      }
      delete state.operations[id];
      delete state.operationsEditor[id];
      if (state.selectedOperation === id) state.selectedOperation = null;
    },
    //this creates and opens a new operation/query.
    createAndOpenOperation: (
      state,
      action: PayloadAction<{
        nodes: {
          inputs: PatternId[];
          patterns: {
            [key: PatternId]: {
              name: string;
              id: PatternId;
              outgoing: PatternId[];
              incoming: PatternId[];
              value: number;
            };
          };
        };
        isQuery: boolean;
      }>
    ) => {
      const nodes = action.payload.nodes;
      const isQuery = action.payload.isQuery;
      const id = makeId(state);
      const valueIds: Record<PatternId, PatternId> = {};
      Object.keys(nodes.patterns).forEach(key => {
        valueIds[key] = makeId(state);
      });
      const patterns: Record<PatternId, Pattern> = {};
      for (let i = 0; i < Object.keys(nodes.patterns).length; i++) {
        const key = Object.keys(nodes.patterns)[i];
        patterns[valueIds[key]] = {
          name: letters[i],
          id: valueIds[key],
          outgoing: nodes.patterns[key].outgoing.map(
            (x: PatternId) => valueIds[x]
          ),
          incoming: nodes.patterns[key].incoming.map(
            (x: PatternId) => valueIds[x]
          ),
        };
      }
      state.operations = {
        ...state.operations,
        [id]: {
          type: 'Operation',
          id,
          name: nameForNewOperation(state.operations, isQuery),
          icon: 'auto_fix_normal',
          inputs: nodes.inputs.map((x: PatternId) => valueIds[x]),
          inputTypes: Object.keys(valueIds).map(() => 'any.algot'),
          patterns: patterns,
          isQuery: isQuery,
          isUserDefined: true,
          deletable: true,
        },
      };

      const initialEditor: PatternEditor = {
        name: 'Pattern #1',
        actionStack: [0],
        expandedActions: [],
        openPaths: ['0'],
        reachablePaths: ['0'],
        approximateGraphs: {},
        patternMatches: {},
        exampleValues: {},
      };

      state.operationsEditor[id] = {
        activeConditions: [],
        undoSnapshots: [],
        patternEditors: [initialEditor],
        currentEditorIndex: 0,
      };
      Object.keys(nodes.patterns).forEach(key => {
        state.operationsEditor[id].patternEditors[0].exampleValues![
          valueIds[key]
        ] = nodes.patterns[key].value;
      });
      state.selectedOperation = id;
      state.operations[id].demoSemantics ||= {
        actions: [],
        queryApplications: {},
        outputNames: {},
      };

      updateApproximations(state);
    },
    addQueryApp: (
      state,
      {
        payload: query,
      }: PayloadAction<{query: OperationId; inputs: AbstractNodeDescriptor[]}>
    ) => {
      const id = makeId(state);
      updateSemantics(state, semantics => {
        semantics.queryApplications[id] = {
          ...query,
          id,
        };
      });
    },
    setDemoOperation: (
      state,
      {payload: operation}: PayloadAction<OperationId>
    ) => {
      state.selectedOperation = operation;
      state.operations[operation].demoSemantics ||= {
        actions: [],
        queryApplications: {},
        outputNames: {},
      };
      updateApproximations(state);
    },
    finishDemo: state => {
      state.selectedOperation = null;
    },
    addCondition: (
      state,
      {payload: condition}: PayloadAction<ActionCondition>
    ) => {
      if (!state.selectedOperation) return;
      const editor = state.operationsEditor[state.selectedOperation];
      editor.activeConditions = editor.activeConditions.filter(
        c => c.queryApp !== condition.queryApp
      );
      editor.activeConditions.push(condition);
    },
    removeCondition: (
      state,
      {payload: queryApp}: PayloadAction<QueryApplicationId>
    ) => {
      if (!state.selectedOperation) return;
      const editor = state.operationsEditor[state.selectedOperation];
      editor.activeConditions = editor.activeConditions.filter(
        c => c.queryApp !== queryApp
      );
    },
    addAction: (
      state,
      action: PayloadAction<{
        operation: OperationId;
        inputs: AbstractNodeDescriptor[];
      }>
    ) => {
      const currentEditor = getCurrentEditor(state);
      const actionStack = currentEditor.actionStack;
      if (actionStack.length !== 1) return;

      const actionOperation = resolveOperation(
        state.operations,
        action.payload.operation
      );
      const hasBaseOperationOutput =
        'hasOutput' in actionOperation && actionOperation.hasOutput;

      updateSemantics(
        state,
        (semantics, editor) => {
          const nextActionId = newActionId(semantics);
          semantics.actions.splice(currentEditor.actionStack[0], 0, {
            ...action.payload,
            conditions: editor.activeConditions,
            id: nextActionId,
          });

          if (hasBaseOperationOutput) {
            semantics.outputNames[nextActionId] = makeNewOutputNodeName(
              Object.values(semantics.outputNames)
            );
          }

          currentEditor.actionStack[0]++;
        },
        true
      );
    },
    deleteQueryApp: (state, action: PayloadAction<QueryApplicationId>) =>
      updateSemantics(state, semantics => {
        delete semantics.queryApplications[action.payload];
        Object.values(state.operations).forEach(op =>
          op.demoSemantics!.actions.forEach(
            a =>
              (a.conditions = a.conditions.filter(
                c => c.queryApp !== action.payload
              ))
          )
        );
      }),
    changeName(state, action: PayloadAction<string>) {
      state.name = action.payload;
    },
    addInput(state, {payload: operation}: PayloadAction<OperationId>) {
      takeUndoSnapshot(state);
      const name = findPatternName(state.operations[operation]);
      const id = makeId(state);

      state.operations[operation].patterns[id] = {
        name,
        id,
        outgoing: [],
        incoming: [],
      };
      state.operations[operation].inputs.push(id);
      state.operations[operation].inputTypes.push(ANY_TYPE_ID);

      state.operationsEditor[operation].patternEditors.forEach(editor => {
        editor.exampleValues[id] = 0;
      });

      Object.values(state.operations).forEach(op =>
        op.demoSemantics!.actions.forEach(a => {
          if (a.operation === state.selectedOperation) {
            a.inputs.push({type: 'Undefined'});
          }
        })
      );

      updateApproximations(state);
    },
    addPatternChild(
      state,
      {
        payload: {operation, parent, prepend},
      }: PayloadAction<{
        operation: OperationId;
        parent: PatternId;
        prepend: boolean;
      }>
    ) {
      takeUndoSnapshot(state);
      const name = findPatternName(state.operations[operation]);
      const id = makeId(state);

      state.operations[operation].patterns[id] = {
        name,
        id,
        incoming: [parent],
        outgoing: [],
      };

      const out = state.operations[operation].patterns[parent].outgoing;
      if (prepend) out.unshift(id);
      else out.push(id);
    },
    addPatternParent(
      state,
      {
        payload: {operation, child, prepend},
      }: PayloadAction<{
        operation: OperationId;
        child: PatternId;
        prepend: boolean;
      }>
    ) {
      takeUndoSnapshot(state);
      const name = findPatternName(state.operations[operation]);
      const id = makeId(state);

      state.operations[operation].patterns[id] = {
        name,
        id,
        incoming: [],
        outgoing: [child],
      };

      const incoming = state.operations[operation].patterns[child].incoming;
      if (prepend) incoming.unshift(id);
      else incoming.push(id);
    },
    undo(state) {
      undoStep(state);
    },
    setTutorialOpen(state, {payload}: PayloadAction<boolean>) {
      state.tutorialOpen = payload;
    },
    toggleShowHidden(state) {
      state.showHidden = !state.showHidden;
    },
    toggleRequired(
      state,
      {
        payload,
      }: PayloadAction<{
        operation: OperationId;
        pattern: PatternId;
      }>
    ) {
      takeUndoSnapshot(state);
      state.operations[payload.operation].patterns[payload.pattern].required =
        !state.operations[payload.operation].patterns[payload.pattern].required;
    },
    makeInput(
      state,
      {
        payload: {operation, pattern},
      }: PayloadAction<{
        operation: OperationId;
        pattern: PatternId;
      }>
    ) {
      takeUndoSnapshot(state);
      const op = state.operations[operation];
      const visited = new Set<PatternId>();
      const queue = [pattern];
      while (queue.length > 0) {
        const current = queue.shift()!;
        const p = op.patterns[current];
        if (op.inputs.includes(p.id)) {
          const idx = op.inputs.indexOf(p.id);
          op.inputs.splice(idx, 1);
          op.inputTypes.splice(idx, 1);
        }
        p.outgoing.forEach(out => {
          if (visited.has(out)) return;
          queue.push(out);
          visited.add(out);
        });
        p.incoming.forEach(incoming => {
          if (visited.has(incoming)) return;
          queue.push(incoming);
          visited.add(incoming);
        });
      }
      op.inputs.push(pattern);
      op.inputTypes.push(ANY_TYPE_ID);
    },
    changePatternNodeName(
      state,
      {
        payload: {operation, pattern, name},
      }: PayloadAction<{
        operation: OperationId;
        pattern: PatternId;
        name: string;
      }>
    ) {
      takeUndoSnapshot(state);
      state.operations[operation].patterns[pattern].name = name;
    },
    highlightArgs: (
      state,
      {payload: args}: PayloadAction<Record<AbstractNodeKey, string[]>>
    ) => {
      state.highlightedNodes = args;
    },
    unhighlightArgs: state => {
      state.highlightedNodes = undefined;
    },
    editorSelectTableNodeForEdit(
      state,
      payload: PayloadAction<{action: number; idx: number}>
    ) {
      state.selectedNodeforEdit = payload.payload;
    },
    editorApplyTableNodeEdit(
      state,
      payload: PayloadAction<{
        argidx: NodeArgumentArgument;
        argnode: AbstractNodeArgument | LiteralArgument;
      }>
    ) {
      takeUndoSnapshot(state);
      const {argidx, argnode} = payload.payload;
      const {operation, action, argument} = argidx;
      state.operations[operation].demoSemantics!.actions[action].inputs[
        argument
      ] = isAbstractNodeArgument(argnode)
        ? argnode.abstractNode
        : {type: 'Literal', value: argnode.value};
    },
    editorDeleteSpecificAction(
      state,
      {
        payload: {operationId, action},
      }: PayloadAction<{
        operationId: OperationId;
        action: number;
      }>
    ) {
      takeUndoSnapshot(state);

      const operation = state.operations[operationId];
      const currentEditor = getCurrentEditor(state);

      const actionToDelete = operation.demoSemantics!.actions[action];

      if (
        currentEditor.actionStack.length > 1 &&
        currentEditor.actionStack[0] === action
      ) {
        currentEditor.actionStack = [action];
      } else {
        const callStack = getCallStack(
          currentEditor.actionStack,
          operation,
          id => resolveOperation(state.operations, id)
        );

        for (let i = 0; i < callStack.length; i++) {
          if (callStack[i] === operation.id) {
            if (currentEditor.actionStack[i] === action) {
              currentEditor.actionStack = currentEditor.actionStack.slice(
                0,
                i + 1
              );
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

      if (!updateApproximations(state)) {
        undoStep(state);
      }

      while (
        !currentEditor.openPaths.includes(
          fromActionStack(currentEditor.actionStack)
        )
      ) {
        if (currentEditor.actionStack.length > 1) {
          currentEditor.actionStack.pop();
        } else {
          currentEditor.actionStack[0]--;
        }
      }
    },
    editorReorderActions(
      state,
      {
        payload: {operation, sourceAction, targetAction},
      }: PayloadAction<{
        operation: OperationId;
        sourceAction: number;
        targetAction: number;
      }>
    ) {
      takeUndoSnapshot(state);
      const ds = state.operations[operation].demoSemantics!;

      const dg = calculateOutputDependencyGraph(ds);

      const source = ds.actions[sourceAction];
      ds.actions.splice(sourceAction, 1);
      ds.actions.splice(targetAction, 0, source);

      const idToIdx: Record<ActionId, number> = {};
      ds.actions.forEach((a, i) => (idToIdx[a.id] = i));

      Object.entries(dg).forEach(([a_id, deps]) => {
        const a_idx = idToIdx[a_id];
        deps.forEach(({id, value}) => {
          const dep_idx = idToIdx[id];
          if (dep_idx >= a_idx) {
            ds.actions[a_idx].inputs[value] = {type: 'Undefined'};
          }
        });
      });

      const currentEditor = getCurrentEditor(state, operation);

      if (!updateApproximations(state)) {
        undoStep(state);
      }

      while (
        !currentEditor.openPaths.includes(
          fromActionStack(currentEditor.actionStack)
        )
      ) {
        if (currentEditor.actionStack.length > 1) {
          currentEditor.actionStack.pop();
        } else {
          currentEditor.actionStack[0]--;
        }
      }
    },
    editorForward(state) {
      forwardInEditor(state);
    },
    editorBackward(state) {
      const currentEditor = getCurrentEditor(state);

      const paths = currentEditor.openPaths;
      let idx = paths.indexOf(fromActionStack(currentEditor.actionStack));
      if (idx > 0) idx -= 1;
      while (idx > 0 && paths[idx].startsWith(paths[idx - 1])) idx -= 1;

      currentEditor.actionStack = toActionStack(paths[idx]);
    },
    editorInto(state) {
      const currentEditor = getCurrentEditor(state);

      const currentActionStack = fromActionStack(currentEditor.actionStack);
      if (
        !currentEditor.openPaths[
          currentEditor.openPaths.indexOf(currentActionStack) + 1
        ].startsWith(currentActionStack)
      ) {
        expandInEditor(
          state,
          currentEditor.actionStack.slice(0, -1),
          currentEditor.actionStack[currentEditor.actionStack.length - 1]
        );
      }

      forwardInEditor(state);
    },
    editorOut(state) {
      const currentEditor = getCurrentEditor(state);
      const paths = currentEditor.openPaths;

      const currentlyOpen = fromActionStack(
        currentEditor.actionStack.slice(0, -1)
      );
      let idx = paths.indexOf(fromActionStack(currentEditor.actionStack));
      while (idx < paths.length && paths[idx].startsWith(currentlyOpen)) idx++;
      idx = Math.min(idx, paths.length - 1);
      const nextActionStack = toActionStack(paths[idx]);
      if (!paths[idx].startsWith(currentlyOpen)) {
        expandInEditor(
          state,
          currentEditor.actionStack.slice(0, -2),
          currentEditor.actionStack[currentEditor.actionStack.length - 2]
        );
      }
      currentEditor.actionStack = nextActionStack;
    },
    editorStepToStart(state) {
      const currentEditor = getCurrentEditor(state);
      currentEditor.actionStack = [0];
    },
    editorStepTo(state, {payload}: PayloadAction<number[]>) {
      const currentEditor = getCurrentEditor(state);
      const lookedPath = payload.slice();
      while (!currentEditor.openPaths.includes(fromActionStack(lookedPath))) {
        lookedPath.pop();
      }
      const idx = currentEditor.openPaths.indexOf(fromActionStack(lookedPath));
      currentEditor.actionStack = toActionStack(
        currentEditor.openPaths[idx + 1] ||
          fromActionStack(getLastPosition(state))
      );
    },
    editorExpand(
      state,
      {payload: {path, idx}}: PayloadAction<{path: number[]; idx: number}>
    ) {
      // path is the list of action indices to get to the operation we want to work on
      // idx is the index to the action that is to be expanded

      expandInEditor(state, path, idx);
    },
    editorSetHoveredActionStack(
      state,
      {payload}: PayloadAction<number[] | undefined>
    ) {
      const currentEditor = getCurrentEditor(state);

      // editor.hoveredActionStack = payload;
      if (payload === undefined) {
        state.highlightedNodes = undefined;
        return;
      }

      const {operation, action} = getCurrentOperation(
        payload,
        state.operations,
        state.selectedOperation!
      );

      const pathIdActionStack = getActionIdStack(
        currentEditor.actionStack,
        state.operations[state.selectedOperation!],
        id => resolveOperation(state.operations, id)
      );

      state.highlightedNodes = {};
      operation.demoSemantics!.actions[action].inputs.forEach(inp => {
        const match = mapNode(
          inp,
          currentEditor.patternMatches,
          payload.slice(0, -1),
          pathIdActionStack
        );
        if (!match) return;
        const key = keyForAbstractNode(match);
        if (!state.highlightedNodes![key]) state.highlightedNodes![key] = [];
        state.highlightedNodes![key].push('a');
      });
    },
    editorChangeExampleValue(
      state,
      {
        payload: {pattern, value},
      }: PayloadAction<{pattern: PatternId; value: number}>
    ) {
      takeUndoSnapshot(state);

      const currentEditor = getCurrentEditor(state);

      const oldValue = currentEditor.exampleValues![pattern];
      currentEditor.exampleValues![pattern] = value;

      if (!updateApproximations(state)) {
        currentEditor.exampleValues![pattern] = oldValue;
      }

      updateExpandedActions(
        currentEditor.expandedActions,
        currentEditor.approximateGraphs,
        id => resolveOperation(state.operations, id),
        state.operations[state.selectedOperation!].demoSemantics!.actions
      );

      currentEditor.actionStack = [0];
    },
    resetErrorMessage(state) {
      state.errorString = undefined;
    },
    editorChangeInputNode(
      state,
      {
        payload: {operation, action, input, node},
      }: PayloadAction<{
        operation: OperationId;
        action: number;
        input: number;
        node: AbstractNodeDescriptor;
      }>
    ) {
      takeUndoSnapshot(state);

      // check that the argument was created before the action

      if (node.type === 'OperationOutput') {
        const semantics = state.operations[operation].demoSemantics!;
        const generatingAction = fromOutputKey(node.id)[0];
        const generatingActionIdx = semantics.actions.findIndex(
          a => a.id === generatingAction
        );
        if (generatingActionIdx >= action) {
          console.log('Cannot use inputs from the future');
          return;
        }
      }

      const actions = state.operations[operation].demoSemantics!.actions;
      const currentEditor = getCurrentEditor(state);

      actions[action].inputs[input] = node;

      if (!updateApproximations(state)) {
        undoStep(state);
      } else {
        updateExpandedActions(
          currentEditor.expandedActions,
          currentEditor.approximateGraphs,
          id => resolveOperation(state.operations, id),
          actions
        );
      }
    },
    editorChangeCalledOperation(
      state,
      {
        payload: {operation, action},
      }: PayloadAction<{operation: OperationId; action: number}>
    ) {
      takeUndoSnapshot(state);
      // eslint-disable-next-line prettier/prettier
      state.operations[state.selectedOperation!].demoSemantics!.actions[action].operation = operation;
      updateApproximations(state);
    },
    patternAddChild(state, {payload: parent}: PayloadAction<PatternId>) {
      const operation = state.operations[state.selectedOperation!];
      const editor = state.operationsEditor[state.selectedOperation!];

      const newPatternId = makeId(state);
      const newPattern = {
        name: findPatternName(operation),
        id: newPatternId,
        incoming: [parent],
        outgoing: [],
      };

      operation.patterns[newPatternId] = newPattern;
      operation.patterns[parent].outgoing.push(newPatternId);

      editor.patternEditors.forEach(editor => {
        // editor.actionStack = [0];
        editor.exampleValues[newPatternId] = 0;
      });

      updateApproximations(state);
    },
    patternAddParent(state, {payload: child}: PayloadAction<PatternId>) {
      const operation = state.operations[state.selectedOperation!];
      const editor = state.operationsEditor[state.selectedOperation!];

      const newPatternId = makeId(state);
      const newPattern = {
        name: findPatternName(operation),
        id: newPatternId,
        incoming: [],
        outgoing: [child],
      };

      operation.patterns[newPatternId] = newPattern;
      operation.patterns[child].incoming.push(newPatternId);

      editor.patternEditors.forEach(editor => {
        // editor.actionStack = [0];
        editor.exampleValues[newPatternId] = 0;
      });

      updateApproximations(state);
    },
    patternDeletePattern(state, {payload: pattern}: PayloadAction<PatternId>) {
      const operation = state.operations[state.selectedOperation!];
      const editor = state.operationsEditor[state.selectedOperation!];

      deletePattern(operation, editor, pattern);

      updateApproximations(state);
    },
    makeNewPattern(state) {
      const editor = state.operationsEditor[state.selectedOperation!];
      const currentEditor = JSON.parse(
        JSON.stringify(getCurrentEditor(editor))
      );
      editor.patternEditors.push({
        ...currentEditor,
        name: newNameForPatternEditor(editor.patternEditors),
        actionStack: [0],
      });
      editor.currentEditorIndex = editor.patternEditors.length - 1;
      updateApproximations(state);
    },
    patternSelectPatternEditor(state, {payload}: PayloadAction<number>) {
      const editor = state.operationsEditor[state.selectedOperation!];
      editor.currentEditorIndex = payload;
      editor.patternEditors[payload].actionStack = [0];
      updateApproximations(state);
    },
    patternChangePatternEditorName(
      state,
      {
        payload: {editor, name},
      }: PayloadAction<{editor: PatternEditor; name: string}>
    ) {
      editor.name = name;
    },
    deletePatternEditor(state, {payload: idx}: PayloadAction<number>) {
      const editor = state.operationsEditor[state.selectedOperation!];
      if (editor.patternEditors.length === 1) {
        throw 'Delete Button should be disabled';
      }
      editor.patternEditors.splice(idx, 1);
      if (editor.currentEditorIndex >= idx)
        editor.currentEditorIndex = Math.max(editor.currentEditorIndex - 1, 0);
    },
  },
  extraReducers: builder => {
    builder.addCase(fetchWorkspace.pending, state => {
      state.loading = true;
    });
    builder.addCase(fetchWorkspace.fulfilled, (state, {payload}) => {
      payload.editor.loading = false;
      return payload.editor;
    });
    builder.addCase(selectTool.fulfilled, state => {
      state.selectedNodeforEdit = undefined;
    });
  },
});

export default editorSlice.reducer;

export const {
  createOperation,
  changeOperationName,
  changeOperationIcon,
  changeOperationDocumentation,
  deleteOperation,
  addQueryApp,
  setDemoOperation,
  createAndOpenOperation,
  addCondition,
  removeCondition,
  addAction,
  finishDemo,
  deleteQueryApp,
  changeName,
  addInput,
  addPatternChild,
  addPatternParent,
  setTutorialOpen,
  undo,
  toggleRequired,
  makeInput,
  changePatternNodeName,

  highlightArgs,
  unhighlightArgs,

  editorSelectTableNodeForEdit,
  editorApplyTableNodeEdit,
  editorDeleteSpecificAction,
  editorReorderActions,

  editorForward,
  editorBackward,
  editorInto,
  editorOut,
  editorExpand,
  editorStepToStart,
  editorStepTo,
  editorSetHoveredActionStack,
  editorChangeExampleValue,
  resetErrorMessage,
  editorChangeInputNode,
  editorChangeCalledOperation,

  makeNewPattern,
  patternAddChild,
  patternAddParent,
  patternDeletePattern,
  patternSelectPatternEditor,
  patternChangePatternEditorName,
  deletePatternEditor,
} = editorSlice.actions;
