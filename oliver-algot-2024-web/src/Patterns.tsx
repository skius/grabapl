/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {AppDispatch} from 'src/store';
import {
  addInput,
  editorChangeExampleValue,
  patternAddChild,
  patternAddParent,
  patternDeletePattern,
} from 'features/editor/editorReducer';
import {Operation} from 'src/Operation';
import {
  AbstractNodeDescriptor,
  PatternMatchAbstractNodeDescriptor,
} from 'src/DemoSemantics';
import {ANY_TYPE_ID, NUMBER_TYPE_ID} from 'src/ConcreteValue';

export type PatternToolId = string & {readonly brand?: unique symbol};
type PatternToolType = 'Structure' | 'Values';

export type PatternTool = {
  type: 'Pattern';
  subtype: PatternToolType;
  id: PatternToolId;
  inputs: {name: string}[];
  inputTypes: string[];
  name: string;
  icon: string;
  documentation: string;
  perform: (
    operation: Operation,
    dispatch: AppDispatch,
    nodes: AbstractNodeDescriptor[]
  ) => void;
};

function withPatternNode(
  f: (
    operation: Operation,
    dispatch: AppDispatch,
    nodes: PatternMatchAbstractNodeDescriptor[]
  ) => void
) {
  return (
    operation: Operation,
    dispatch: AppDispatch,
    nodes: AbstractNodeDescriptor[]
  ) => {
    if (nodes.some(n => n.type !== 'PatternMatch')) return;
    f(operation, dispatch, nodes as PatternMatchAbstractNodeDescriptor[]);
  };
}

export const patternTools: Record<PatternToolId, PatternTool> = {
  addInput: {
    type: 'Pattern',
    subtype: 'Structure',
    id: 'addInput',
    inputs: [],
    inputTypes: [],
    name: 'Add Input',
    icon: 'new_label',
    documentation: 'Creates an input node for an operation.',
    perform: (operation, dispatch) => dispatch(addInput(operation!.id)),
  },
  appendChild: {
    type: 'Pattern',
    subtype: 'Structure',
    id: 'appendChild',
    inputs: [{name: 'Node'}],
    inputTypes: [ANY_TYPE_ID],
    name: 'Add Child',
    icon: 'arrow_downward',
    documentation: 'Adds a child patter node.',
    perform: withPatternNode((o, dispatch, [n]) =>
      dispatch(patternAddChild(n.pattern))
    ),
  },
  appendParent: {
    type: 'Pattern',
    subtype: 'Structure',
    id: 'appendParent',
    inputs: [{name: 'Node'}],
    inputTypes: [ANY_TYPE_ID],
    name: 'Add Parent',
    icon: 'arrow_upward',
    documentation: 'Adds a parent pattern node.',
    perform: withPatternNode((o, dispatch, [n]) =>
      dispatch(patternAddParent(n.pattern))
    ),
  },
  deletePatternNode: {
    type: 'Pattern',
    subtype: 'Structure',
    id: 'deletePatternNode',
    inputs: [{name: 'Node'}],
    inputTypes: [ANY_TYPE_ID],
    name: 'Delete Pattern',
    icon: 'close',
    documentation:
      'Deletes a pattern node and the parents and children sprouting away from it.',
    perform: withPatternNode((o, dispatch, [n]) =>
      dispatch(patternDeletePattern(n.pattern))
    ),
  },
  changeExampleValue: {
    perform: (operation, dispatch, [a, b]) => {
      if (a.type === 'PatternMatch' && b.type === 'Literal') {
        dispatch(
          editorChangeExampleValue({pattern: a.pattern, value: b.value.value})
        );
      }
    },
    subtype: 'Values',
    documentation: 'Change the example value of an input/pattern node.',
    inputs: [{name: 'Node'}, {name: 'Value'}],
    inputTypes: [ANY_TYPE_ID, NUMBER_TYPE_ID],
    icon: 'edit',
    type: 'Pattern',
    name: 'Change Example Value',
    id: 'changeExampleValue',
  },
};
