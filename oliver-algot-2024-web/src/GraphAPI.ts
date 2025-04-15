/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {ConcreteValue, Type} from 'src/ConcreteValue';
import {NodeStyle} from 'src/NodeStyle';
import {
  Action,
  ActionId,
  QueryApplicationId,
  fromOutputKey,
  toOutputKey,
} from 'src/DemoSemantics';
import {
  ApproximateGraphData,
  ApproximateGraphNodeData,
} from './ApproximateGraphAPI';
import {ComparisonOperator} from 'features/editor/PredicateButton';

export interface GraphNode<N> {
  get neighbors(): N[];
  get nodesWithIncomingEdges(): N[];
  remove(): void;
  addEdgeTo(node: N): void;
  hasEdgeTo(node: N): boolean;
  removeEdges(): void;
  get hasNeighbors(): boolean;
  setStyle<K extends keyof NodeStyle>(key: K, value: NodeStyle[K]): void;
  payload<P>(type: Type<P>): P;
  get value(): ConcreteValue;
  set value(v: ConcreteValue | number | string);
  get numberValue(): number;
  get stringValue(): string;
  updateOutputId(outputId: string): void;
  makeSerializable(): ApproximateGraphNodeData;
}

type RemovalListener<N> = (node: N) => void;

export interface StackFrame<Node> {
  outputNodes: Record<string, Node>;
  actionIndex: number;
  actionId: ActionId;
  outputNames: Record<ActionId, string>;
}

export interface BeginActionInfo<Node> {
  action: Action;
  outputNames: Record<ActionId, string>;
  queryResults: Record<
    QueryApplicationId,
    boolean | ComparisonOperator[] | undefined
  >;
  inputs: Node[];
}

export abstract class GraphAPI<Node extends GraphNode<Node>> {
  public removalListeners: RemovalListener<Node>[] = [];
  private temporaryStack: Node[][] = [];
  private outputStack: StackFrame<Node>[] = [];
  public queryResult: boolean = false;

  public withRemovalListener(
    onRemoval: RemovalListener<Node>,
    block: () => void
  ) {
    this.removalListeners.push(onRemoval);
    try {
      block();
    } finally {
      this.removalListeners.pop();
    }
  }

  public setQueryResult(result: boolean) {
    this.queryResult = result;
  }

  public beginTemporary() {
    this.temporaryStack.push([]);
  }

  public deleteTemporary() {
    this.temporaryStack.pop()!.forEach(node => node.remove());
  }

  /**
   * Informs the receiver that an action will be executed, i.e. another operation
   * will be interpreted. It starts collecting the output nodes of the action.
   */
  public beginAction(info: BeginActionInfo<Node>, actionIndex: number): void {
    this.outputStack.push({
      outputNodes: {},
      actionIndex,
      actionId: info.action.id,
      outputNames: info.outputNames,
    });
  }

  get currentFrame(): StackFrame<Node> | undefined {
    return this.outputStack[this.outputStack.length - 1];
  }

  /**
   * Counterpart to beginAction. Returns the output nodes of the action.
   */
  public endAction(): Record<string, Node> {
    const frame = this.outputStack.pop()!;
    if (this.currentFrame) {
      for (const [id, node] of Object.entries(frame.outputNodes)) {
        const ids = [this.currentFrame.actionId, ...fromOutputKey(id)];
        const newId = toOutputKey(ids as [...ActionId[], string]);
        this.currentFrame.outputNodes[newId] = node;
        node.updateOutputId(newId);
      }
    }
    return frame.outputNodes;
  }

  /**
   * Creates a node that represents the output of an operation. It will be
   * collected and returned by endAction.
   * @param value
   */
  public makeOutputNode(value: ConcreteValue): Node {
    if (this.currentFrame) {
      // Important: We assume that each base operation can only create one
      // output node. Otherwise, we need to count the output nodes and add the
      // index to the id here (only here).
      const outputNodeName =
        this.currentFrame.outputNames[this.currentFrame.actionId];
      const fullId = toOutputKey([this.currentFrame.actionId, outputNodeName]);
      if (outputNodeName === undefined) {
        throw 'das not good';
      }
      // const id = this.currentFrame.actionIndex.toString();
      // const id = this.capitalLetters[this.currentFrame.actionIndex];
      const node = this.makeNode(value, false, fullId);
      this.currentFrame.outputNodes[fullId] = node;
      return node;
    }

    // If we are executing a base operation in the state view, there
    // is no output stack.
    return this.makeNode(value, false, '');
  }

  protected abstract makeNode(
    value: ConcreteValue,
    temporary: boolean,
    id: string
  ): Node;

  public makeTemporaryNode(value: ConcreteValue): Node {
    const node = this.makeNode(value, true, 'temporary');
    this.temporaryStack[this.temporaryStack.length - 1].push(node);
    return node;
  }

  public abstract makeSerializable(): ApproximateGraphData;
}
