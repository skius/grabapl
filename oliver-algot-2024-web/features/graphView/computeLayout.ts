/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import {hierarchy, HierarchyNode, HierarchyPointNode, tree} from 'd3-hierarchy';
import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
} from 'd3-force';
import {GraphAdapter} from './GraphAdapter';
import {computeFlex} from './computeFlex';
import {NodeStyle} from 'src/NodeStyle';

interface Point {
  x: number;
  y: number;
}

interface EdgeLayout<Id> {
  start: Id;
  end: Id;
  index: number;
  startPoint: Point;
  endPoint: Point;
}

export interface VertexLayout<Id> {
  x: number;
  y: number;
  id: Id;
  style: ComputedNodeStyle;
}

export interface ComputedNodeStyle extends NodeStyle {
  computedWidth: number;
  computedHeight: number;
}

interface PlacedLayout<Id> {
  vertices(): Generator<VertexLayout<Id>>;
  edges(): Generator<EdgeLayout<Id>>;
}

interface Layout<Id> extends PlacedLayout<Id> {
  width: number;
  height: number;
}

export interface LayoutComputation<Id> {
  boundingBox: {w: number; h: number};
  place(
    width: number,
    height: number,
    absolute: (x: number, y: number) => Point
  ): PlacedLayout<Id>;
}

export const NODE_SIZE = 60;
export const BOUNDING_BOX_PADDING = 160;

function boundingBox(points: Iterable<Point>) {
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;

  for (const point of points) {
    if (point.x < minX) minX = point.x;
    if (point.y < minY) minY = point.y;
    if (point.x > maxX) maxX = point.x;
    if (point.y > maxY) maxY = point.y;
  }

  maxX += NODE_SIZE;
  maxY += NODE_SIZE;

  return {minX, maxX, minY, maxY};
}

function directionIsVertical(direction: LayoutDirection) {
  return direction === LayoutDirection.Down || direction === LayoutDirection.Up;
}

export default function computeLayout<Id>(
  graph: GraphAdapter<Id, unknown>,
  playgroundWidth: number,
  showHidden: boolean
): Layout<Id> {
  const components = findComponents(graph);

  const layouts = components.map(component => {
    const layout =
      component.root !== null
        ? graph.style(component.root)?.flexDirection
          ? computeFlex(graph, component, showHidden)
          : computeTree(graph, component, false, LayoutDirection.Down)
        : component.inverseRoot !== null
        ? computeTree(graph, component, true, LayoutDirection.Up)
        : computeForce(graph, component);
    return {
      layout,
      width: layout.boundingBox.w + BOUNDING_BOX_PADDING,
      height: layout.boundingBox.h + BOUNDING_BOX_PADDING,
      x: 0,
      y: 0,
    };
  });

  const {width, height} = pack(layouts, playgroundWidth);

  const placedLayouts = layouts.map(({layout, width, height, x, y}) =>
    layout.place(width, height, (dx, dy) => ({x: x + dx, y: y + dy}))
  );

  return {
    width,
    height,
    *vertices() {
      for (const l of placedLayouts) yield* l.vertices();
    },
    *edges() {
      for (const l of placedLayouts) yield* l.edges();
    },
  };
}

interface Bounds {
  width: number;
  height: number;
  x: number;
  y: number;
}

function pack(boxes: Bounds[], playgroundWidth: number) {
  const rowWidth = Math.max(
    playgroundWidth,
    Math.max(...boxes.map(({width}) => width))
  );

  function maxHeight(from: number, n: number) {
    let max = 0;
    for (let i = 0; i < n; i++) max = Math.max(boxes[from + i].height, max);
    return max;
  }

  function row(
    start: number,
    placeAtLeast: number,
    absY: number
  ): {consumed: number; rowHeight: number} {
    const rowHeight = maxHeight(start, placeAtLeast);
    let currentX = 0;
    let currentY = 0;
    let currentColWidth = boxes[start].width;
    let consumed = 0;

    for (let i = start; i < boxes.length; i++) {
      const box = boxes[i];

      if (currentY + box.height > rowHeight || box.width > currentColWidth) {
        if (consumed === 0)
          throw 'First box must fit first column. Invariant violated.';

        // then create the next column
        currentX += currentColWidth;
        currentY = 0;
        currentColWidth = box.width;

        if (currentX + currentColWidth > rowWidth)
          // Oh, shoot! The new column doesn't fit! We're done.
          break;
      }

      consumed++;

      if (box.height > rowHeight)
        // The column is okay, but the box is higher than the row!
        // Recalculate entire row.
        return row(start, consumed, absY);

      box.x = currentX;
      box.y = absY + currentY;
      currentY += box.height;
    }

    return {consumed, rowHeight};
  }

  function buildRows() {
    let consumed = 0;
    let totalHeight = 0;
    while (consumed < boxes.length) {
      const r = row(consumed, 1, totalHeight);
      consumed += r.consumed;
      totalHeight += r.rowHeight;
    }
    return {width: rowWidth, height: totalHeight};
  }

  return buildRows();
}

export interface Component<Id> {
  nodes: Id[];
  /// root !== null ==> the component forms a tree
  root: null | Id;
  inverseRoot: null | Id;
}

function findComponents<Id>(graph: GraphAdapter<Id, unknown>): Component<Id>[] {
  const visited = new Set<Id>();
  const result: Component<Id>[] = [];

  function exploreComponent(u: Id, component: Component<Id>) {
    if (visited.has(u)) return;
    visited.add(u);

    component.nodes.push(u);

    const incomingEdges = graph.incomingEdges(u);
    const outgoingEdges = graph.outgoingEdges(u);

    if (incomingEdges.length === 0) {
      // Note that each tree Component can only have one node with this property.
      component.root = u;
    }

    if (outgoingEdges.length === 0) {
      component.inverseRoot = u;
    }

    outgoingEdges.forEach(v => exploreComponent(v, component));
    incomingEdges.forEach(v => exploreComponent(v, component));
  }

  function checkTree(component: Component<Id>, inverse: boolean) {
    const visitedTree = new Set<Id>();

    function checkTreeDfs(u: Id): boolean {
      if (visitedTree.has(u)) return false;

      visitedTree.add(u);
      return (inverse ? graph.incomingEdges(u) : graph.outgoingEdges(u)).every(
        v => checkTreeDfs(v)
      );
    }

    return (
      checkTreeDfs(inverse ? component.inverseRoot! : component.root!) &&
      component.nodes.every(id => visitedTree.has(id))
    );
  }

  for (const u of graph.nodes) {
    if (visited.has(u)) continue;

    const component: Component<Id> = {nodes: [], root: null, inverseRoot: null};
    exploreComponent(u, component);
    if (component.root !== null && !checkTree(component, false)) {
      component.root = null;
    }
    if (component.inverseRoot !== null && !checkTree(component, true)) {
      component.inverseRoot = null;
    }
    result.push(component);
  }
  return result;
}

export const enum LayoutDirection {
  Left = 'LEFT',
  Right = 'RIGHT',
  Up = 'UP',
  Down = 'DOWN',
}

function computeTree<Id>(
  graph: GraphAdapter<Id, unknown>,
  component: Component<Id>,
  inverse: boolean,
  direction: LayoutDirection
): LayoutComputation<Id> {
  const root: HierarchyNode<Id> = inverse
    ? hierarchy(
        component.inverseRoot!,
        node => graph.incomingEdges(node) as Iterable<NonNullable<Id>>
      )
    : hierarchy(
        component.root!,
        node => graph.outgoingEdges(node) as Iterable<NonNullable<Id>>
      );
  tree()
    .nodeSize([80, 100])
    .separation((a, b) => (a.parent === b.parent ? 1.5 : 1))(root);

  const {maxY, maxX, minY, minX} = boundingBox(
    root.descendants() as HierarchyPointNode<Id>[]
  );
  const height = Math.abs(maxY - minY);
  const width = 2 * Math.max(maxX, Math.abs(minX));

  return {
    boundingBox: directionIsVertical(direction)
      ? {h: height, w: width}
      : {h: width, w: height},
    place(width, height, absolute) {
      function transformPoint(node: HierarchyPointNode<unknown>) {
        switch (direction) {
          case LayoutDirection.Down:
            return absolute(node.x + width / 2, node.y + 100);
          case LayoutDirection.Right:
            return absolute(node.y + 60, node.x + height / 2 - 100);
          case LayoutDirection.Left:
            return absolute(width - (node.y + 100), node.x + height / 2 - 100);
          case LayoutDirection.Up:
            return absolute(node.x + width / 2, height - (node.y + 100));
        }
      }

      return {
        *vertices() {
          for (const n of root.descendants()) {
            const node = n as HierarchyPointNode<Id>;
            yield {
              ...transformPoint(node),
              id: node.data,
              style: {
                ...graph.style(n.data),
                computedWidth: NODE_SIZE,
                computedHeight: NODE_SIZE,
              },
            };
          }
        },
        *edges() {
          function* traverse(
            node: HierarchyPointNode<Id>
          ): Generator<EdgeLayout<Id>> {
            if (!node.children) return;
            for (const [index, child] of node.children.entries()) {
              yield {
                index,
                startPoint: inverse
                  ? transformPoint(child)
                  : transformPoint(node),
                endPoint: inverse
                  ? transformPoint(node)
                  : transformPoint(child),
                start: node.data,
                end: child.data,
              };
              yield* traverse(child);
            }
          }
          yield* traverse(root as HierarchyPointNode<Id>);
        },
      };
    },
  };
}

function computeForce<Id>(
  graph: GraphAdapter<Id, unknown>,
  component: Component<Id>
): LayoutComputation<Id> {
  type ForceNode = (typeof nodes)[0];
  interface Link {
    idx: number;
    source: string | ForceNode;
    target: string | ForceNode;
  }

  const nodes = component.nodes.map(node => ({
    id: graph.key(node),
    node,
    x: 0,
    y: 0,
  }));
  const links: Link[] = component.nodes.flatMap(source =>
    graph.outgoingEdges(source).map((target, index) => ({
      idx: index,
      source: graph.key(source),
      target: graph.key(target),
    }))
  );

  const link = forceLink<ForceNode, Link>(links)
    .id(n => n.id)
    .distance(100);

  forceSimulation(nodes)
    .force('link', link)
    .force('charge', forceManyBody())
    .force('center', forceCenter())
    .force('collide', forceCollide(43))
    .tick(300);

  const {maxY, maxX, minY, minX} = boundingBox(nodes);

  return {
    boundingBox: {w: Math.abs(maxX - minX), h: Math.abs(maxY - minY)},
    place(width, height, absolute) {
      return {
        *vertices() {
          for (const node of nodes) {
            yield {
              ...absolute(node.x + width / 2, node.y + height / 2),
              id: node.node,
              style: {
                ...graph.style(node.node),
                computedWidth: NODE_SIZE,
                computedHeight: NODE_SIZE,
              },
            };
          }
        },
        *edges() {
          for (const link of links) {
            const s = link.source as ForceNode,
              t = link.target as ForceNode;
            yield {
              index: link.idx,
              startPoint: absolute(s.x + width / 2, s.y + height / 2),
              endPoint: absolute(t.x + width / 2, t.y + height / 2),
              start: s.node,
              end: t.node,
            };
          }
        },
      };
    },
  };
}
