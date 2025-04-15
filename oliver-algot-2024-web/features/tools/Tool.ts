/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {PatternTool} from 'src/Patterns';
import {Operation, OperationId} from 'src/Operation';
import {AbstractNodeDescriptor} from 'src/DemoSemantics';
import {ConcreteNodeId} from 'src/ConcreteGraph';
import {keyForAbstractNode} from 'src/AbstractNodeUtils';
import {ConcreteValue, hashValue} from 'src/ConcreteValue';
import {EditorTool} from 'features/editor/OperationEditor';

export interface NodeArgumentArgument {
  operation: OperationId;
  action: number;
  argument: number;
}

export interface LiteralArgument {
  value: ConcreteValue;
}

export interface NodeIdArgument {
  nodeId: ConcreteNodeId;
}

export interface AbstractNodeArgument {
  abstractNode: AbstractNodeDescriptor;
}

export type Tool = Operation | PatternTool | EditorTool;
export type ToolId = Tool['id'];
export type ToolArgument =
  | NodeArgumentArgument
  | LiteralArgument
  | NodeIdArgument
  | AbstractNodeArgument;
export type ConcreteArgument = LiteralArgument | NodeIdArgument;

export function isNodeArgumentArgument(
  arg: ToolArgument
): arg is NodeArgumentArgument {
  return 'operation' in arg;
}

export function isLiteralArgument(arg: ToolArgument): arg is LiteralArgument {
  return 'value' in arg;
}

export function isNodeIdArgument(arg: ToolArgument): arg is NodeIdArgument {
  return 'nodeId' in arg;
}

export function isAbstractNodeArgument(
  arg: ToolArgument
): arg is AbstractNodeArgument {
  return 'abstractNode' in arg;
}

export function getToolInputName(tool: Tool, i: number) {
  return tool.type === 'Operation'
    ? tool.patterns[tool.inputs[i]].name
    : tool.inputs[i].name;
}

export function toolArgumentKey(arg: ToolArgument) {
  if (isNodeArgumentArgument(arg))
    return `${arg.operation}@${arg.action}@${arg.argument}`;
  if (isLiteralArgument(arg))
    return arg.value.type + '@' + hashValue(arg.value);
  if (isNodeIdArgument(arg)) return arg.nodeId;
  return keyForAbstractNode(arg.abstractNode);
}
