/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import GraphView from 'features/graphView/GraphView';
import {useMemo} from 'react';
import ScrollView from 'components/ScrollView';
import {
  ConcreteGraph,
  ConcreteNode as ConcreteNodeType,
  ConcreteNodeId,
} from 'src/ConcreteGraph';
import {GraphAdapter} from 'features/graphView/GraphAdapter';
import ConcreteNode from 'features/playground/ConcreteNode';

/**
 * The Playground component is responsible for showing a concrete graph.
 * It is used in the state view or template view, for example.
 * @param graph A concrete graph to display.
 */
export default function Playground({graph}: {graph: ConcreteGraph}) {
  const biGraph: GraphAdapter<ConcreteNodeId, ConcreteNodeType> = useMemo(
    () => ({
      nodes: Object.keys(graph.nodes),
      outgoingEdges(id) {
        return graph.nodes[id].outgoingEdges.map(({target}) => target);
      },
      incomingEdges(id) {
        return graph.nodes[id].incomingEdges.map(({source}) => source);
      },
      style(id) {
        return graph.nodes[id].style || null;
      },
      payload(id) {
        return graph.nodes[id];
      },
      key: i => i,
    }),
    [graph]
  );

  return (
    <ScrollView>
      <GraphView GraphNode={ConcreteNode} graph={biGraph} />
    </ScrollView>
  );
}
