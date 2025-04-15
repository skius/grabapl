/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import {Operation, OperationId} from 'src/Operation';
import {AbstractNodeKey, Pattern, PatternId} from 'src/DemoSemantics';
import {
  ANY_TYPE,
  ANY_TYPE_ID,
  ConcreteValue,
  makeValue,
  NUMBER_TYPE,
  NUMBER_TYPE_ID,
  STRING_TYPE_ID,
} from 'src/ConcreteValue';
import {GraphNode} from 'src/GraphAPI';
import React, {HTMLProps, PropsWithChildren} from 'react';
import {useAppDispatch} from './hooks';
import {highlightArgs, unhighlightArgs} from 'features/editor/editorReducer';
import {ComparisonOperator} from 'features/editor/PredicateButton';
import { NumberTypeError } from './ConcreteGraphAPI';

export enum BaseOperationCategory {
  Basics = 'Basics',
  Math = 'Math',
  Lists = 'Lists',
  Text = 'Text',
  Hidden = 'Hidden',
}

export interface BaseOperation extends Operation {
  /**
   * Provide a JavaScript implementation of the operation.
   * @param nodes List of nodes provided as inputs to the operation.
   * @param api Access to the API for modifying the state graph.
   */
  category: BaseOperationCategory;
  perform: <T extends GraphNode<T>>(
    nodes: T[],
    api: {
      makeNode(value: ConcreteValue): T;
      setQueryResult?(result: boolean): void;
    }
  ) => boolean | ComparisonOperator[] | void;
  instruction: GeneralFunc;
  question?: GeneralFunc;
  trueCase?: GeneralFunc;
  falseCase?: GeneralFunc;
  hasOutput?: boolean;
  outputDescription?: (name: string) => string;
}

/**
 * A user-defined type guard to test and inform the TypeScript type-checker
 * whether an operation is a BaseOperation.
 * @param operation
 */
export function isBaseOperation(
  operation: Operation
): operation is BaseOperation {
  return 'perform' in operation;
}

export function isQuery(operation: Operation): operation is BaseOperation {
  return operation !== undefined && operation.isQuery;
}

export function hasCategory(operation: Operation): operation is BaseOperation {
  return 'category' in operation;
}

export function N({
  ankey,
  normalText = false,
  inexistent = false,
  boldText = false,
  children,
  ...props
}: PropsWithChildren<
  {
    ankey?: AbstractNodeKey;
    normalText?: boolean;
    inexistent?: boolean;
    boldText?: boolean;
    // onDragEnter?: DragEventHandler<HTMLSpanElement>;
    // onDragEnd?: DragEventHandler<HTMLSpanElement>;
  } & HTMLProps<HTMLSpanElement>
>) {
  const dispatch = useAppDispatch();
  const {style, ...propsRest} = props;
  return (
    <span
      style={{
        position: 'relative',
        display: 'inline-block',
        cursor: 'pointer',
        ...style,
      }}
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      {...propsRest}
    >
      <span
        style={
          inexistent
            ? {color: 'red'}
            : boldText
            ? {fontWeight: 900}
            : normalText
            ? {}
            : {fontWeight: 900, color: 'darkblue'}
        }
      >
        {children}
      </span>
      <span
        style={{
          position: 'absolute',
          top: '-10px',
          left: '-10px',
          right: '-10px',
          bottom: '-10px',
        }}
        onMouseEnter={() => {
          if (!ankey) return;
          const args: Record<AbstractNodeKey, string[]> = {};
          args[ankey] = [];
          dispatch(highlightArgs(args));
        }}
        onMouseLeave={() => {
          if (!ankey) return;
          dispatch(unhighlightArgs());
        }}
      />
    </span>
  );
}

export type GeneralFunc = <T>(nodes: T[]) => (string | T)[];
export type QuestionFunc = <T>(nodes: T[]) => (string | T)[];

function makeInputs(names: string[]) {
  return {
    inputs: names,
    patterns: names.reduce((o, name) => {
      o[name] = {id: name, name, incoming: [], outgoing: []};
      return o;
    }, {} as Record<PatternId, Pattern>),
  };
}

type PartialBy<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;

function addBaseOperation(
  op: PartialBy<
    BaseOperation,
    'type' | 'inputs' | 'patterns' | 'isUserDefined'
  > & {
    inputNames?: string[];
  }
) {
  baseOperations[op.id] = {
    type: 'Operation',
    patterns: {},
    inputs: [],
    ...(op.inputNames ? makeInputs(op.inputNames) : null),
    ...op,
    isUserDefined: false,
  };
}

function m<T extends unknown[], A>(f: (...args: T) => A, ...args: T): A {
  return f(...args);
}

const baseOperations: Record<OperationId, BaseOperation> = {};

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a', 'b'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID],
  icon: 'difference',
  name: 'Compare Numbers',
  question: ([a, b]) => ['Is ', a, ' <OP> ', b, '?'],
  trueCase: ([a, b]) => [a, ' <OP> ', b],
  falseCase: () => ['THIS SHOULD NOT EXIST'],
  id: 'compareNumbers',
  instruction: ([a, b]) => [a, ' <OP> ', b],
  perform([a, b]) {
    const results = new Set<ComparisonOperator>();
    if (a.numberValue > b.numberValue) results.add('>').add('>=');
    if (a.numberValue === b.numberValue) results.add('==').add('>=').add('<=');
    else results.add('!=');
    if (a.numberValue < b.numberValue) results.add('<').add('<=');
    return Array.from(results);
  },
  isQuery: true,
  documentation: 'Compares two numbers.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a', 'b'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'fingerprint',
  name: 'Is Same?',
  question: ([a, b]) => ['Is ', a, ' the same node as ', b, '?'],
  trueCase: ([a, b]) => [a, ' is the same node as ', b],
  falseCase: ([a, b]) => [a, ' is not the same node as ', b],
  id: 'isSame',
  instruction: ([a, b]) => [a, ' and ', b, ' are the same node'],
  perform([a, b]) {
    return a === b;
  },
  isQuery: true,
  documentation: 'Compares whether two nodes are in fact the same node.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'exposure_zero',
  name: 'Is Zero?',
  question: ([a]) => ['Is the value of ', a, ' equal to 0?'],
  trueCase: ([a]) => [a, ' = 0'],
  falseCase: ([a]) => [a, ' ≠ 0'],
  id: 'isZero',
  instruction: ([a]) => [a, ' is 0'],
  perform: ([a]) => {
    const res = m(x => x.type === NUMBER_TYPE_ID && x.value === 0, a.value);
    return res;
  },
  isQuery: true,
  documentation: 'Checks whether a node has the value 0.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'start',
  name: 'Is Positive?',
  question: ([a]) => ['Is the value of ', a, ' positive?'],
  trueCase: ([a]) => [a, ' > 0'],
  falseCase: ([a]) => [a, ' ≤ 0'],
  id: 'isPositive',
  instruction: ([a]) => [a, ' is positive'],
  perform: ([a]) => {
    const res = m(x => x.type === NUMBER_TYPE_ID && x.value > 0, a.value);
    return res;
  },
  isQuery: true,
  documentation: 'Checks whether a node has a positive value.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'moving',
  name: 'Has Outgoing?',
  question: ([a]) => ['Does ', a, ' have any children?'],
  trueCase: ([a]) => [a, ' has children'],
  falseCase: ([a]) => [a, " doesn't have children"],
  id: 'hasOutgoing',
  instruction: ([a]) => [a, ' has children'],
  perform: ([a]) => a.hasNeighbors,
  isQuery: true,
  documentation: 'Checks whether a node has any outgoing edges.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a', 'b'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'fingerprint',
  name: 'Is Same Value?',
  question: ([a, b]) => [
    'Is the value of ',
    a,
    ' equal to the value of ',
    b,
    '?',
  ],
  trueCase: ([a, b]) => [a, ' = ', b],
  falseCase: ([a, b]) => [a, ' ≠ ', b],
  id: 'equalNumber',
  instruction: ([a, b]) => [a, ' and ', b, ' have the same value'],
  perform: ([a, b]) => m((a, b) => a.value === b.value, a.value, b.value),
  isQuery: true,
  documentation: 'Compares whether two nodes have the same value.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a', 'b'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'compare_arrows',
  name: 'Has Lower Value?',
  question: ([a, b]) => [
    'Is the value of ',
    a,
    ' less than the value of ',
    b,
    '?',
  ],
  trueCase: ([a, b]) => [a, ' < ', b],
  falseCase: ([a, b]) => [a, ' ≥ ', b],
  id: 'isLessThan',
  instruction: ([a, b]) => [a, ' has a lower value than ', b],
  perform: ([a, b]) => m((a, b) => a < b, a.numberValue, b.numberValue),
  isQuery: true,
  documentation:
    'Checks if the first node has a lower value than the second node.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['a', 'b'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'east',
  name: 'Has Edge to?',
  question: ([a, b]) => ['Does ', a, ' have an edge to ', b, '?'],
  trueCase: ([a, b]) => [a, ' has an edge to ', b],
  falseCase: ([a, b]) => [a, " doesn't have an edge to ", b],
  id: 'hasEdgeTo',
  instruction: ([a, b]) => [a, ' has an edge to ', b],
  perform: ([a, b]) => {
    const result = a.hasEdgeTo(b);
    return result;
  },
  isQuery: true,
  documentation: 'Checks whether a node has an edge to another node.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: [],
  inputTypes: [],
  icon: 'check',
  name: 'Change Query Result',
  id: 'setQueryResultToTrue',
  instruction: () => ['Set Query Result to True'],
  //calls setQueryResult(true) on the api:
  perform(_, {setQueryResult}) {
    if (setQueryResult !== undefined) {
      setQueryResult(true);
    }
  },
  isQuery: false,
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: [],
  inputTypes: [],
  icon: 'close',
  name: 'Change Query Result',
  id: 'setQueryResultToFalse',
  instruction: () => ['Set Query Result to False'],
  perform(_, {setQueryResult}) {
    if (setQueryResult !== undefined) {
      setQueryResult(false);
    }
  },
  isQuery: false,
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: [],
  inputTypes: [],
  icon: 'add_box',
  name: 'New Node',
  id: 'newNode',
  instruction: () => ['Create a new node with value 0'],
  perform(_, {makeNode}) {
    makeNode(makeValue(0, NUMBER_TYPE));
  },
  isQuery: false,
  documentation: 'Creates a new node that has the value 0.',
  hasOutput: true,
  outputDescription: name => `Call the new node ${name}`,
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['parent'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'add_comment',
  name: 'Add Child',
  id: 'addChild',
  instruction: ([a]) => ['Add a child to ', a],
  perform([a], {makeNode}) {
    const n = makeNode(makeValue(0, NUMBER_TYPE));
    a.addEdgeTo(n);
  },
  isQuery: false,
  documentation: 'Adds a new child node with the value 0 to the selected node.',
  hasOutput: true,
  outputDescription: name => `Call the new node ${name}`,
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['a'],
  inputTypes: [NUMBER_TYPE_ID],
  icon: 'add_circle',
  name: 'Increment',
  id: 'increment',
  instruction: ([a]) => ['Increment the value of ', a],
  perform([a]) {
    a.value = m(x => x + 1, a.numberValue);
  },
  isQuery: false,
  documentation: 'Increases the value of the selected node by 1.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['a'],
  inputTypes: [NUMBER_TYPE_ID],
  icon: 'remove_circle',
  name: 'Decrement',
  id: 'decrement',
  instruction: ([a]) => ['Decrement the value of ', a],
  perform([a]) {
    a.value = m(x => x - 1, a.numberValue);
  },
  isQuery: false,
  documentation: 'Decreases the value of the selected node by 1.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['from', 'to'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'arrow_right_alt',
  name: 'Add Edge To',
  id: 'addEdgeTo',
  instruction: ([a, b]) => ['Add an edge from ', a, ' to ', b],
  perform([a, b]) {
    a.addEdgeTo(b);
  },
  isQuery: false,
  documentation: 'Adds an edge from the first node to the second node.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'leak_remove',
  name: 'Remove Edges',
  id: 'removeEdges',
  instruction: ([a]) => ['Remove all edges from ', a],
  perform([a]) {
    a.removeEdges();
  },
  isQuery: false,
  documentation: 'Removes all edges from and to the selected node.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['from', 'to'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'content_copy',
  name: 'Copy Value',
  id: 'copyValue',
  instruction: ([a, b]) => ['Copy value from ', a, ' to ', b],
  perform([from, to]) {
    to.value = from.value;
  },
  isQuery: false,
  documentation: 'Copies the value of the first node to the second node.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'backspace',
  name: 'Delete Node',
  id: 'removeNode',
  instruction: ([a]) => ['Delete node ', a],
  perform([a]) {
    a.remove();
  },
  isQuery: false,
  documentation: 'Deletes the selected node.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['destination'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'question_answer',
  name: 'Set Value',
  id: 'setValue',
  instruction: ([a, b]) => ['Set the value of ', a, ' to ', b],
  perform([a, b]) {
    b.value = a.value;
  },
  isQuery: false,
  documentation: 'Asks for a number and stores it in the selected node.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['a', 'b'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'swap_horizontal_circle',
  name: 'Swap Value',
  id: 'swapValue',
  instruction: ([a, b]) => ['Swap the values of ', a, ' and ', b],
  perform([a, b]) {
    const tmp = a.value;
    a.value = b.value;
    b.value = tmp;
  },
  isQuery: false,
  documentation: 'Swaps the values of the selected nodes',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['destination'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'question_answer',
  name: 'Ask for Number',
  id: 'prompt',
  instruction: ([a]) => ['Set the value of ', a, ' by asking the user'],
  perform([a]) {
    let v = NaN;
    do {
      const input = prompt('Please enter a value');
      if (input) v = parseInt(input);
    } while (Number.isNaN(v));
    a.value = v;
  },
  isQuery: false,
  documentation: 'Asks for a number and stores it in the selected node.',
});

addBaseOperation({
  category: BaseOperationCategory.Basics,
  inputNames: ['destination'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'question_answer',
  name: 'Ask for Text',
  id: 'promptString',
  instruction: ([a]) => ['Set the value of ', a, ' by asking the user'],
  perform([a]) {
    const response = prompt('Please enter a string');
    a.value = response || '';
  },
  isQuery: false,
  documentation: 'Asks for a number and stores it in the selected node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'b', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'calculate',
  name: 'Sum',
  id: 'sum',
  instruction: ([a, b, result]) => [result, ' = ', a, ' + ', b],
  perform([a, b, result]) {
    try {
      result.value = m((x, y) => x + y, a.numberValue, b.numberValue);
    } catch (e) {
      // console.log('error with number value e');
    }
  },
  isQuery: false,
  documentation: 'Saves the sum of the first two nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'b', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'calculate',
  name: 'Multiply',
  id: 'multiply',
  instruction: ([a, b, result]) => [result, ' = ', a, ' * ', b],
  perform([a, b, result]) {
    try {
      result.value = m((x, y) => x * y, a.numberValue, b.numberValue);
    } catch (e) {
      // console.log('error with number value e');
    }
  },
  isQuery: false,
  documentation: 'Saves the product of the first two nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'b', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'calculate',
  name: 'Subtract',
  id: 'subtract',
  instruction: ([a, b, result]) => [result, ' = ', a, ' - ', b],
  perform([a, b, result]) {
    result.value = m((x, y) => x - y, a.numberValue, b.numberValue);
  },
  isQuery: false,
  documentation:
    'Saves the difference of the first two nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'b', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'calculate',
  name: 'Modulo',
  id: 'modulo',
  instruction: ([a, b, result]) => [result, ' = ', a, ' % ', b],
  perform([a, b, result]) {
    result.value = m((x, y) => ((x % y) + y) % y, a.numberValue, b.numberValue);
  },
  isQuery: false,
  documentation: 'Saves the modulo of the first two nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'b', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'calculate',
  name: 'Maximum',
  id: 'maximum',
  instruction: ([a, b, result]) => [result, ' = max(', a, ', ', b, ')'],
  perform([a, b, result]) {
    result.value = m((x, y) => Math.max(x, y), a.numberValue, b.numberValue);
  },
  isQuery: false,
  documentation: 'Saves the maximum of the first two nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['from', 'to', 'result'],
  inputTypes: [NUMBER_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'casino',
  name: 'Random in range',
  id: 'randomRange',
  instruction: ([a, b, result]) => [
    result,
    ' = random number from ',
    a,
    ' to ',
    b,
  ],
  perform([a, b, result]) {
    result.value = m(
      (min, max) => Math.floor(Math.random() * (max - min + 1) + min),
      a.numberValue,
      b.numberValue
    );
  },
  isQuery: false,
  documentation:
    'Stores a random number in the range between the two input nodes in the third node.',
});

addBaseOperation({
  category: BaseOperationCategory.Math,
  inputNames: ['a', 'result'],
  inputTypes: [NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'text_increase',
  name: 'Absolute Value',
  id: 'absoluteValue',
  instruction: ([a, result]) => [result, ' = |', a, '|'],
  perform([a, result]) {
    result.value = m(x => Math.abs(x), a.numberValue);
  },
  isQuery: false,
  documentation:
    'Stores the absolute value of the input node in the output node.',
});

addBaseOperation({
  category: BaseOperationCategory.Lists,
  inputNames: ['list', 'len'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'straighten',
  name: 'List Length',
  id: 'listLength',
  instruction: ([a, b]) => [
    'Store the length of the list starting at ',
    a,
    ' in ',
    b,
  ],
  perform([a, b]) {
    b.value = 0;
    const visited = new Set<typeof a>();
    let current = a;
    while (!visited.has(current)) {
      visited.add(current);
      if (current.hasNeighbors) current = current.neighbors[0];
      b.value = m((a, b) => a + b, b.numberValue, 1);
    }
  },
  isQuery: false,
  documentation: 'Stores the length of a list in a node.',
});

addBaseOperation({
  category: BaseOperationCategory.Hidden,
  inputNames: ['list', 'value'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'add_box',
  name: 'Contains in List?',
  id: 'containsInList',
  question: ([a, b]) => ['Does ', a, ' contain ', b, '?'],
  trueCase: ([a, b]) => [a, ' contains ', b],
  falseCase: ([a, b]) => [a, ' does not contain ', b],
  instruction: ([a, b]) => [a, ' contains ', b],
  perform([a, b]) {
    let current = a;
    const visited = new Set<typeof a>();
    while (!visited.has(current)) {
      visited.add(current);
      if (m((a, b) => a === b, current.numberValue, b.numberValue)) return true;
      if (current.hasNeighbors) current = current.neighbors[0];
    }
    return false;
  },
  isQuery: true,
  documentation: 'Checks whether a list contains a value.',
});

addBaseOperation({
  category: BaseOperationCategory.Lists,
  inputNames: ['list', 'a'],
  inputTypes: [ANY_TYPE_ID, ANY_TYPE_ID],
  icon: 'playlist_add',
  name: 'Add to List',
  id: 'addToList',
  instruction: ([a, b]) => ['Add ', b, ' to ', a],
  perform([a, b], {makeNode}) {
    const newNode = makeNode(makeValue(b.numberValue, NUMBER_TYPE));
    let current = a;
    while (current.hasNeighbors) {
      current = current.neighbors[0];
    }
    current.addEdgeTo(newNode);
  },
  hasOutput: true,
  outputDescription: name => `Call the new node ${name}`,
  isQuery: false,
  documentation: 'Adds a value to a list.',
});

addBaseOperation({
  category: BaseOperationCategory.Lists,
  inputNames: ['a'],
  inputTypes: [ANY_TYPE_ID],
  icon: 'playlist_remove',
  name: 'Remove from List',
  id: 'removeFromList',
  instruction: ([a]) => ['Remove ', a, ' from its list'],
  perform([a]) {
    const incoming = a.nodesWithIncomingEdges;
    const next = a.neighbors;

    a.remove();
    if (incoming.length === 1 && next.length === 1) {
      incoming[0].addEdgeTo(next[0]);
    }
  },
  isQuery: false,
  documentation: 'Removes a value from its list.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['text', 'length'],
  inputTypes: [STRING_TYPE_ID, ANY_TYPE_ID],
  icon: 'straighten',
  name: 'Length of String',
  id: 'stringLength',
  instruction: ([a, b]) => ['Store the length of ', a, ' in ', b],
  perform([a, b]) {
    b.value = m(x => x.length, a.stringValue);
  },
  isQuery: false,
  documentation: 'Stores the length of a string in a node.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['text', 'index', 'result'],
  inputTypes: [STRING_TYPE_ID, NUMBER_TYPE_ID, ANY_TYPE_ID],
  icon: 'copyright',
  name: 'Get nth Character',
  id: 'getCharacter',
  instruction: ([a, b, c]) => [
    'Store the character at index ',
    b,
    ' of ',
    a,
    ' in ',
    c,
  ],
  perform([a, b, c]) {
    c.value = m((a, b) => a[b], a.stringValue, b.numberValue);
  },
  isQuery: false,
  documentation: 'Stores the character at a given index in a string in a node.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['text', 'index', 'value'],
  inputTypes: [STRING_TYPE_ID, NUMBER_TYPE_ID, STRING_TYPE_ID],
  icon: 'copyright',
  name: 'Set nth Character',
  id: 'setCharacter',
  instruction: ([a, b, c]) => [
    'Set the character at index ',
    b,
    ' of ',
    a,
    ' to ',
    c,
  ],
  perform([a, b, c]) {
    a.value = m(
      (a, b, c) => a.slice(0, b) + c + a.slice(b + 1),
      a.stringValue,
      b.numberValue,
      c.stringValue
    );
  },
  isQuery: false,
  documentation: 'Sets the character at a given index in a string.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['text'],
  inputTypes: [STRING_TYPE_ID],
  icon: 'cut',
  name: 'Remove First Letter',
  id: 'removeFirstLetter',
  instruction: ([a]) => ['Remove the first letter of ', a],
  perform([a]) {
    a.value = m(a => a.slice(1), a.stringValue);
  },
  isQuery: false,
  documentation: 'Removes the first letter of a string.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['text'],
  inputTypes: [STRING_TYPE_ID],
  icon: 'cut',
  name: 'Remove Last Letter',
  id: 'removeLastLetter',
  instruction: ([a]) => ['Remove the last letter of ', a],
  perform([a]) {
    a.value = m(a => a.slice(0, -1), a.stringValue);
  },
  isQuery: false,
  documentation: 'Removes the last letter of a string.',
});

// Concatenation
addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['a', 'b', 'target'],
  inputTypes: [STRING_TYPE_ID, STRING_TYPE_ID, ANY_TYPE_ID],
  icon: 'add',
  name: 'Concatenate',
  id: 'concatenate',
  instruction: ([a, b, c]) => [c, ' = ', a, ' concatenated with ', b],
  perform([a, b, c]) {
    c.value = m((a, b) => a + b, a.stringValue, b.stringValue);
  },
  isQuery: false,
  documentation: 'Concatenates the second node to the first node.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['target', 'copy'],
  inputTypes: [STRING_TYPE_ID, ANY_TYPE_ID],
  icon: 'first_page',
  name: 'Get First Letter',
  id: 'firstLetter',
  instruction: ([a, b]) => ['Store the first letter of ', a, ' in ', b],
  perform([a, b]) {
    b.value = m(a => (a.length > 0 ? a.slice(0, 1) : ''), a.stringValue);
  },
  isQuery: false,
  documentation: 'Stores the first letter of a node into the second node.',
});

addBaseOperation({
  category: BaseOperationCategory.Text,
  inputNames: ['target', 'copy'],
  inputTypes: [STRING_TYPE_ID, ANY_TYPE_ID],
  icon: 'last_page',
  name: 'Get Last Letter',
  id: 'lastLetter',
  instruction: ([a, b]) => ['Store the last letter of ', a, ' in ', b],
  perform([a, b]) {
    b.value = m(a => (a.length > 0 ? a.slice(-1) : ''), a.stringValue);
  },
  isQuery: false,
  documentation: 'Stores the last letter of a node into the second node.',
});

export default baseOperations;
