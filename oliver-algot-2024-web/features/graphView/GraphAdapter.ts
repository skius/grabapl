/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
/**
 * An interface for specifying simple directed graphs. Used by computeLayout.
 */
import {NodeStyle} from 'src/NodeStyle';

export interface GraphAdapter<Id, Payload> {
  /** A list of all nodes in the graph. */
  nodes: Id[];
  /** @returns All the adjacent nodes of `id`. */
  outgoingEdges(id: Id): Id[];
  /** @returns All the nodes to which `id` is adjacent. */
  incomingEdges(id: Id): Id[];

  /** @returns A unique (in regard to this graph) string key for the node. */
  key(id: Id): string;

  style(id: Id): NodeStyle | null;

  payload(id: Id): Payload;
}
