/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {GraphAPI, GraphNode} from 'src/GraphAPI';
import {
  ConcreteValue,
  extractPayload,
  NUMBER_TYPE_ID,
  STRING_TYPE_ID,
  Type,
} from 'src/ConcreteValue';
import {NodeStyle} from 'src/NodeStyle';
import {AbstractNodeDescriptor, QueryApplicationId} from 'src/DemoSemantics';
import {GraphAdapter} from 'features/graphView/GraphAdapter';
import {keyForAbstractNode} from 'src/AbstractNodeUtils';
import {NumberTypeError} from './ConcreteGraphAPI';

export class ApproximateGraphNode implements GraphNode<ApproximateGraphNode> {
  constructor(
    public readonly abstractNode: AbstractNodeDescriptor,
    private readonly graphApi: ApproximateGraphAPI,
    public concreteValue: ConcreteValue
  ) {
    graphApi.registerNode(this);
  }

  private _neighbors: ApproximateGraphNode[] = [];

  get neighbors(): ApproximateGraphNode[] {
    return this._neighbors;
  }

  style: NodeStyle = {};

  nodesWithIncomingEdges: ApproximateGraphNode[] = [];

  remove(): void {
    this.removeEdges();
    this.graphApi.nodes = this.graphApi.nodes.filter(n => n !== this);
  }

  addEdgeTo(node: ApproximateGraphNode): void {
    if (this._neighbors.indexOf(node) === -1) {
      this._neighbors.push(node);
      node.nodesWithIncomingEdges.push(this);
    }
  }

  get hasNeighbors(): boolean {
    return this.neighbors.length > 0;
  }

  removeEdges(): void {
    this.neighbors.forEach(n => {
      n.nodesWithIncomingEdges = n.nodesWithIncomingEdges.filter(
        incoming => incoming !== this
      );
    });
    this.nodesWithIncomingEdges.forEach(n => {
      n._neighbors = n._neighbors.filter(incoming => incoming !== this);
    });
    this.nodesWithIncomingEdges = [];
    this._neighbors = [];
  }

  hasEdgeTo(node: ApproximateGraphNode): boolean {
    return this._neighbors.indexOf(node) !== -1;
  }

  get numberValue(): number {
    const val = this.value;
    switch (val.type) {
      case STRING_TYPE_ID: {
        const res = parseFloat(val.value);
        // if (isNaN(res)) throw new NumberTypeError();
        return res;
      }
      case NUMBER_TYPE_ID:
        return val.value;
      default:
        throw new NumberTypeError();
    }
  }

  get stringValue(): string {
    const val = this.value;
    return `${val.value}`;
  }

  payload<P>(type: Type<P>): P {
    return extractPayload(this.concreteValue, type);
  }

  get value(): ConcreteValue {
    return this.concreteValue;
  }

  set value(v: ConcreteValue | number | string) {
    const val =
      typeof v === 'object'
        ? v
        : typeof v === 'string'
        ? {type: STRING_TYPE_ID, value: v}
        : {type: NUMBER_TYPE_ID, value: v};

    this.concreteValue = val;
  }

  setStyle<K extends keyof NodeStyle>(key: K, value: NodeStyle[K]) {
    this.style[key] = value;
  }

  updateOutputId(outputId: string) {
    if (this.abstractNode.type !== 'OperationOutput')
      throw 'Invariant violated! Someone called updateOutputId on a node that is not an operation output. Did this node end up in the output stack?';
    this.abstractNode.id = outputId;
  }

  makeSerializable(): ApproximateGraphNodeData {
    return {
      abstractNode: this.abstractNode,
      concreteValue: this.concreteValue,
    };
  }
}

export type ApproximateGraphNodeData = {
  abstractNode: AbstractNodeDescriptor;
  concreteValue: ConcreteValue;
};

export function reconstructApproximateNodeFrom(
  data: ApproximateGraphNodeData,
  graphApi: ApproximateGraphAPI
): ApproximateGraphNode {
  return new ApproximateGraphNode(
    data.abstractNode,
    graphApi,
    data.concreteValue
  );
}

export class ApproximateGraphAPI
  extends GraphAPI<ApproximateGraphNode>
  implements GraphAdapter<ApproximateGraphNode, ApproximateGraphNode>
{
  nodes: ApproximateGraphNode[] = [];

  registerNode(node: ApproximateGraphNode) {
    this.nodes.push(node);
    return this.nodes.length.toString();
  }

  protected makeNode(
    value: ConcreteValue,
    temporary: boolean,
    id: string
  ): ApproximateGraphNode {
    return new ApproximateGraphNode(
      temporary
        ? {type: 'Literal', value}
        : {
            type: 'OperationOutput',
            id: id!,
          },
      this,
      value
    );
  }

  incomingEdges(id: ApproximateGraphNode): ApproximateGraphNode[] {
    return id.nodesWithIncomingEdges;
  }

  key(node: ApproximateGraphNode): string {
    return keyForAbstractNode(node.abstractNode);
  }

  outgoingEdges(id: ApproximateGraphNode): ApproximateGraphNode[] {
    return id.neighbors;
  }

  payload(id: ApproximateGraphNode): ApproximateGraphNode {
    return id;
  }

  style(node: ApproximateGraphNode): NodeStyle | null {
    return node.style as NodeStyle;
  }

  makeSerializable(): ApproximateGraphData {
    const res = {
      nodes: this.nodes
        .filter(n => n.abstractNode.type !== 'Literal')
        .map(n => n.makeSerializable()),
      edges: this.nodes.flatMap((n, i) =>
        n.neighbors.map(n => ({from: i, to: this.nodes.indexOf(n)}))
      ),
      queryResults: {},
      customQueryResult: this.queryResult,
    };
    return res;
  }
}

export type ApproximateGraphData = {
  nodes: ApproximateGraphNodeData[];
  edges: {from: number; to: number}[];
  queryResults: Record<QueryApplicationId, boolean | null>;
  customQueryResult: boolean;
};

export function reconstructApproximateGraphFrom(
  data: ApproximateGraphData
): ApproximateGraphAPI {
  const graph = new ApproximateGraphAPI();
  data.nodes.forEach(n => reconstructApproximateNodeFrom(n, graph));
  data.edges.forEach(n => {
    graph.nodes[n.from].addEdgeTo(graph.nodes[n.to]);
  });
  graph.queryResult = data.customQueryResult;
  return graph;
}
