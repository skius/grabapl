/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styles from 'features/editor/AbstractNode.module.scss';
import Node from 'features/tools/Node';
import useOuD from 'features/editor/useOuD';
import {GraphNodeParams} from 'features/graphView/GraphView';
import {ApproximateGraphNode} from 'src/ApproximateGraphAPI';
import {AbstractNodeDescriptor} from 'src/DemoSemantics';
import {keyForAbstractNode, nameForAbstractNode} from 'src/AbstractNodeUtils';
import {useAppSelector} from 'src/hooks';
import {Operation} from 'src/Operation';

export function getColorForAbstractNode(
  abstractNode: AbstractNodeDescriptor,
  operation: Operation
): string {
  if (abstractNode.type === 'PatternMatch') {
    if (operation.inputs.includes(abstractNode.pattern)) return 'var(--blue)';
    else return 'var(--pattern-match)';
  } else if (abstractNode.type === 'OperationOutput') return 'var(--orange)';
  return '#ff0000';
}

export default function AbstractNode({
  payload: approximateNode,
  toolLabels,
  style,
}: GraphNodeParams<ApproximateGraphNode>) {
  const operation = useOuD();

  if (!operation) return null;

  const highlight = useAppSelector(state => {
    if (state.editor.highlightedNodes)
      return state.editor.highlightedNodes[
        keyForAbstractNode(approximateNode.abstractNode)
      ];
    else return undefined;
  });

  const name = nameForAbstractNode(approximateNode.abstractNode, operation);

  const abstractNode = approximateNode.abstractNode;

  const color = getColorForAbstractNode(abstractNode, operation);

  // const color =
  //   abstractNode.type === 'PatternMatch'
  //     ? operation.inputs.includes(abstractNode.pattern)
  //       ? 'var(--blue)'
  //       : 'var(--pattern-match)'
  //     : abstractNode.type === 'OperationOutput'
  //     ? 'var(--orange)'
  //     : undefined;

  return (
    <div className={styles.abstractNode}>
      <Node
        toolLabels={toolLabels}
        argument={{abstractNode}}
        computedStyle={style}
        concreteValue={approximateNode.value}
        highlightNode={!!highlight}
        defaultColor={/*highlight ? 'var(--light-orange)' : */ color}
      ></Node>
      {/* consider having a different style if abstractNode.type === 'PatternMatch'*/}
      <div className={styles.someValue} style={{color}}>
        {name}
      </div>
    </div>
  );
}
