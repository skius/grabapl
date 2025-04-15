/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {ConcreteGraph, ConcreteNodeId} from 'src/ConcreteGraph';
import {ConcreteArgument, isLiteralArgument} from 'features/tools/Tool';
import {
  ConcreteValue,
  extractPayload,
  NUMBER_TYPE_ID,
  STRING_TYPE_ID,
  Type,
} from 'src/ConcreteValue';
import {GraphAPI, GraphNode} from 'src/GraphAPI';
import {NodeStyle} from 'src/NodeStyle';
import {
  ApproximateGraphData,
  ApproximateGraphNodeData,
} from './ApproximateGraphAPI';

export class NumberTypeError extends Error {
  constructor() {
    super('Input value expected a number, but was supplied something else');
  }
}

export class ConcreteGraphNode implements GraphNode<ConcreteGraphNode> {
  constructor(
    public readonly id: ConcreteNodeId,
    private readonly api: ConcreteGraphAPI
  ) {}

  addEdgeTo(node: ConcreteGraphNode) {
    if (this.id === node.id) throw 'Self loop';
    if (this.hasEdgeTo(node)) return;
    const edge = {
      source: this.id,
      target: node.id,
      weight: 0,
    };
    this.api.graph.nodes[this.id].outgoingEdges.push(edge);
    this.api.graph.nodes[node.id].incomingEdges.push(edge);
  }

  get value(): ConcreteValue {
    return this.api.graph.nodes[this.id].value;
  }

  set value(v: ConcreteValue | number | string) {
    this.api.graph.nodes[this.id].value =
      typeof v === 'object'
        ? v
        : typeof v === 'string'
        ? {type: STRING_TYPE_ID, value: v}
        : {type: NUMBER_TYPE_ID, value: v};
  }

  get numberValue(): number {
    const val = this.value;
    switch (val.type) {
      case STRING_TYPE_ID: {
        const res = parseFloat(val.value);
        if (isNaN(res)) throw new NumberTypeError();
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
    return extractPayload(this.value, type);
  }

  get neighbors() {
    return this.api.graph.nodes[this.id].outgoingEdges.map(({target}) =>
      this.api.getNode(target)
    );
  }

  get hasNeighbors() {
    return this.api.graph.nodes[this.id].outgoingEdges.length > 0;
  }

  get nodesWithIncomingEdges() {
    return this.api.graph.nodes[this.id].incomingEdges.map(({source}) =>
      this.api.getNode(source)
    );
  }

  setStyle<K extends keyof NodeStyle>(key: K, value: NodeStyle[K]) {
    (this.api.graph.nodes[this.id].style ||= {})[key] = value;
  }

  hasEdgeTo(node: ConcreteGraphNode) {
    return this.api.graph.nodes[this.id].outgoingEdges.some(
      ({target}) => target === node.id
    );
  }

  remove() {
    this.removeEdges();
    delete this.api.graph.nodes[this.id];
    this.api.removalListeners.forEach(f => f(this));
  }

  removeEdges() {
    this.api.graph.nodes[this.id].outgoingEdges.forEach(({target}) => {
      this.api.graph.nodes[target].incomingEdges = this.api.graph.nodes[
        target
      ].incomingEdges.filter(({source}) => source !== this.id);
    });
    this.api.graph.nodes[this.id].outgoingEdges = [];

    this.api.graph.nodes[this.id].incomingEdges.forEach(({source}) => {
      this.api.graph.nodes[source].outgoingEdges = this.api.graph.nodes[
        source
      ].outgoingEdges.filter(({target}) => target !== this.id);
    });
    this.api.graph.nodes[this.id].incomingEdges = [];
  }

  updateOutputId() {}

  makeSerializable(): ApproximateGraphNodeData {
    return {
      abstractNode: {type: 'OperationOutput', id: '@playground'},
      concreteValue: this.value,
    };
  }
}

export class ConcreteGraphAPI extends GraphAPI<ConcreteGraphNode> {
  public graph: ConcreteGraph;
  private nodePool: Record<ConcreteNodeId, ConcreteGraphNode> = {};

  constructor(graph: ConcreteGraph) {
    super();
    this.graph = JSON.parse(JSON.stringify(graph)) as ConcreteGraph;
  }

  protected makeNode(value: ConcreteValue): ConcreteGraphNode {
    const id = (this.graph.nextId++).toString();
    this.graph.nodes[id] = {
      id,
      outgoingEdges: [],
      incomingEdges: [],
      value,
    };
    return new ConcreteGraphNode(id, this);
  }

  public withConcreteArguments<T>(
    args: ConcreteArgument[],
    block: (apiNodes: ConcreteGraphNode[]) => T
  ): T {
    this.beginTemporary();
    const ret = block(
      args.map(n =>
        isLiteralArgument(n)
          ? this.makeTemporaryNode(n.value)
          : this.getNode(n.nodeId)
      )
    );
    this.deleteTemporary();
    return ret;
  }

  getNode(id: ConcreteNodeId) {
    if (this.nodePool[id]) return this.nodePool[id];
    return (this.nodePool[id] = new ConcreteGraphNode(id, this));
  }

  public makeSerializable(): ApproximateGraphData {
    const indices: Record<ConcreteNodeId, number> = {};
    Object.values(this.graph.nodes).forEach((n, i) => (indices[n.id] = i));
    return {
      nodes: Object.values(this.graph.nodes).map(n => ({
        abstractNode: {type: 'OperationOutput', id: n.id},
        concreteValue: n.value,
      })),
      edges: Object.values(this.graph.nodes).flatMap(n =>
        n.outgoingEdges.map(e => ({
          from: indices[n.id],
          to: indices[this.graph.nodes[e.target].id],
        }))
      ),
      queryResults: {},
      customQueryResult: this.queryResult,
    };
  }
}
