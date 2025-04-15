/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {GraphNodeParams} from 'features/graphView/GraphView';
import {ConcreteNode as ConcreteNodeType} from 'src/ConcreteGraph';
import Node, {NodeEvent} from 'features/tools/Node';
import {NodeStyle} from 'src/NodeStyle';
import {OperationId} from 'src/Operation';
import {executeTool} from 'features/tools/toolsReducer';
import {useAppDispatch} from 'src/hooks';

type KeyOfType<T, V> = keyof {
  [P in keyof T as T[P] extends V ? P : never]: never;
};

const eventStyleProps: Record<
  NodeEvent,
  KeyOfType<NodeStyle, OperationId | undefined>
> = {
  [NodeEvent.Click]: 'onClick',
  [NodeEvent.RightClick]: 'onRightClick',
  [NodeEvent.MouseLeave]: 'onMouseLeave',
  [NodeEvent.MouseEnter]: 'onMouseEnter',
};

export default function ConcreteNode({
  toolLabels,
  payload: node,
  style,
}: GraphNodeParams<ConcreteNodeType>) {
  const dispatch = useAppDispatch();
  return (
    <Node
      concreteValue={node.value}
      toolLabels={toolLabels}
      argument={{nodeId: node.id}}
      computedStyle={style}
      eventHandler={e => {
        const handler = style[eventStyleProps[e]];
        if (handler)
          dispatch(executeTool({args: [{nodeId: node.id}], tool: handler}));
      }}
    />
  );
}
