/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useAppDispatch, useAppSelector} from 'src/hooks';
import AbstractNode, {
  getColorForAbstractNode,
} from 'features/editor/AbstractNode';
import {
  finishDemo,
  fromActionStack,
  PatternEditor,
  resetErrorMessage,
  undo,
} from 'features/editor/editorReducer';
import React, {useEffect, useMemo} from 'react';
import QueryOutputPanel from 'features/editor/QueryOutputPanel';
import styled from 'styled-components';
import useOuD from 'features/editor/useOuD';
import GraphView from 'features/graphView/GraphView';
import ScrollView from 'components/ScrollView';
import ContextBar from 'components/ContextBar';
import {reconstructApproximateGraphFrom} from 'src/ApproximateGraphAPI';
import Palette from 'features/tools/Palette';
import QueryResultPanel from './QueryResultPanel';

const ResultPanels = styled.div`
  position: sticky;
  left: 0;
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  grid-template-rows: auto auto;
  gap: var(--space-6);
  padding: var(--space-6);
`;

const CustomQueryContainer = styled.div`
  display: flex;
  justify-content: center;
`;

const Container = styled.div`
  position: absolute;
  height: 100%;
  width: 100%;
  z-index: 120;
  background-color: white;
  display: flex;
  align-items: stretch;
  flex-direction: column;
`;

const Main = styled.div`
  flex-grow: 1;
  padding-bottom: 140px;
  z-index: 1;
  position: relative;
`;

const Header = styled.div`
  position: relative;
  z-index: 2;
`;

export default function DemonstrationView() {
  const dispatch = useAppDispatch();

  const operation = useOuD();

  const currentEditor = useAppSelector(state => {
    if (operation === null) return null;
    const editor = state.editor.operationsEditor[operation.id];
    return editor.patternEditors[editor.currentEditorIndex];
  });

  const {actionStack, approximateGraphs: storedGraphs} = currentEditor || {
    actionStack: [] as number[],
    approximateGraphs: {} as PatternEditor['approximateGraphs'],
  };

  const idx = fromActionStack(actionStack);

  const [abstractGraph, queryValue] = useMemo(() => {
    if (!operation) return [undefined, undefined];

    if (!idx || !storedGraphs?.[idx]) {
      throw `Invariant violated! The graph @${idx} should be stored in the state`;
    }

    const outputGraph = reconstructApproximateGraphFrom(
      storedGraphs![idx!].graph
    );
    return [outputGraph, outputGraph.queryResult];
  }, [operation, actionStack, storedGraphs]);

  const errorMessage = useAppSelector(state => state.editor.errorString);
  useEffect(() => {
    if (typeof errorMessage === 'string') {
      alert(errorMessage);
      dispatch(resetErrorMessage());
    }
  }, [errorMessage]);

  if (operation === null) return null;

  return (
    <Container>
      <Header>
        <ContextBar
          onClose={() => dispatch(finishDemo())}
          title={operation.name}
          undo={undo}
        />
      </Header>
      <Main>
        <ScrollView>
          <ResultPanels>
            {Object.keys(operation.demoSemantics!.queryApplications).map(
              app => (
                <QueryResultPanel queryAppId={app} key={app} />
              )
            )}
          </ResultPanels>
          <CustomQueryContainer>
            {operation.isQuery ? (
              <QueryOutputPanel queryResult={queryValue!} />
            ) : (
              ''
            )}
          </CustomQueryContainer>

          <GraphView
            GraphNode={AbstractNode}
            lineProps={(from, to) => {
              const fromNode = abstractGraph!.payload(from);
              const color = getColorForAbstractNode(
                fromNode.abstractNode,
                operation
              );
              return {stroke: color};
            }}
            graph={abstractGraph!}
          />
        </ScrollView>
        <Palette />
      </Main>
    </Container>
  );
}
