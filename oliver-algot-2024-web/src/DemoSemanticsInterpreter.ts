/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {
  AbstractNodeDescriptor,
  Action,
  PatternId,
  QueryApplication,
  QueryApplicationId,
} from 'src/DemoSemantics';
import {Operation, OperationId} from 'src/Operation';
import {GraphAPI, GraphNode} from 'src/GraphAPI';
import {isBaseOperation} from 'src/BaseOperations';
import {
  NextStepSummary,
  PatternEditor,
  fromActionStack,
} from 'features/editor/editorReducer';
import {ComparisonOperator} from 'features/editor/PredicateButton';

export type PatternMatches<Node extends GraphNode<Node>> = Record<
  PatternId,
  Node | null
>;

class ConcreteNodeNotPresentError extends Error {
  constructor(public readonly node: AbstractNodeDescriptor) {
    super('Concrete node not present.');
  }
}

export class MaxDepthExceededError extends Error {}

export class DemoSemanticsInterpreter<Node extends GraphNode<Node>> {
  private actionOutputs: Record<string, Node> = {};
  private queryResults: Record<
    QueryApplicationId,
    boolean | ComparisonOperator[]
  > = {};

  constructor(
    private readonly operation: Operation,
    private runOperation: (
      id: OperationId,
      nodes: Node[],
      idx?: number
    ) => void | Node[],
    private readonly graphAPI: GraphAPI<Node>,
    private readonly patternMatches: PatternMatches<Node>,
    private readonly catchMaxDepth: boolean,
    private resolveOperation: (id: OperationId) => Operation,
    private graphData: PatternEditor['approximateGraphs'],
    private path: number[]
  ) {}

  run() {
    this.graphAPI.beginTemporary();

    try {
      this.graphAPI.withRemovalListener(
        node => {
          Object.keys(this.actionOutputs).forEach(k => {
            if (this.actionOutputs[k] === node) delete this.actionOutputs[k];
          });
          Object.entries(this.patternMatches).forEach(([pattern, match]) => {
            if (match === node) this.patternMatches[pattern] = null;
          });
        },
        () => {
          this.operation.demoSemantics!.actions.forEach((a, idx) => {
            this.execute(a, idx);
          });
        }
      );
    } finally {
      this.graphAPI.deleteTemporary();
    }
  }

  private evaluateQuery(qa: QueryApplication) {
    const query = this.resolveOperation(qa.query);
    try {
      const concreteNodes = this.concreteNodesFor(qa.inputs);
      if (qa.inputs.some(i => i.type === 'Undefined')) {
        this.queryResults[qa.id] = false;
        return;
      }
      if (isBaseOperation(query) && query.isQuery) {
        const queryResults = query.perform(concreteNodes as Node[], {
          makeNode: this.graphAPI.makeOutputNode.bind(this.graphAPI),
        });
        if (queryResults !== undefined) {
          this.queryResults[qa.id] = queryResults;
        }
      } else {
        this.runOperation(query.id, concreteNodes as Node[]);
        this.queryResults[qa.id] = this.graphAPI.queryResult;
        this.graphAPI.queryResult = false;
      }
    } catch (e) {
      if (e instanceof ConcreteNodeNotPresentError) {
        this.queryResults[qa.id] = false;
      } else {
        throw e;
      }
    }
  }

  private execute(action: Action, index: number) {
    const executeWrapper = (): NextStepSummary => {
      let inputs: Node[];
      try {
        const tmpInputs = this.concreteNodesFor(action.inputs);
        if (action.inputs.some(i => i.type === 'Undefined')) {
          return {nextStep: 'UnknownInput', expandable: false};
        }
        inputs = tmpInputs as Node[];
      } catch (e) {
        if (e instanceof ConcreteNodeNotPresentError) {
          // One of the inputs doesn't exist. This is fine, we just don't execute the action.
          return {nextStep: 'Noinput', expandable: false};
        } else {
          throw e;
        }
      }

      action.conditions.forEach(condition => {
        this.evaluateQuery(
          this.operation.demoSemantics!.queryApplications[condition.queryApp]
        );
      });

      const condFulfilled = action.conditions.every(condition => {
        const queryResult = this.queryResults[condition.queryApp];
        if (typeof queryResult === 'boolean') {
          return queryResult === condition.result;
        } else {
          return queryResult.includes(condition.result as ComparisonOperator);
        }
      });

      if (!condFulfilled) {
        return {nextStep: 'QFalse', expandable: false};
      }

      this.graphAPI.beginAction(
        {
          action,
          inputs,
          queryResults: this.queryResults,
          outputNames: this.operation.demoSemantics?.outputNames || {},
        },
        index
      );

      this.runOperation(action.operation, inputs, index);
      // may throw an exception, will be bubbled up

      const output = this.graphAPI.endAction();
      Object.assign(this.actionOutputs, output);

      return {nextStep: 'Run', expandable: false};
    };

    this.graphData[fromActionStack([...this.path, index])] = {
      graph: this.graphAPI.makeSerializable(),
      nextStep: {nextStep: 'Noop', expandable: false},
    };

    this.graphData[fromActionStack([...this.path, index])].nextStep =
      executeWrapper();

    const nextPath = fromActionStack([...this.path, index + 1]);
    this.graphData[nextPath] = {
      graph: this.graphAPI.makeSerializable(),
      nextStep: {
        nextStep: 'Noop',
        expandable: false,
      },
    };
  }

  // returns undefined only for nodes of input type 'Undefined',
  // throws ConcreteNodeNotPresentError for non-'Undefined' non-present nodes
  private concreteNodesFor(
    nodes: AbstractNodeDescriptor[]
  ): (Node | undefined)[] {
    return nodes.map(n => {
      if (n.type === 'Undefined') {
        return undefined;
      }
      const concrete = this.concreteNodeFor(n);
      if (concrete === null) throw new ConcreteNodeNotPresentError(n);
      return concrete;
    });
  }

  private concreteNodeFor(node: AbstractNodeDescriptor): Node | null {
    switch (node.type) {
      case 'PatternMatch':
        return this.patternMatches[node.pattern];
      case 'OperationOutput':
        return this.actionOutputs[node.id] || null;
      case 'Literal':
        return this.graphAPI.makeTemporaryNode(node.value);
      case 'Undefined':
        return null;
    }
  }
}
