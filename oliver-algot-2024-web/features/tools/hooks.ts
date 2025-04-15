/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {getToolInputName, toolArgumentKey} from 'features/tools/Tool';
import {useAppSelector} from 'src/hooks';
import {ConcreteNodeId} from 'src/ConcreteGraph';
import {resolveTool} from 'src/resolveOperation';
import {AbstractNodeKey} from 'src/DemoSemantics';
import {BuiltInTool} from 'features/tools/toolsReducer';

export type GraphToolLabeling = {
  labels: Record<ConcreteNodeId | AbstractNodeKey, string>;
  next: null | string;
};

/** Returns a map mapping NodeIds to the labels of the operation inputs. */
export function useGraphToolLabeling() {
  const inputNodes = useAppSelector(state => state.tools.selectedNodes);
  const op = useCurrentTool();

  if (!op || inputNodes.length > op.inputs.length) return null;

  const map: GraphToolLabeling = {
    next:
      inputNodes.length < op.inputs.length
        ? getToolInputName(op, inputNodes.length)
        : null,
    labels: {},
  };
  for (let i = 0; i < inputNodes.length; i++) {
    map.labels[toolArgumentKey(inputNodes[i])] = getToolInputName(op, i);
  }
  return map;
}

/**
 * Returns the tool the user has selected or null.
 */
export function useCurrentTool() {
  const selection = useAppSelector(state => state.tools.selectedTool);
  const ops = useAppSelector(state => state.editor.operations);
  return selection ? resolveTool(ops, selection) : null;
}

export function useBuiltInTool(): BuiltInTool | null {
  return useAppSelector(state =>
    state.tools.selectedTool === null ? state.tools.builtInTool : null
  );
}
