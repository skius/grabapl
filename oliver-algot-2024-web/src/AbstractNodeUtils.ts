/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {Operation} from 'src/Operation';
import {
  AbstractNodeDescriptor,
  AbstractNodeKey,
  fromOutputKey,
} from 'src/DemoSemantics';
import {stringRepresentation} from 'src/ConcreteValue';

/**
 * Looks up the name assigned to this abstract node in the operation.
 */
export function nameForAbstractNode(
  node: AbstractNodeDescriptor,
  operation: Operation
): string {
  switch (node.type) {
    case 'PatternMatch':
      return operation.patterns[node.pattern].name;
    case 'OperationOutput': {
      const split = fromOutputKey(node.id);
      const outputName = split[split.length - 1];
      return (
        outputName +
        split
          .slice(0, -2)
          .map(() => "'")
          .join('')
      );
    }
    case 'Literal':
      return `${stringRepresentation(node.value)}`;
    case 'Undefined':
      return '??';
  }
}

/**
 * Returns a key uniquely identifying the provided abstract node.
 * This key is not supposed to be parsed. It is intended for looking up values
 * in maps or passing it to the React key parameter.
 */
export function keyForAbstractNode(
  node: AbstractNodeDescriptor
): AbstractNodeKey {
  switch (node.type) {
    case 'PatternMatch':
      return `p${node.pattern}`;
    case 'OperationOutput':
      return `o${node.id}`;
    case 'Literal':
      return `l${JSON.stringify(node.value)}`;
    case 'Undefined':
      return 'u';
  }
}
