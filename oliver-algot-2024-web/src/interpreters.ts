/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {ConcreteGraph} from 'src/ConcreteGraph';
import {isBaseOperation} from 'src/BaseOperations';
import {
  DemoSemanticsInterpreter,
  MaxDepthExceededError,
} from 'src/DemoSemanticsInterpreter';
import {Operation, OperationId} from 'src/Operation';
import {PatternId} from 'src/DemoSemantics';
import {ConcreteArgument} from 'features/tools/Tool';
import {ConcreteGraphAPI} from 'src/ConcreteGraphAPI';
import {GraphAPI, GraphNode} from 'src/GraphAPI';
import patternMatch from 'src/patternMatch';
import {
  ApproximateGraphAPI,
  ApproximateGraphNode,
} from 'src/ApproximateGraphAPI';
import {NUMBER_TYPE_ID, STRING_TYPE_ID} from 'src/ConcreteValue';
import {PatternEditor, fromActionStack} from 'features/editor/editorReducer';
import {NumberTypeError} from './ConcreteGraphAPI';
import {ComparisonOperator} from 'features/editor/PredicateButton';

export type OperationResolver = (id: OperationId) => Operation;

export default function execute(
  operation: OperationId,
  resolveOperation: OperationResolver,
  graph: ConcreteGraph,
  args: ConcreteArgument[]
): ConcreteGraph {
  const graphApi = new ConcreteGraphAPI(graph);

  try {
    graphApi.withConcreteArguments(args, nodes =>
      // eslint-disable-next-line prettier/prettier
      runOperation(resolveOperation, graphApi, operation, nodes, {}, [], 0)
    );
  } catch (e) {
    // console.log('error', e);
    alert(
      `The following error occurred while executing operation ${
        resolveOperation(operation)?.name
      }: ${e}`
    );
    return graph;
  }
  return graphApi.graph;
}

export function evaluate(
  operation: OperationId,
  resolveOperation: OperationResolver,
  graph: ConcreteGraph,
  args: ConcreteArgument[]
): boolean {
  const op = resolveOperation(operation);
  const graphApi = new ConcreteGraphAPI(graph);

  if (!isBaseOperation(op)) {
    try {
      graphApi.withConcreteArguments(args, nodes =>
        // eslint-disable-next-line prettier/prettier
        runOperation(resolveOperation, graphApi, operation, nodes, {}, [], 0)
      );
    } catch (e) {
      // console.log('error', e);
      alert(
        `The following error occurred while executing operation ${
          resolveOperation(operation)?.name
        }: ${e}`
      );
    }
    return graphApi.queryResult;
  } else {
    const result = graphApi.withConcreteArguments(args, nodes =>
      op.perform!(nodes, {
        makeNode: graphApi.makeOutputNode.bind(graphApi),
      })
    );
    if (typeof result === 'boolean') return result;
    return false;
  }
}

export function approximate(
  operation: Operation,
  resolveOperation: OperationResolver,
  exampleValues: Record<PatternId, number | string> | null = null
):
  | {
      graphApi: ApproximateGraphAPI;
      graphData: PatternEditor['approximateGraphs'];
    }
  | {error: string} {
  const graphApi = new ApproximateGraphAPI();

  const graphData = {} as PatternEditor['approximateGraphs'];

  // Build inputs nodes
  const nodeMap: Record<PatternId, ApproximateGraphNode> = {};
  const patterns = Object.values(operation.patterns);

  patterns.forEach(pattern => {
    const node = new ApproximateGraphNode(
      {type: 'PatternMatch', pattern: pattern.id},
      graphApi,
      {
        type:
          typeof exampleValues?.[pattern.id] === 'string'
            ? STRING_TYPE_ID
            : NUMBER_TYPE_ID,
        value: exampleValues?.[pattern.id] ?? 0,
      }
    );
    if (pattern.style) node.style = {...pattern.style};
    nodeMap[pattern.id] = node;
  });
  patterns.forEach(pattern => {
    const node = nodeMap[pattern.id];
    pattern.outgoing.forEach(incoming => {
      if (incoming in nodeMap) {
        node.addEdgeTo(nodeMap[incoming]);
      }
    });
  });

  // put here in case the operation has no actions
  // ensure there still is an approximation
  graphData[fromActionStack([0])] = {
    graph: graphApi.makeSerializable(),
    nextStep: {
      nextStep: 'Noop',
      expandable: false,
    },
  };

  try {
    runOperation(
      resolveOperation,
      graphApi,
      operation.id,
      operation.inputs.map(i => nodeMap[i]),
      graphData,
      [],
      0
    );
  } catch (e) {
    if (e instanceof MaxDepthExceededError) {
      return {error: 'Max Depth exceeded!'};
    } else if (e instanceof NumberTypeError) {
      return {error: 'Number Type expected!'};
    } else {
      throw e;
    }
  }

  return {graphApi, graphData};
}

export function runOperation<T extends GraphNode<T>>(
  resolveOperation: OperationResolver,
  graphApi: GraphAPI<T>,
  id: OperationId,
  nodes: T[],
  graphData: PatternEditor['approximateGraphs'],
  path: number[],
  depth: number
): boolean | ComparisonOperator[] | void {
  const operation = resolveOperation(id);

  if (path.length !== 0) {
    graphData[fromActionStack(path)] = {
      graph: graphApi.makeSerializable(),
      nextStep: {
        nextStep: 'Run',
        expandable: !!operation.demoSemantics,
      },
    };
  }

  function handleNextPath() {
    if (path.length !== 0) {
      const nextPath = fromActionStack([
        ...path.slice(0, -1),
        path[path.length - 1] + 1,
      ]);
      graphData[nextPath] = {
        graph: graphApi.makeSerializable(),
        nextStep: {
          nextStep: 'Noop',
          expandable: false,
        },
      };
    }
  }

  if (depth > 100) {
    throw new MaxDepthExceededError('Maximum depth reached');
  }

  if (isBaseOperation(operation)) {
    const val = operation.perform!(nodes, {
      makeNode: graphApi.makeOutputNode.bind(graphApi),
      setQueryResult: graphApi.setQueryResult.bind(graphApi),
    });
    handleNextPath();
    return val;
  } else if (operation.demoSemantics) {
    const dsi = new DemoSemanticsInterpreter(
      operation,
      (i, n, idx) => {
        runOperation(
          resolveOperation,
          graphApi,
          i,
          n,
          graphData,
          idx === undefined ? [] : [...path, idx],
          depth + 1
        );
      },
      graphApi,
      patternMatch(operation, nodes),
      depth === 0,
      resolveOperation,
      graphData,
      path
    );

    dsi.run(); // may throw an error, bubbles up
    handleNextPath();
  } else {
    throw `Operation semantics of ${operation.name} unknown`;
  }
}
