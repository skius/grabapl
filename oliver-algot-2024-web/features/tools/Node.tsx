/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styles from './Node.module.scss';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {BuiltInTool, executeTool, selectNode} from './toolsReducer';
import {CSSProperties, PropsWithChildren, useEffect, useState} from 'react';
import {GraphToolLabeling, useBuiltInTool} from './hooks';
import classNames from 'classnames';
import {
  Tool,
  ToolArgument,
  isAbstractNodeArgument,
  toolArgumentKey,
} from 'features/tools/Tool';
import {
  ConcreteValue,
  makeValue,
  NUMBER_TYPE,
  renderEditable,
  renderValue,
} from 'src/ConcreteValue';
import {ComputedNodeStyle} from 'features/graphView/computeLayout';
import {Resizable} from 'react-resizable';
import 'react-resizable/css/styles.css';

export enum NodeEvent {
  Click,
  RightClick,
  MouseEnter,
  MouseLeave,
}

function isSelectable(arg: ToolArgument, selectedTool: Tool['id'] | null) {
  if (
    selectedTool === 'changeExampleValue' &&
    !(isAbstractNodeArgument(arg) && arg.abstractNode.type === 'PatternMatch')
  )
    return false;
  return true;
}

/**
 * A node in an abstract or concrete graph. Nodes can be selected. Upon
 * selection the provided `argument` value is added to the selection in the
 * tools reducer.
 */
export default function Node({
  toolLabels,
  argument,
  concreteValue,
  defaultColor,
  highlightNode,
  computedStyle,
  eventHandler,
  children,
  overrideEditHandler,
}: PropsWithChildren<{
  toolLabels: GraphToolLabeling | null;
  argument: ToolArgument;
  concreteValue?: ConcreteValue;
  defaultColor?: string;
  highlightNode?: boolean;
  computedStyle: ComputedNodeStyle;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  eventHandler?: (event: NodeEvent) => void;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  overrideEditHandler?: (value: ConcreteValue) => void;
}>) {
  const argKey = toolArgumentKey(argument);
  const nodeIndices = useAppSelector(state =>
    state.tools.selectedNodes.reduce<number[]>((indices, arg, idx) => {
      if (toolArgumentKey(arg) === argKey) {
        indices.push(idx);
      }
      return indices;
    }, [])
  );

  const isSelected = nodeIndices.length > 0;
  const currentTool = useAppSelector(state => state.tools.selectedTool);
  const builtInTool = useBuiltInTool();
  const toolInUse = useAppSelector(state => state.tools.selectedTool !== null);
  const dispatch = useAppDispatch();

  const [isDragging, setIsDragging] = useState(false);

  const {
    computedWidth,
    computedHeight,
    borderRadius,
    hidden,
    nodeBackground,
    editable,
  } = computedStyle;

  const [width, setWidth] = useState(computedWidth);
  const [height, setHeight] = useState(computedHeight);

  useEffect(() => {
    setWidth(computedWidth);
    setHeight(computedHeight);
  }, [computedWidth, computedHeight]);

  const willEdit =
    !!overrideEditHandler ||
    builtInTool === BuiltInTool.ValueTool ||
    (builtInTool === BuiltInTool.Cursor && editable);

  const forbiddenArgument = !isSelectable(argument, currentTool);

  const letters = 'abcdefghijklmnopqrstuvwxyz';

  const inner = (
    <div
      className={classNames(
        styles.node,
        isSelected && styles.selected,
        !willEdit && toolInUse && styles.toolInUse,
        hidden && styles.hidden,
        highlightNode && styles.highlighted,
        forbiddenArgument && styles.forbiddenArgument,
        isDragging && styles.nodeDragging
      )}
      onClick={() =>
        forbiddenArgument
          ? null
          : willEdit
          ? null
          : dispatch(selectNode(argument))
      }
      style={
        {
          '--node-color': nodeBackground || defaultColor,
          '--node-height': height && `${height}px`,
          '--node-width': width && `${width}px`,
          '--node-border-radius': borderRadius && `${borderRadius}px`,
          color: computedStyle.fontColor,
          fontSize: computedStyle.fontSize,
          textAlign: computedStyle.textAlign,
          '--node-border-width':
            computedStyle.borderWidth && `${computedStyle.borderWidth}px`,
          '--node-border-color': computedStyle.borderColor,
          padding: computedStyle.padding,
        } as CSSProperties
      }
      onContextMenu={e => {
        e.preventDefault();
        if (!toolInUse) eventHandler?.(NodeEvent.RightClick);
      }}
      onMouseEnter={() => {
        eventHandler?.(NodeEvent.MouseEnter);
      }}
      onMouseLeave={() => {
        eventHandler?.(NodeEvent.MouseLeave);
      }}
      onDragStart={e => {
        setIsDragging(true);
        e.dataTransfer.setData('private/algottool', JSON.stringify(argument));
      }}
      onDragEnd={() => setIsDragging(false)}
      draggable={builtInTool === BuiltInTool.Cursor}
    >
      {concreteValue
        ? willEdit
          ? renderEditable(
              concreteValue,
              overrideEditHandler ||
                (value =>
                  dispatch(
                    executeTool({
                      args: [{value}, argument],
                      tool: 'copyValue',
                    })
                  ))
            )
          : renderValue(concreteValue)
        : null}
      {children}
      {!forbiddenArgument && !willEdit && (
        <div className={styles.label}>
          <span>
            {nodeIndices?.map(index => letters.charAt(index)).join(',') ||
              toolLabels?.next}
          </span>
        </div>
      )}
      {builtInTool !== BuiltInTool.Cursor && computedStyle.editable && (
        <div className={styles.editable}>
          <span className="material-icons-outlined">edit</span>
        </div>
      )}
    </div>
  );

  return builtInTool === BuiltInTool.Resizing ? (
    <Resizable
      width={width}
      height={height}
      onResize={(e, {size}) => {
        setHeight(size.height);
        setWidth(size.width);
      }}
      onResizeStop={async (e, {size}) => {
        if (size.width !== computedWidth)
          await dispatch(
            executeTool({
              args: [argument, {value: makeValue(size.width, NUMBER_TYPE)}],
              tool: 'changeWidth',
            })
          );
        if (size.height !== computedHeight)
          await dispatch(
            executeTool({
              args: [argument, {value: makeValue(size.height, NUMBER_TYPE)}],
              tool: 'changeHeight',
            })
          );
      }}
    >
      {inner}
    </Resizable>
  ) : (
    inner
  );
}
