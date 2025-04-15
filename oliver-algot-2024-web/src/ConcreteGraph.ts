/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {ConcreteValue} from 'src/ConcreteValue';
import {NodeStyle} from 'src/NodeStyle';

export type ConcreteNodeId = string & {readonly brand?: unique symbol};

export interface ConcreteGraph {
  nextId: number;
  nodes: Record<ConcreteNodeId, ConcreteNode>;
}

export interface ConcreteNode {
  id: ConcreteNodeId;
  value: ConcreteValue;
  outgoingEdges: Edge[];
  incomingEdges: Edge[];
  style?: NodeStyle;
}

export interface Edge {
  source: ConcreteNodeId;
  target: ConcreteNodeId;
  weight: number;
}
