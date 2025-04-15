/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {GraphAdapter} from 'features/graphView/GraphAdapter';
import Yoga from 'yoga-layout-prebuilt';
import {
  BOUNDING_BOX_PADDING,
  Component,
  LayoutComputation,
  VertexLayout,
} from 'features/graphView/computeLayout';

const ALIGN_MAP = Object.freeze({
  start: Yoga.ALIGN_FLEX_START,
  end: Yoga.ALIGN_FLEX_END,
  center: Yoga.ALIGN_CENTER,
  stretch: Yoga.ALIGN_STRETCH,
  'space-between': Yoga.ALIGN_SPACE_BETWEEN,
  'space-around': Yoga.ALIGN_SPACE_AROUND,
});

function numberOrDefault(val: number | null | undefined, def: number): number {
  if (val !== null && val !== undefined) return val;
  return def;
}

/**
 * Computes a map that assigns each node a LayoutSettings object that
 * describes the layout settings that apply to it in the tree graph.
 */
export function computeFlex<Id>(
  graph: GraphAdapter<Id, unknown>,
  component: Component<Id>,
  showHidden: boolean
): LayoutComputation<Id> {
  const yogaMap = new Map<Id, Yoga.YogaNode>();

  function walk(node: Id, yogaParent: Yoga.YogaNode | null, index: number) {
    const style = graph.style(node);
    if (style?.hidden && !showHidden) return false;

    const yogaNode = Yoga.Node.create();
    yogaMap.set(node, yogaNode);

    if (yogaParent) yogaParent.insertChild(yogaNode, index);

    const edges = graph.outgoingEdges(node);

    if (Number.isFinite(style?.nodeWidth) || edges.length === 0)
      yogaNode.setWidth(style?.nodeWidth || 60);
    if (Number.isFinite(style?.nodeHeight) || edges.length === 0)
      yogaNode.setHeight(style?.nodeHeight || 60);

    yogaNode.setPadding(Yoga.EDGE_ALL, numberOrDefault(style?.padding, 12));
    if (Number.isFinite(style?.margin))
      yogaNode.setMargin(Yoga.EDGE_ALL, style!.margin!);
    if (Number.isFinite(style?.marginLeft))
      yogaNode.setMargin(Yoga.EDGE_LEFT, style!.marginLeft!);
    if (Number.isFinite(style?.marginTop))
      yogaNode.setMargin(Yoga.EDGE_TOP, style!.marginTop!);

    if (style?.flexDirection === 'row')
      yogaNode.setFlexDirection(Yoga.FLEX_DIRECTION_ROW);
    if (style?.flexDirection === 'column')
      yogaNode.setFlexDirection(Yoga.FLEX_DIRECTION_COLUMN);
    if (style?.flexWrap === 'wrap') yogaNode.setFlexWrap(Yoga.WRAP_WRAP);
    if (style?.flexWrap === 'wrap-reverse')
      yogaNode.setFlexWrap(Yoga.WRAP_WRAP_REVERSE);

    if (style?.flexAlign) yogaNode.setAlignItems(ALIGN_MAP[style.flexAlign]);

    edges.reduce((i, n) => (walk(n, yogaNode, i) ? i + 1 : i), 0);
    return true;
  }

  walk(component.root!, null, 0);
  const rootYogaNode = yogaMap.get(component.root!)!;
  rootYogaNode.calculateLayout();

  return {
    boundingBox: {
      w: rootYogaNode.getComputedWidth(),
      h: rootYogaNode.getComputedHeight(),
    },
    place(width, height, absolute) {
      return {
        *vertices() {
          function* put(
            id: Id,
            parentX: number,
            parentY: number
          ): Generator<VertexLayout<Id>> {
            const yogaNode = yogaMap.get(id);
            if (!yogaNode) return;
            const x = yogaNode.getComputedLeft() + parentX;
            const y = yogaNode.getComputedTop() + parentY;
            yield {
              ...absolute(x + BOUNDING_BOX_PADDING, y + BOUNDING_BOX_PADDING),
              id: id,
              style: {
                ...graph.style(id),
                computedHeight: yogaNode.getComputedHeight(),
                computedWidth: yogaNode.getComputedWidth(),
              },
            };

            for (const child of graph.outgoingEdges(id)) {
              yield* put(child, x, y);
            }
          }

          yield* put(component.root!, 0, 0);
        },
        *edges() {},
      };
    },
  };
}
