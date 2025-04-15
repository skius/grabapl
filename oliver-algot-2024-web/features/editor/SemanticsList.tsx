/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styles from './SemanticsList.module.scss';
import {keyForAbstractNode, nameForAbstractNode} from 'src/AbstractNodeUtils';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import styled from 'styled-components';
import {
  editorChangeCalledOperation,
  editorChangeInputNode,
  editorDeleteSpecificAction,
  editorExpand,
  editorReorderActions,
  editorSetHoveredActionStack,
  editorStepTo,
  editorStepToStart,
  fromActionIdStack,
  fromActionStack,
  getCurrentEditor,
  PatternEditor,
  toActionStack,
} from 'features/editor/editorReducer';
import {useResolveOperation} from 'features/editor/operationHooks';
import IconButton from 'components/IconButton';
import {Operation, OperationId} from 'src/Operation';
import baseOperations, {GeneralFunc, isBaseOperation} from 'src/BaseOperations';
import React, {useEffect, useRef, useState} from 'react';
import {N} from 'src/BaseOperations';
import {
  AbstractNodeDescriptor,
  ActionId,
  fromOutputKey,
  toOutputKey,
} from 'src/DemoSemantics';
import {Disclosure, Transition} from '@headlessui/react';
import {ArraySet} from 'src/ArraySet';
import {DragDropContext, Draggable, Droppable} from 'react-beautiful-dnd';
import {ToolArgument, ToolId, isAbstractNodeArgument} from 'features/tools/Tool';
import { strForComp } from './PredicateButton';

const ActionField = styled.div<{
  type: 'Operation' | 'Query' | 'Template';
  indent: number;
  debug: boolean;
  isSemantics: boolean;
  doNotExecute: boolean;
  current: boolean;
}>`
  display: flex;
  font-size: ${({isSemantics}) =>
    isSemantics ? 'var(--font-xs)' : 'var(--font-xxs)'};
  align-items: center;
  padding: 8px 3px;
  background-color: ${({current}) =>
    current ? 'var(--selected)' : 'var(--off-white)'};

  padding-left: ${({indent, isSemantics}) =>
    (indent + (isSemantics ? 0 : 1)) * 20 + 10}px;

  :hover {
    background-color: ${({current}) =>
      current ? 'var(--selected)' : 'var(--hovered)'};
  }

  border-radius: 5px;

  cursor: pointer;

  button {
    margin-left: auto;
    align-self: flex-start;
  }

  .icon,
  .iconYes,
  .iconNo {
    background-color: ${({type, isSemantics, doNotExecute}) =>
      doNotExecute
        ? 'var(--text-gray)'
        : type === 'Query'
        ? 'var(--query-light)'
        : type === 'Template'
        ? 'var(--template-light)'
        : isSemantics
        ? 'var(--action-light)'
        : 'var(--debug-edit)'};
    border-radius: 50%;
    color: ${({type, isSemantics, doNotExecute}) =>
      doNotExecute
        ? 'var(--off-black)'
        : type === 'Query'
        ? 'var(--query)'
        : type === 'Template'
        ? 'var(--template)'
        : isSemantics
        ? 'var(--action)'
        : 'var(--debug-edit-action)'};
    padding: 5px;
    margin-right: 15px;
    width: ${({isSemantics}) => (isSemantics ? '28px' : '24px')};
    height: ${({isSemantics}) => (isSemantics ? '28px' : '24px')};

    .material-icons-outlined {
      font-size: ${({isSemantics}) =>
        isSemantics ? 'var(--font-md)' : 'var(--font-sm)'};
    }
  }

  .iconYes {
    background-color: var(--green);
    color: var(--light-green);
  }

  .iconNo {
    background-color: var(--red);
    color: var(--light-orange);
  }

  .text {
    line-height: 1.3;
    color: ${({isSemantics, doNotExecute}) =>
      doNotExecute || !isSemantics ? 'gray' : 'black'};
  }

  .outermostIcon {
    color: black;
    border-radius: 50%;
    padding: 5px;
    margin-right: 15px;
    width: 28px;
    height: 28px;

    display: flex;
    align-items: center;

    .material-icons-outlined {
      font-size: var(--font-lg);
    }
  }

  .title {
    color: black;
    font-size: var(--font-sm);
  }

  .details {
    color: ${({doNotExecute}) => (doNotExecute ? 'gray' : 'var(--off-black)')};
  }

  .opDesc {
    font-weight: 500;
  }

  .outDesc {
    fond-weight: 200;
    font-size: 90%;
    color: var(--text-gray);
    margin-top: 3px;
  }

  .buttons {
    margin-left: auto;
    display: flex;
    flex-direction: row;
  }
`;

export function standardInstruction<T>(op: Operation) {
  return (nodes: T[]) =>
    ([op.name, ' on '] as (string | T)[]).concat(
      nodes
        .map(n => [n, ', '])
        .flat()
        .slice(0, -1)
    );
}

function addBetween<T, U>(l: T[], e: U) {
  return l.flatMap(x => [x, e]).slice(0, -1);
}

function updateNamingContext(
  node: AbstractNodeDescriptor,
  context: ActionId[]
): AbstractNodeDescriptor {
  switch (node.type) {
    case 'OperationOutput': {
      const names = fromOutputKey(node.id);
      return {...node, id: toOutputKey([...context, ...names])};
    }
    default:
      return node;
  }
}

export function mapNode(
  node: AbstractNodeDescriptor,
  patternMatches: PatternEditor['patternMatches'],
  path: number[],
  actionIdPath: ActionId[]
) {
  const workingIndex = Object.keys(patternMatches).find(
    k =>
      fromActionStack(toActionStack(k).slice(0, -1)) === fromActionStack(path)
  );
  const matchers = workingIndex ? patternMatches[workingIndex] : {};
  const mappedNode =
    node.type === 'OperationOutput'
      ? updateNamingContext(node, actionIdPath.slice(0, -1))
      : node.type === 'PatternMatch'
      ? matchers[node.pattern]
      : node;
  return mappedNode as typeof mappedNode | undefined;
}

export default function SemanticsList({
  operation: outerOperation,
}: {
  operation: Operation;
}) {
  const dispatch = useAppDispatch();
  const resolveOperation = useResolveOperation();
  const patternMatches = useAppSelector(
    state => getCurrentEditor(state.editor).patternMatches
  );

  const {actionStack, expandedActions, approximateGraphs, openPaths} =
    useAppSelector(state => getCurrentEditor(state.editor));

  const allOpenPaths = openPaths;
  const currentPath =
    allOpenPaths[allOpenPaths.indexOf(fromActionStack(actionStack)) - 1];

  const currentBarRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    currentBarRef.current?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    });
  }, [actionStack]);

  function Semantics({
    operation,
    indent,
    actionStack,
    path,
    actionIdPath,
    expandedActions,
    allPaths,
  }: {
    operation: Operation;
    indent: number;
    actionStack: number[];
    path: number[];
    actionIdPath: ActionId[];
    expandedActions: string[];
    allPaths: string[];
  }) {
    const mapNode2 = (n: AbstractNodeDescriptor) =>
      mapNode(n, patternMatches, path, actionIdPath);

    const TextNodeName = ({
      node,
      doExecute,
      changeInputHandler,
    }: {
      node: AbstractNodeDescriptor;
      doExecute: boolean;
      changeInputHandler?: (node: AbstractNodeDescriptor) => void;
    }) => {
      const [isDraggingOn, setIsDraggingOn] = useState(false);

      const mappedNode = mapNode2(node);
      return (
        <N
          ankey={mappedNode && keyForAbstractNode(mappedNode)}
          inexistent={!mappedNode}
          normalText={node.type === 'Literal'}
          boldText={!doExecute && node.type !== 'Literal'}
          onDragEnter={() => setIsDraggingOn(true)}
          onDragOver={e => e.preventDefault()} // need this to make onDrop work
          onDragLeave={() => setIsDraggingOn(false)}
          onDrop={e => {
            if (!changeInputHandler) return;
            const newnode_str = e.dataTransfer.getData('private/algottool');
            if (newnode_str.length > 0) {
              const toolArgument = JSON.parse(newnode_str) as ToolArgument;
              if (isAbstractNodeArgument(toolArgument)) {
                changeInputHandler(toolArgument.abstractNode);
              }
            }
            setIsDraggingOn(false);
          }}
          style={{
            borderRadius: '30%',
            backgroundColor: isDraggingOn
              ? 'var(--interact-tint-lighter)'
              : undefined,
          }}
        >
          {mappedNode
            ? nameForAbstractNode(mappedNode, outerOperation)
            : nameForAbstractNode(node, operation)}
        </N>
      );
    };

    const enableReordering = path.length === 0;

    const elements = operation.demoSemantics!.actions.map((a, i) => {
      if (!allPaths.includes(fromActionStack([i]))) return <></>;
      const allNextPaths = allPaths
        .filter(x => x.startsWith(fromActionStack([i]) + '.'))
        .map(x => x.split('.').slice(1).join('.'));

      const nextActionStack = fromActionStack([...path, i]);
      const nextActionIdStack = fromActionIdStack([...actionIdPath, a.id]);
      if (!(nextActionStack in approximateGraphs)) return <></>;

      const {
        graph: {queryResults},
        nextStep: {nextStep, expandable},
      } = approximateGraphs[nextActionStack];
      const doExecute =
        nextStep === 'Run' && a.inputs.every(x => x?.type !== 'Undefined');

      if (nextStep === 'Noinput' || nextStep === 'Noop') {
        return <></>;
      }

      const doDisclosure = expandable && doExecute;
      const isDisclosureOpen =
        doDisclosure && ArraySet.has(expandedActions, nextActionIdStack);

      const isCurrentPosition =
        actionStack.length === 1 && actionStack[0] === i;

      const isHighlightedCurrent =
        currentPath === fromActionStack([...path, i]);

      const op = resolveOperation(a.operation);

      const actionDiv = (open?: boolean) => (
        <ActionField
          type="Operation"
          key={`${path}:${i}:${actionStack}`}
          indent={indent}
          debug={false}
          isSemantics={true}
          doNotExecute={!doExecute}
          onClick={() => {
            dispatch(editorStepTo([...path, i]));
            dispatch(editorSetHoveredActionStack(undefined));
          }}
          ref={isHighlightedCurrent ? currentBarRef : undefined}
          current={isHighlightedCurrent}
          onDragOver={e => e.preventDefault()}
          onDrop={e => {
            if (path.length > 0) return;
            const toolId: ToolId | '' =
              e.dataTransfer.getData('private/toolid');
            if (toolId in baseOperations) {
              const tool = baseOperations[toolId as OperationId];
              if (tool.inputs.length === a.inputs.length) {
                dispatch(
                  editorChangeCalledOperation({operation: tool.id, action: i})
                );
              }
            }
          }}
        >
          <div className="icon">
            <span className="material-icons-outlined">
              {resolveOperation(a.operation).icon}
            </span>
          </div>
          <div className="text">
            {a.conditions.length > 0 && (
              <div className="details">
                {a.conditions.map((c, index, array) => {
                  const queryApp =
                    operation.demoSemantics!.queryApplications[c.queryApp];
                  const query = resolveOperation(queryApp.query);
                  let conditionText;
                  const highlight = isCurrentPosition
                    ? queryResults?.[c.queryApp]
                    : undefined;
                  if (query) {
                    // eslint-disable-next-line no-inner-declarations
                    function f<T>(nodes: T[]) {
                      return [
                        query.name,
                        ` is ${c.result ? 'true' : 'false'} for `,
                        ...addBetween(nodes, ' and '),
                        '?',
                      ];
                    }
                    const conditionFunction: GeneralFunc =
                      ('trueCase' in query &&
                        (c.result ? query.trueCase : query.falseCase)) ||
                      f;
                    conditionText = conditionFunction(
                      queryApp.inputs.map((n, i) => (
                        <TextNodeName
                          node={n}
                          doExecute={doExecute}
                          key={`qinp-${i}`}
                        />
                      ))
                    ).map(x => {
                      if (typeof x === 'string') {
                        if (typeof c.result === 'string') {
                          return x.replace('<OP>', strForComp(c.result));
                        }
                        return x;
                      } else {
                        return x;
                      }
                    });
                  }
                  return (
                    <div
                      className={styles.queryText}
                      key={`query-${c}-${index}-${array}`}
                    >
                      <div
                        className={styles.queryTextWithin}
                        style={{
                          backgroundColor: 'transparent',
                        }}
                      >
                        If {conditionText}
                        {index < array.length - 1 ? ' and ' : null}
                      </div>
                      <div
                        className={styles.queryTextBorder}
                        style={{
                          backgroundColor:
                            highlight === undefined
                              ? 'transparent'
                              : highlight
                              ? 'var(--green-background)'
                              : 'var(--red-background)',
                        }}
                      />
                    </div>
                  );
                })}
              </div>
            )}
            <div className="opDesc">
              {(isBaseOperation(op)
                ? op.instruction
                : standardInstruction<JSX.Element>(op))(
                a.inputs.map((n, j) => (
                  <TextNodeName
                    node={n}
                    doExecute={doExecute}
                    changeInputHandler={node => {
                      if (path.length === 0) {
                        dispatch(
                          editorChangeInputNode({
                            operation: outerOperation.id,
                            action: i,
                            input: j,
                            node,
                          })
                        );
                      }
                    }}
                    key={`input-${j}`}
                  />
                ))
              ).concat()}
            </div>
            <div className="outDesc">
              {isBaseOperation(op) &&
                op.hasOutput &&
                op.outputDescription!(
                  operation.demoSemantics!.outputNames[a.id]
                )}
            </div>
          </div>
          <div className="buttons">
            {doDisclosure && (
              <div onClick={(e: React.MouseEvent) => e.stopPropagation()}>
                <Disclosure.Button>
                  <IconButton
                    icon={open! ? 'expand_more' : 'expand_less'}
                    onClick={() => {
                      dispatch(editorExpand({path, idx: i}));
                    }}
                  />
                </Disclosure.Button>
              </div>
            )}
            {path.length === 0 && (
              <IconButton
                icon="delete"
                onClick={(e: React.MouseEvent) => {
                  e.stopPropagation();
                  dispatch(
                    editorDeleteSpecificAction({
                      operationId: operation.id,
                      action: i,
                    })
                  );
                }}
              />
            )}
          </div>
        </ActionField>
      );

      const item = doDisclosure ? (
        <Disclosure
          defaultOpen={isDisclosureOpen}
          key={`${path}:${i}:${actionStack}`}
        >
          {({open}) => (
            <>
              {actionDiv(open)}
              <Transition
                enter="transition duration-100 ease-out"
                enterFrom="transform scale-95 opacity-0"
                enterTo="transform scale-100 opacity-100"
                leave="transition duration-75 ease-out"
                leaveFrom="transform scale-100 opacity-100"
                leaveTo="transform scale-95 opacity-0"
              >
                <Disclosure.Panel>
                  <Semantics
                    operation={resolveOperation(a.operation)}
                    indent={indent + 1}
                    actionStack={
                      actionStack[0] === i ? actionStack.slice(1) : []
                    }
                    path={[...path, i]}
                    actionIdPath={[...actionIdPath, a.id]}
                    expandedActions={expandedActions}
                    allPaths={allNextPaths}
                  />
                </Disclosure.Panel>
              </Transition>
            </>
          )}
        </Disclosure>
      ) : (
        actionDiv()
      );

      if (enableReordering) {
        return (
          <Draggable key={`d${i}`} draggableId={`d${i}`} index={i}>
            {provided => (
              <div
                ref={provided.innerRef}
                {...provided.draggableProps}
                {...provided.dragHandleProps}
              >
                {item}
              </div>
            )}
          </Draggable>
        );
      } else {
        return item;
      }
    });

    if (enableReordering) {
      return (
        <DragDropContext
          onDragEnd={result => {
            if (result.destination) {
              dispatch(
                editorReorderActions({
                  operation: outerOperation.id,
                  sourceAction: result.source.index,
                  targetAction: result.destination.index,
                })
              );
            }
          }}
          key={`${path}:${actionStack}`}
        >
          <Droppable droppableId="list">
            {provided => (
              <div {...provided.droppableProps} ref={provided.innerRef}>
                {elements}
                {provided.placeholder}
              </div>
            )}
          </Droppable>
        </DragDropContext>
      );
    } else {
      return <>{elements}</>;
    }
  }

  return (
    <div style={{position: 'relative', marginTop: '5px', marginBottom: '5px'}}>
      <ActionField
        type="Operation"
        indent={0}
        debug={false}
        isSemantics={true}
        doNotExecute={false}
        onClick={() => {
          dispatch(editorStepToStart());
        }}
        current={actionStack.length === 1 && actionStack[0] === 0}
      >
        <div className="outermostIcon">
          <span className="material-icons-outlined">{outerOperation.icon}</span>
        </div>
        <div className="title">{outerOperation.name}</div>
      </ActionField>
      <Semantics
        operation={outerOperation}
        indent={1}
        actionStack={actionStack}
        path={[]}
        actionIdPath={[]}
        expandedActions={expandedActions}
        allPaths={allOpenPaths}
      />
      {/*placeholder*/}
      <div style={{height: '50px'}} />
    </div>
  );
}
