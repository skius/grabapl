/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styles from './GraphView.module.scss';
import {animated, useSpring} from 'react-spring';
import {ComponentType, SVGProps, useEffect, useMemo, useState} from 'react';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import computeLayout, {ComputedNodeStyle, NODE_SIZE} from './computeLayout';
import {
  GraphToolLabeling,
  useCurrentTool,
  useGraphToolLabeling,
} from 'features/tools/hooks';
import {GraphAdapter} from 'features/graphView/GraphAdapter';
import classNames from 'classnames';
import 'src/OperationTypeView';
import {resetAll} from 'features/tools/toolsReducer';

type LineProps = Omit<SVGProps<SVGLineElement>, 'ref'>;

/**
 * @param graph The graph to show. We rerender if this value changes. Make sure to memoize to avoid unnecessary updates.
 * @param GraphNode Used to render nodes.
 * @param lineProps Used to customize the line representing edges between nodes.
 */
export default function GraphView<Id, Payload>({
  graph,
  GraphNode,
  lineProps,
}: {
  graph: GraphAdapter<Id, Payload>;
  GraphNode: ComponentType<GraphNodeParams<Payload>>;
  lineProps?: (start: Id, end: Id) => LineProps;
}) {
  const drawArrows = useAppSelector(state => state.editor.drawArrows);
  const drawEdges = useAppSelector(state => state.editor.drawEdges);
  const drawEdgeIndex = useAppSelector(state => state.editor.drawEdgeIndex);
  const showHidden = useAppSelector(state => state.editor.showHidden);
  const labeling = useGraphToolLabeling();
  const tool = useCurrentTool();
  const dispatch = useAppDispatch();

  const [sizerDiv, setSizerDiv] = useState<HTMLDivElement | null>(null);
  const [width, setWidth] = useState<number>(1000);

  useEffect(() => {
    function handleResize() {
      setWidth(
        sizerDiv?.parentElement ? sizerDiv?.parentElement.clientWidth : 1000
      );
    }

    handleResize();
    if (sizerDiv) {
      window.addEventListener('resize', handleResize);
      return () => window.removeEventListener('resize', handleResize);
    }
  }, [sizerDiv]);

  const layout = useMemo(
    () => computeLayout(graph, width, showHidden),
    [graph, width, showHidden]
  );

  return (
    <div
      className={styles.sizer}
      style={{
        width: `${layout.width}px`,
        flexBasis: `${layout.height}px`,
      }}
      ref={setSizerDiv}
    >
      <svg className={styles.svg}>
        {drawEdges &&
          Array.from(layout.edges(), edge => (
            <Edge
              {...edge}
              key={`${graph.key(edge.start)} to ${graph.key(edge.end)}`}
              drawIndex={drawEdgeIndex}
              drawArrow={drawArrows}
              lineProps={lineProps?.(edge.start, edge.end)}
            />
          ))}
      </svg>
      <div
        className={classNames(styles.container, tool && styles.toolInUse)}
        style={{
          cursor: tool
            ? `url('/icons/${tool.icon}/materialicons/24px.svg'), crosshair`
            : undefined,
        }}
        onClick={e => {
          if (e.currentTarget !== e.target) return;
          dispatch(resetAll());
        }}
      >
        {Array.from(layout.vertices(), props => (
          <Vertex
            {...props}
            key={graph.key(props.id)}
            payload={graph.payload(props.id)}
            toolLabels={labeling}
            GraphNode={GraphNode}
          />
        ))}
      </div>
    </div>
  );
}

interface Point {
  x: number;
  y: number;
}

const borderRadius = 15;

// based on https://stackoverflow.com/a/50261758/1779477 by Peter Collingridge
function getIntersection(
  dx: number,
  dy: number,
  cx: number,
  cy: number,
  halfNodeSize: number
) {
  const borderLimit = halfNodeSize - borderRadius;
  const [x, y] =
    Math.abs(dy / dx) < 1
      ? // Hit vertical edge of box1
        [
          dx > 0 ? halfNodeSize : -halfNodeSize,
          (dy * halfNodeSize) / Math.abs(dx),
        ]
      : // Hit horizontal edge of box1
        [
          (dx * halfNodeSize) / Math.abs(dy),
          dy > 0 ? halfNodeSize : -halfNodeSize,
        ];
  // taking care of rounded corners:
  if (Math.abs(x) > borderLimit && Math.abs(y) > borderLimit) {
    const [borderX, borderY] = [x, y].map(i => Math.abs(i) - borderLimit);
    const angle = Math.atan(Math.min(borderX, borderY) / borderRadius);
    const trig1 = (angle: number) =>
      borderY < borderX ? Math.cos(angle) : Math.sin(angle);
    const trig2 = (angle: number) =>
      borderY < borderX ? Math.sin(angle) : Math.cos(angle);
    const [trueX, trueY] = [
      borderLimit + trig1(angle) * borderRadius,
      +borderLimit + trig2(angle) * borderRadius,
    ];
    return [cx + (x > 0 ? trueX : -trueX), cy + (y > 0 ? trueY : -trueY)];
  } else {
    return [cx + x, cy + y];
  }
}

function Edge({
  startPoint,
  endPoint,
  index,
  drawIndex,
  drawArrow,
  lineProps,
}: {
  startPoint: Point;
  endPoint: Point;
  index: number;
  drawIndex: boolean;
  drawArrow: boolean;
  lineProps?: LineProps;
}) {
  const halfNodeSize = NODE_SIZE / 2;
  const cx1 = startPoint.x + halfNodeSize;
  const cy1 = startPoint.y + halfNodeSize;
  const cx2 = endPoint.x + halfNodeSize;
  const cy2 = endPoint.y + halfNodeSize;
  const dx = cx2 - cx1;
  const dy = cy2 - cy1;
  const [x1, y1] = getIntersection(dx, dy, cx1, cy1, halfNodeSize);
  const [x2, y2] = getIntersection(-dx, -dy, cx2, cy2, halfNodeSize);

  const pos = useSpring({x1, y1, x2, y2});
  const posLabel = useSpring({x: (x1 + x2) / 2, y: (y1 + y2) / 2});
  const posCircle = useSpring({cx: (x1 + x2) / 2, cy: (y1 + y2) / 2});

  const strokeColor = lineProps?.stroke || '#3f47f4';

  const markerId = `arrow-${startPoint.x}-${startPoint.y}-${endPoint.x}-${endPoint.y}`;

  return (
    <>
      <defs>
        <marker
          id={markerId}
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M 0 0 L 10 5 L 0 10 z" fill={strokeColor} />
        </marker>
      </defs>
      <animated.line
        stroke={strokeColor}
        markerEnd={drawArrow ? `url(#${markerId})` : undefined}
        {...lineProps}
        {...pos}
      />
      {drawIndex && (
        <>
          <animated.circle {...posCircle} r="8" fill="var(--hover-gray)" />
          <animated.text
            {...posLabel}
            font-size="11"
            color="var(--text-gray)"
            dominant-baseline="middle"
            text-anchor="middle"
          >
            {index}
          </animated.text>
        </>
      )}
    </>
  );
}

export interface GraphNodeParams<Payload> {
  toolLabels: GraphToolLabeling | null;
  payload: Payload;
  style: ComputedNodeStyle;
}

function Vertex<Payload>({
  x,
  y,
  GraphNode,
  ...graphNodeProps
}: {
  x: number;
  y: number;
  GraphNode: ComponentType<GraphNodeParams<Payload>>;
} & GraphNodeParams<Payload>) {
  const pos = useSpring({left: x, top: y});
  return (
    <animated.div style={{position: 'absolute', ...pos}}>
      <GraphNode {...graphNodeProps} />
    </animated.div>
  );
}
