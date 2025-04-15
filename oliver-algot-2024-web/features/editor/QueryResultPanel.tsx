/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {deleteQueryApp} from 'features/editor/editorReducer';
import {nameForAbstractNode} from 'src/AbstractNodeUtils';
import styled from 'styled-components';
import useOuD from 'features/editor/useOuD';
import PredicateButton, {strForComp} from 'features/editor/PredicateButton';
import {ActionCondition, QueryApplicationId} from 'src/DemoSemantics';
import IconButton from 'components/IconButton';
import React from 'react';
import {useResolveOperation} from 'features/editor/operationHooks';
import {isQuery, N} from 'src/BaseOperations';

export const PanelTitle = styled.div`
  font-weight: 500;
  font-size: var(--font-sm);
  line-height: 1.2;
`;

export const Panel = styled.div<{isUserDefined?: boolean}>`
  position: relative;
  background: ${'var(--background-lighterblue)'};
  border: ${props =>
    props.isUserDefined
      ? '2px dashed var(--border-gray)'
      : '2px solid var(--border-gray)'};
  padding: var(--space-4);
`;

export const Predicates = styled.div`
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 0.5rem;
`;

export const SmallPredicates = styled.div`
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  column-gap: 0.5rem;
`;

export default function QueryResultPanel({
  queryAppId,
}: {
  queryAppId: QueryApplicationId;
}) {
  function treatString(str: string, actionCondition?: ActionCondition) {
    if (typeof actionCondition?.result === 'string') {
      return str.replace('<OP>', strForComp(actionCondition.result));
    }
    return str;
  }

  const operation = useOuD()!;
  const isActiveCondition = useAppSelector(state =>
    state.editor.operationsEditor[
      state.editor.selectedOperation!
    ].activeConditions.find(c => c.queryApp === queryAppId)
  );

  const resolveOperation = useResolveOperation();
  const dispatch = useAppDispatch();
  const queryApp = operation.demoSemantics!.queryApplications[queryAppId];
  const query = resolveOperation(queryApp.query);
  const isUserDefined = query.isUserDefined;
  const inputNames = queryApp.inputs.map(i =>
    nameForAbstractNode(i, operation)
  );

  const isComparisonQuery = queryApp.query === 'compareNumbers';

  return (
    <Panel isUserDefined={isUserDefined}>
      <IconButton
        onClick={() => dispatch(deleteQueryApp(queryAppId))}
        icon="delete"
      />
      <PanelTitle>
        {isQuery(query) && query.question ? (
          query
            .question(inputNames)
            .map((element, index) => (
              <React.Fragment key={index}>
                {treatString(element, isActiveCondition)}
              </React.Fragment>
            ))
        ) : (
          <>
            {query.name} on{' '}
            {inputNames.map((name, index) => (
              <React.Fragment key={`name-${index}`}>
                <N>{name}</N>
                {index < inputNames.length - 1 ? ', ' : ''}
              </React.Fragment>
            ))}
          </>
        )}
      </PanelTitle>

      {isComparisonQuery ? (
        <SmallPredicates>
          {(['>=', '==', '<=', '>', '!=', '<'] as const).map(op => (
            <PredicateButton
              queryAppId={queryAppId}
              style={op}
              isActiveCondition={isActiveCondition}
            />
          ))}
        </SmallPredicates>
      ) : (
        <Predicates>
          <PredicateButton
            queryAppId={queryAppId}
            style={true}
            isActiveCondition={isActiveCondition}
          />
          <PredicateButton
            queryAppId={queryAppId}
            style={false}
            isActiveCondition={isActiveCondition}
          />
        </Predicates>
      )}
    </Panel>
  );
}
