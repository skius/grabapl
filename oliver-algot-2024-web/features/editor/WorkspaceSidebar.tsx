/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styles from './WorkspaceSidebar.module.scss';
import {
  changeName,
  changeOperationDocumentation,
} from 'features/editor/editorReducer';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import BlockLabel from 'components/BlockLabel';
import {Tab} from '@headlessui/react';
import Sidebar, {TabPanel, Tabs} from 'components/Sidebar';
import OperationBox from 'features/editor/OperationBox';
import SemanticsList from 'features/editor/SemanticsList';
import EditorActions from './EditorActions';
import {useEffect, useState} from 'react';

const tabs: [string, string][] = [
  ['info', 'Workspace'],
  ['home_repair_service', 'My Operations'],
];

export default function WorkspaceSidebar() {
  const workspace = useAppSelector(state => state.editor);
  const operations = useAppSelector(state => state.editor.operations);
  const selected = useAppSelector(state => state.editor.selectedOperation);
  const dispatch = useAppDispatch();
  const myOperations = Object.values(operations)
    .filter(op => !op.isQuery)
    .map(op => <OperationBox operation={op} key={op.id} />);
  const myQueries = Object.values(operations)
    .filter(op => op.isQuery)
    .map(op => <OperationBox operation={op} key={op.id} />);

  const [descriptionValue, setDescriptionValue] = useState(
    (selected && operations[selected]?.documentation) || ''
  );

  // Update local state when the selected operation changes
  useEffect(() => {
    if (selected)
      setDescriptionValue(operations[selected]?.documentation || '');
  }, [selected, operations]);

  const nArguments = selected ? operations[selected].inputs.length : 0;
  const nPatterns = selected
    ? Object.keys(operations[selected].patterns).length - nArguments
    : 0;

  return (
    <div className={styles.leftContainer}>
      <Sidebar left={true}>
        <Tab.Group>
          <Tabs tabs={tabs} />
          <Tab.Panels>
            <TabPanel>
              <BlockLabel>Workspace Name</BlockLabel>
              <input
                type="text"
                value={workspace.name}
                onChange={e => dispatch(changeName(e.currentTarget.value))}
              />
            </TabPanel>
            <TabPanel space={false}>
              {myOperations.length === 0 && myQueries.length === 0 && (
                <div className={styles.noOperations}>
                  You haven't created any operations yet.
                </div>
              )}
              {myOperations.length > 0 && (
                <>
                  <div className={styles.heading}>My Operations</div>
                  {myOperations}
                </>
              )}
              {myQueries.length > 0 && (
                <>
                  <div className={styles.heading}>My Queries</div>
                  {myQueries}
                </>
              )}
            </TabPanel>
          </Tab.Panels>
        </Tab.Group>
      </Sidebar>
      {selected ? (
        <>
          <Sidebar left={true} style={{flex: '2'}}>
            <Tab.Group>
              <Tabs
                borderTop={true}
                tabs={[
                  ['format_list_numbered', 'Steps'],
                  ['info', 'Details'],
                ]}
              />
              <Tab.Panels>
                <TabPanel space={false}>
                  <SemanticsList operation={operations[selected]} />
                </TabPanel>
                <TabPanel>
                  <BlockLabel>{operations[selected].name}</BlockLabel>
                  <div className={styles.operationDescription}>
                    {' '}
                    This {operations[selected].isQuery
                      ? 'query'
                      : 'operation'}{' '}
                    takes{' '}
                    <span
                      style={{whiteSpace: 'nowrap'}}
                      className={styles.blueText}
                    >
                      <b>{nArguments}</b> {nArguments !== 1 ? 'nodes' : 'node'}
                    </span>{' '}
                    as input and uses up to{' '}
                    <span
                      style={{whiteSpace: 'nowrap'}}
                      className={styles.pinkText}
                    >
                      <b>{nPatterns}</b> {nPatterns !== 1 ? 'nodes' : 'node'}
                    </span>{' '}
                    for pattern matching.
                  </div>
                  <BlockLabel>Description</BlockLabel>
                  <textarea
                    className={styles.descriptionInput}
                    placeholder="Describe briefly what this operation does."
                    value={descriptionValue}
                    onChange={e => setDescriptionValue(e.target.value)}
                    onBlur={e =>
                      dispatch(
                        changeOperationDocumentation({
                          ...operations[selected],
                          documentation: e.currentTarget.value,
                        })
                      )
                    }
                    rows={3}
                    maxLength={70}
                  />
                </TabPanel>
              </Tab.Panels>
            </Tab.Group>
          </Sidebar>
          <EditorActions />
        </>
      ) : (
        <Sidebar left={true} style={{flex: '1'}} />
      )}
    </div>
  );
}
