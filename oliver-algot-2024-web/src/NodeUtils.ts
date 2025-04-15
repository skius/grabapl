type AllNodes = {
  [key: string]: {
    id: string;
    outgoingEdges: Array<{source: string; target: string}>;
    incomingEdges: Array<{source: string; target: string}>;
    value: {type: string; value: number};
  };
};

type NodeInfo = {
  name: string;
  id: string;
  outgoing: string[];
  incoming: string[];
  value: number;
};

export function dfs(
  node: AllNodes[string],
  visited: {[key: string]: boolean},
  allNodes: AllNodes,
  result: Array<AllNodes[string] & {selected: boolean}>
) {
  visited[node.id] = true;
  const isSelected = result.some(n => n.id === node.id);
  if (!isSelected) {
    result.push({...node, selected: false});
  }

  for (const edge of node.outgoingEdges) {
    if (!visited[edge.target]) {
      dfs(allNodes[edge.target], visited, allNodes, result);
    }
  }
  for (const edge of node.incomingEdges) {
    if (!visited[edge.source]) {
      dfs(allNodes[edge.source], visited, allNodes, result);
    }
  }
}

export function findWeaklyConnectedComponents(
  selectedNodes: Array<AllNodes[string]>,
  allNodes: AllNodes
) {
  const visited: {[key: string]: boolean} = {};
  const result: Array<AllNodes[string] & {selected: boolean}> = [];
  const components: {inputs: string[]; patterns: {[key: string]: NodeInfo}} = {
    inputs: selectedNodes.map(node => node.id),
    patterns: {},
  };

  for (const node of selectedNodes) {
    if (!visited[node.id]) {
      dfs(node, visited, allNodes, result);
    }
  }

  for (const node of result) {
    components.patterns[node.id] = {
      name: node.id,
      id: node.id,
      outgoing: node.outgoingEdges.map(edge => edge.target),
      incoming: node.incomingEdges.map(edge => edge.source),
      value: node.value['value'],
    };
  }

  return components;
}
