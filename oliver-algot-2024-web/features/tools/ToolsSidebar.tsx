/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import Sidebar, {TabPanel, Tabs} from 'components/Sidebar';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import styles from './ToolSidebar.module.scss';
import {useAllOperations} from 'features/editor/operationHooks';
import {Tab} from '@headlessui/react';
import ValueDialog from 'components/ValueDialog';
import {
  useState,
  useEffect,
  Dispatch,
  SetStateAction,
  PropsWithChildren,
} from 'react';
import {useHotkeys} from 'react-hotkeys-hook';
import baseOperations, {
  BaseOperationCategory,
  hasCategory,
} from 'src/BaseOperations';
import {
  makeValue,
  NUMBER_TYPE,
  NUMBER_TYPE_ID,
  STRING_TYPE,
  STRING_TYPE_ID,
} from 'src/ConcreteValue';
import {Operation} from 'src/Operation';
import {patternTools} from 'src/Patterns';
import {
  selectTool,
  removeSelectedTool,
  resetAll,
  executeTool,
  selectNode,
} from './toolsReducer';
import classNames from 'classnames';
import Bevel, {BevelType} from 'components/Bevel';
import {
  createAndOpenOperation,
  deletePatternEditor,
  makeNewPattern,
  patternChangePatternEditorName,
  PatternEditor,
  patternSelectPatternEditor,
} from 'features/editor/editorReducer';
import {findWeaklyConnectedComponents} from 'src/NodeUtils';
import {
  NodeIdArgument,
  ToolId,
  isNodeIdArgument,
  Tool as AlgotTool,
  isAbstractNodeArgument,
} from './Tool';
import {useCurrentTool} from './hooks';
import HoverInput from 'components/HoverInput';
import IconButton from 'components/IconButton';

const toolHotkey: Record<ToolId, string> = {
  changeFlexRow: 'R',
  changeFlexColumn: 'O',
  copyValue: 'C',
  removeNode: 'D',
  setValue: 'V',
  newNode: 'N',
  addChild: 'A',
  changeColor: 'B',
  hide: 'X',
  changeWidth: 'W',
  changeHeight: 'H',
  setBorderRadius: 'P',
};

type DocumentationInfo =
  | {
      name?: string;
      info?: string;
    }
  | undefined;

export default function ToolsSidebar() {
  const allOperations = useAllOperations();
  const operationEditor = useAppSelector(
    state =>
      state.editor.selectedOperation &&
      state.editor.operationsEditor[state.editor.selectedOperation]
  );
  const selectedTool = useAppSelector(state => state.tools.selectedTool);
  const selectedNodes = useAppSelector(state => state.tools.selectedNodes);
  const [toolTabIndex, setToolTabIndex] = useState(0);
  const [searchString, setSearchString] = useState('');
  const dispatch = useAppDispatch();
  const [hoveredDocumentation, setHoveredDocumentation] =
    useState<DocumentationInfo>({});

  const searchedOperations = allOperations.filter(
    op =>
      op.name.toLowerCase().includes(searchString.toLowerCase()) &&
      (selectedNodes.length === 0 || op.inputs.length === selectedNodes.length)
  );

  useHotkeys('esc', () =>
    dispatch(
      selectTool({
        id: null,
        selectedType: toolTabIndex === 1 ? 'Operation' : 'Other',
      })
    )
  );

  const renderTools = (name: string, condition: (op: Operation) => boolean) => {
    const tools = searchedOperations
      .filter(condition)
      .map(tool => (
        <Tool
          tool={tool}
          key={tool.id}
          setIsDialogVisible={setIsDialogVisible}
          setHoveredDocumentation={setHoveredDocumentation}
          toolTabIndex={toolTabIndex}
        />
      ));
    if (tools.length === 0) return null;
    return (
      <>
        <div className={styles.category}>{name}</div>
        {tools}
      </>
    );
  };

  useEffect(() => {
    if (!operationEditor) {
      setToolTabIndex(1);
    }
  }, [operationEditor]);

  useEffect(() => {
    dispatch(removeSelectedTool());
  }, [toolTabIndex]);

  const [isDialogVisible, setIsDialogVisible] = useState(false);

  const showDialog = () => {
    dispatch(removeSelectedTool());
    setIsDialogVisible(true);
  };

  const hideDialog = (value: string | null) => {
    setIsDialogVisible(false);
    if (value && selectedNodes[0]) {
      dispatch(
        executeTool({
          args: [
            {
              value: !isNaN(Number(value))
                ? makeValue(Number(value), NUMBER_TYPE)
                : makeValue(value, STRING_TYPE),
            },
            selectedNodes[0],
          ],
          tool: 'setValue',
        })
      );
    }
    dispatch(resetAll());
    return value;
  };

  const changeExampleEnabled =
    selectedNodes.length <= 1 &&
    selectedNodes.every(
      a => isAbstractNodeArgument(a) && a.abstractNode.type === 'PatternMatch'
    );
  const showChangeExampleDialog =
    selectedTool === 'changeExampleValue' && selectedNodes.length === 1;
  const handleExampleDialogClose = (value: string | null) => {
    if (value === null) {
      dispatch(selectTool({id: null, selectedType: null}));
      return;
    }
    const asInt = parseInt(value);
    if (isNaN(asInt)) {
      dispatch(selectNode({value: {type: STRING_TYPE_ID, value}}));
    } else {
      dispatch(selectNode({value: {type: NUMBER_TYPE_ID, value: asInt}}));
    }
  };

  const patterns = operationEditor?.patternEditors;

  return (
    <Sidebar left={false} className={styles.sidebar}>
      {isDialogVisible && selectedNodes.length === 1 && (
        <ValueDialog onClose={hideDialog} />
      )}
      <section className={styles.toolsList}>
        <Tab.Group selectedIndex={toolTabIndex} onChange={setToolTabIndex}>
          <Tabs
            tabs={[
              ['pattern', 'Patterns', !operationEditor],
              ['build', 'Operations'],
              ['live_help', 'Queries'],
            ]}
          />
          <Tab.Panels className={styles.tabPanels}>
            <TabPanel space={false} visible={!!operationEditor}>
              <div className={styles.patternContainer}>
                <div className={styles.patternOps}>
                  <div className={styles.category}>Input Structure</div>
                  {operationEditor &&
                    Object.values(patternTools)
                      .filter(t => t.subtype === 'Structure')
                      .map(tool => (
                        <Tool
                          tool={tool}
                          key={tool.id}
                          setIsDialogVisible={setIsDialogVisible}
                          setHoveredDocumentation={setHoveredDocumentation}
                          toolTabIndex={toolTabIndex}
                        />
                      ))}
                  <div className={styles.category}>Input Values</div>
                  <ToolButton
                    name="Change Example Value"
                    icon="edit"
                    type="Pattern"
                    isActive={selectedTool === 'changeExampleValue'}
                    onClick={() =>
                      dispatch(
                        selectTool({
                          id: 'changeExampleValue',
                          selectedType: null,
                        })
                      )
                    }
                    onMouseEnter={() => {}}
                    disabled={!changeExampleEnabled}
                  />
                  {showChangeExampleDialog && (
                    <ValueDialog onClose={handleExampleDialogClose} />
                  )}
                </div>
                <div style={{borderTop: '1px solid var(--border-gray)'}} />
                <div className={styles.category}>Pattern Editor</div>
                <ToolButton
                  name="Add Pattern"
                  icon="playlist_add"
                  type="NewPattern"
                  isActive={false}
                  onClick={() => {
                    dispatch(makeNewPattern());
                  }}
                  disabled={false}
                />
                <div className={styles.patterneditor}>
                  <ul>
                    {patterns?.map((pattern, i, patterns) => (
                      <PatternEditorButton
                        key={i}
                        pattern={pattern}
                        idx={i}
                        active={i === operationEditor?.currentEditorIndex}
                        deleteDisabled={patterns.length === 1}
                      />
                    ))}
                  </ul>
                </div>
              </div>
            </TabPanel>
            <TabPanel space={false}>
              <input
                placeholder="Search"
                className={styles.searchInput}
                type="text"
                value={searchString}
                onChange={e => setSearchString(e.target.value)}
              />
              <NewOperation
                setToolTabIndex={setToolTabIndex}
                isQuery={false}
                operationEditor={!!operationEditor}
                setHoveredDocumentation={setHoveredDocumentation}
              />
              {Object.keys(BaseOperationCategory)
                .filter(category => category !== 'Hidden')
                .map(category => (
                  <div key={category}>
                    {renderTools(
                      category,
                      op => hasCategory(op) && op.category === category
                    )}
                    {category === 'Basics' && selectedNodes.length <= 1 ? (
                      <ToolButton
                        name={'Set Value'}
                        icon="question_answer"
                        type="Operation"
                        hotkey={toolHotkey['setValue']}
                        isActive={isDialogVisible}
                        onClick={showDialog}
                        onMouseEnter={() =>
                          setHoveredDocumentation({
                            name: 'Set Value',
                            info: 'Manually sets the value of a node.',
                          })
                        }
                        onMouseLeave={() => setHoveredDocumentation(undefined)}
                      />
                    ) : (
                      ''
                    )}
                  </div>
                ))}
              {renderTools(
                'My Operations',
                op => !hasCategory(op) && !op.isQuery
              )}
              {operationEditor &&
                renderTools(
                  'My Queries as Operations',
                  op => !hasCategory(op) && op.isQuery
                )}
            </TabPanel>
            <TabPanel space={false}>
              <input
                placeholder="Search"
                className={styles.searchInput}
                type="text"
                value={searchString}
                onChange={e => setSearchString(e.target.value)}
              />
              <NewOperation
                setToolTabIndex={setToolTabIndex}
                isQuery={true}
                operationEditor={!!operationEditor}
                setHoveredDocumentation={setHoveredDocumentation}
              />
              {renderTools('Base', op => op.isQuery && !op.isUserDefined)}
              {renderTools('Custom', op => op.isQuery && op.isUserDefined)}
            </TabPanel>
          </Tab.Panels>
        </Tab.Group>
      </section>
      <Documentation documentationInfo={hoveredDocumentation} />
    </Sidebar>
  );
}

function NewOperation({
  setToolTabIndex,
  isQuery,
  operationEditor,
  setHoveredDocumentation,
}: {
  setToolTabIndex: (index: number) => void;
  isQuery: boolean;
  operationEditor: boolean;
  setHoveredDocumentation: (docInfo: DocumentationInfo | undefined) => void;
}) {
  const dispatch = useAppDispatch();
  const selectedNodes = useAppSelector(state => state.tools.selectedNodes);
  const allNodes = useAppSelector(state => state.playground.graph.nodes);

  const selectedConcreteNodes = selectedNodes
    .filter(node => isNodeIdArgument(node))
    .map(arg => allNodes[(arg as NodeIdArgument).nodeId]);

  const selectedGraphs = () => {
    return findWeaklyConnectedComponents(selectedConcreteNodes, allNodes);
  };

  if (operationEditor) {
    return null;
  }

  return (
    <>
      <div className={styles.category}>New</div>
      <ToolButton
        name={isQuery ? 'New Query' : 'New Operation'}
        icon="add"
        type="NewOperation"
        isActive={false}
        onClick={() => {
          dispatch(
            createAndOpenOperation({
              nodes: selectedGraphs(),
              isQuery: isQuery,
            })
          );
          setToolTabIndex(0);
        }}
        onMouseEnter={() =>
          setHoveredDocumentation({
            name: isQuery ? 'New Query' : 'New Operation',
            info: isQuery
              ? 'Create a new query from the selected nodes.'
              : 'Create a new operation from the selected nodes.',
          })
        }
        onMouseLeave={() => setHoveredDocumentation(undefined)}
      />
    </>
  );
}

function Tool({
  tool,
  setHoveredDocumentation,
  setIsDialogVisible,
  toolTabIndex,
}: {
  tool: AlgotTool;
  setHoveredDocumentation: (
    documentation: DocumentationInfo | undefined
  ) => void;
  setIsDialogVisible: Dispatch<SetStateAction<boolean>>;
  toolTabIndex?: number;
}) {
  const selectedTool = useAppSelector(state => state.tools.selectedTool);
  const dispatch = useAppDispatch();
  return (
    <ToolButton
      name={tool.name}
      icon={tool.icon || 'help'}
      type={
        tool.id === 'addInput'
          ? 'AddInput'
          : toolTabIndex === 2
          ? 'Query'
          : tool.type
      }
      isActive={selectedTool === tool.id}
      hotkey={toolHotkey[tool.id]}
      onClick={() => {
        dispatch(
          selectTool({
            id: tool.id,
            selectedType: toolTabIndex === 1 ? 'Operation' : 'Other',
          })
        );
        setIsDialogVisible(false);
      }}
      onMouseEnter={() => {
        setHoveredDocumentation({name: tool.name, info: tool.documentation});
      }}
      onMouseLeave={() => setHoveredDocumentation(undefined)}
      draggable={tool.id in baseOperations ? 'true' : undefined}
      onDragStart={e => {
        e.dataTransfer.setData('private/toolid', tool.id);
      }}
    />
  );
}

function ToolButton({
  name,
  icon,
  type,
  isActive,
  onClick,
  onMouseEnter,
  onMouseLeave,
  hotkey,
  disabled,
  draggable,
  onDragStart,
  children,
}: PropsWithChildren<{
  name: string;
  icon: string;
  type: BevelType;
  isActive: boolean;
  onClick: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  hotkey?: string;
  disabled?: boolean;
  draggable?: 'true';
  onDragStart?: (e: React.DragEvent<HTMLDivElement>) => void;
}>) {
  useHotkeys(hotkey || [], e => {
    e.preventDefault();
    onClick();
  });

  return (
    <div
      className={styles.toolWrapper}
      draggable={draggable}
      onDragStart={onDragStart}
    >
      <button
        className={classNames(
          styles.tool,
          !disabled && isActive && styles.toolActive,
          disabled && styles.toolDisabled
        )}
        onClick={disabled ? undefined : onClick}
        onMouseEnter={disabled ? undefined : onMouseEnter}
        onMouseLeave={onMouseLeave}
      >
        <Bevel icon={icon} type={type} />
        <span className={styles.toolName}>{name}</span>
        <span className={styles.toolHotkey}>{hotkey}</span>
        {children}
      </button>
    </div>
  );
}

function Documentation({
  documentationInfo,
}: {
  documentationInfo: DocumentationInfo | undefined;
}) {
  const currentTool = useCurrentTool();

  const stringToDisplay = documentationInfo?.info || currentTool?.documentation;
  const titleToDisplay =
    documentationInfo?.name || currentTool?.name || 'Documentation';

  return (
    <section className={styles.documentation}>
      <h3 className={styles.documentationH3}>{titleToDisplay}</h3>

      {stringToDisplay
        ? stringToDisplay || 'No help provided.'
        : 'Select a tool to see its documentation.'}
    </section>
  );
}

function PatternEditorButton({
  pattern,
  idx,
  active,
  deleteDisabled,
}: {
  pattern: PatternEditor;
  idx: number;
  active: boolean;
  deleteDisabled: boolean;
}) {
  const dispatch = useAppDispatch();

  return (
    <div className={styles.toolWrapper}>
      <button
        className={classNames(styles.tool, active && styles.toolActive)}
        onClick={() => dispatch(patternSelectPatternEditor(idx))}
      >
        <Bevel icon="playlist_add" type="Pattern" />
        <HoverInput
          size={pattern.name.length || 1}
          style={{marginLeft: 'var(--space-3)'}}
          disabled={!active}
          value={pattern.name}
          onChange={e =>
            dispatch(
              patternChangePatternEditorName({
                editor: pattern,
                name: e.target.value,
              })
            )
          }
        />
        <div style={{marginLeft: 'auto'}}>
          <IconButton
            icon="delete"
            onClick={e => {
              dispatch(deletePatternEditor(idx));
              e.stopPropagation();
            }}
            disabled={deleteDisabled}
          />
        </div>
      </button>
    </div>
    /*<ToolButton
      name={pattern.name}
      icon="playlist_add"
      type="Pattern"
      isActive={active}
      onClick={() => {
        dispatch(patternSelectPatternEditor(idx));
      }}
    >
      <div style={{marginLeft: 'auto'}}>
        <IconButton
          icon="delete"
          onClick={e => {
            dispatch(deletePatternEditor(idx));
            e.stopPropagation();
          }}
          disabled={deleteDisabled}
        />
      </div>
    </ToolButton>*/
  );
}
