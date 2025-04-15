/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {AppDispatch, RootState} from 'src/store';
import execute, {evaluate} from 'src/interpreters';
import {performedOperation} from 'features/playground/playgroundReducer';
import {addAction, addQueryApp} from 'features/editor/editorReducer';
import {
  ConcreteArgument,
  isAbstractNodeArgument,
  isLiteralArgument,
  isNodeArgumentArgument,
  isNodeIdArgument,
  ToolArgument,
  ToolId,
} from 'features/tools/Tool';
import {isEditorToolId} from 'src/resolveOperation';
import {patternTools} from 'src/Patterns';
import {AbstractNodeDescriptor} from 'src/DemoSemantics';
import resolveOperation, {isPatternToolId} from 'src/resolveOperation';
import {OperationId} from 'src/Operation';
import {editorTools} from 'features/editor/OperationEditor';

/**
 * Takes a list of tool arguments and represents all of them as abstract nodes.
 * More specifically, this function ensures there are only abstract nodes or
 * literal values provided as arguments.
 */
function toAbstractNodes(args: ToolArgument[]): AbstractNodeDescriptor[] {
  return args.map(arg => {
    if (isNodeArgumentArgument(arg))
      throw 'Invariant violated. Argument reference node is not allowed as an abstract node.';
    if (isNodeIdArgument(arg))
      throw 'Invariant violated. Concrete node is selected in demo.';
    if (isLiteralArgument(arg)) return {type: 'Literal', value: arg.value};
    return arg.abstractNode;
  });
}

function toConcreteArgs(args: ToolArgument[]): ConcreteArgument[] {
  return args.map(a => {
    if (isNodeArgumentArgument(a))
      throw 'Invariant violated. Argument reference node is not allowed as a concrete node.';
    if (isAbstractNodeArgument(a))
      throw 'Abstract node in concrete tool application';
    return a;
  });
}

/**
 * Provided with the dispatch function and the app state, this function resolves
 * the tool and executes it. The tool will be either added to the demonstration
 * or executed in the state view, depending on which mode the user is in.
 * This function is only intended to be called by the tools reducer.
 * @attention If you just want to execute a tool, please dispatch executeTool
 * from toolsReducer.ts instead.
 */
export default function executeToolOnStore(
  dispatch: AppDispatch,
  state: RootState,
  args: ToolArgument[],
  toolId: ToolId
) {
  const graph = state.playground.graph;

  if (isEditorToolId(toolId)) {
    editorTools[toolId].perform(dispatch, toAbstractNodes(args));
    return;
  }

  if (state.editor.selectedOperation !== null) {
    executeToolForDemo(dispatch, state, toAbstractNodes(args), toolId);
    return;
  }

  const concreteArgs = toConcreteArgs(args);
  const operation = resolveOperation(
    state.editor.operations,
    toolId as OperationId
  );

  if (operation && operation.isQuery) {
    //display the query result using HeadlessUI
    const result = evaluate(
      toolId as OperationId,
      id => resolveOperation(state.editor.operations, id),
      graph,
      concreteArgs
    );
    alert(`Query result: ${result}`);
    //QueryDialog();
  } else if (isPatternToolId(toolId)) {
    alert(
      'You can only use pattern tools in demonstration view for now. A later version of Algot will improve this.'
    );
  } else {
    const newGraph = execute(
      toolId as OperationId,
      id => resolveOperation(state.editor.operations, id),
      graph,
      concreteArgs
    );
    dispatch(performedOperation({newGraph}));
  }
}

export function executeToolForDemo(
  dispatch: AppDispatch,
  state: RootState,
  abstractNodes: AbstractNodeDescriptor[],
  toolId: ToolId
) {
  const {operations, selectedOperation} = state.editor;

  const isQueryTool =
    !isPatternToolId(toolId) &&
    resolveOperation(operations, toolId as OperationId).isQuery &&
    state.tools.selectedType !== 'Operation';

  function dispatchAddAction() {
    dispatch(
      addAction({operation: toolId as OperationId, inputs: abstractNodes})
    );
  }

  function dispatchAddQueryApp() {
    dispatch(
      addQueryApp({query: toolId as OperationId, inputs: abstractNodes})
    );
  }

  if (isQueryTool) {
    dispatchAddQueryApp();
  } else if (isPatternToolId(toolId)) {
    patternTools[toolId].perform(
      operations[selectedOperation!],
      dispatch,
      abstractNodes
    );
  } else {
    dispatchAddAction();
  }
}
