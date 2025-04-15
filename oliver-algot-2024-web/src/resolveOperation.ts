/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import baseOperations, {BaseOperation} from 'src/BaseOperations';
import {Tool, ToolId} from 'features/tools/Tool';
import {PatternToolId, patternTools} from 'src/Patterns';
import {Operation, OperationId} from 'src/Operation';
import {EditorToolId, editorTools} from 'features/editor/OperationEditor';

export default function resolveOperation(
  operations: Record<OperationId, Operation>,
  id: OperationId
): BaseOperation | Operation {
  return baseOperations[id] || operations[id];
}

export function resolveTool(
  operations: Record<OperationId, Operation>,
  id: ToolId
): Tool {
  return (
    baseOperations[id as OperationId] ||
    patternTools[id as PatternToolId] ||
    operations[id as OperationId] ||
    editorTools[id as EditorToolId]
  );
}

export function isEditorToolId(toolId: ToolId): toolId is EditorToolId {
  return toolId in editorTools;
}

export function isPatternToolId(toolId: ToolId): toolId is PatternToolId {
  return toolId in patternTools;
}
