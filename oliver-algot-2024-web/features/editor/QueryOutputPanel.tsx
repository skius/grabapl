import styled from 'styled-components';
import IconButton from 'components/IconButton';
import Button from 'components/Button';
import {useAppDispatch} from 'src/hooks';
import {PanelTitle, Predicates} from './QueryResultPanel';
import {OperationId} from 'src/Operation';
import {executeTool} from 'features/tools/toolsReducer';

const Panel = styled.div<{queryValue: boolean}>`
  position: relative;
  background: ${props =>
    props.queryValue ? 'var(--light-green)' : 'var(--light-orange)'};
  border: 2px solid var(--border-gray);
  padding: var(--space-4);
`;

const IconButtonContainer = styled.div`
  display: flex;
  justify-content: center;
`;

const buttonStyle = (valueRepresentative: boolean, queryValue: boolean) => ({
  marginTop: '12px',
  background: queryValue
    ? valueRepresentative
      ? 'var(--green)'
      : 'var(--red)'
    : 'var(--border-gray)',
});

export default function QueryOutputPanel({
  queryResult,
}: {
  queryResult: boolean;
}) {
  const dispatch = useAppDispatch();
  function update(operation: OperationId) {
    dispatch(
      executeTool({
        args: [],
        tool: operation,
      })
    );
  }

  return (
    <Panel queryValue={queryResult}>
      <IconButtonContainer>
        <IconButton icon="star" />
      </IconButtonContainer>
      <PanelTitle>The query result</PanelTitle>
      <Predicates>
        <Button
          active={queryResult}
          style={buttonStyle(true, queryResult)}
          onClick={() => update('setQueryResultToTrue')}
        >
          <span className="material-icons-outlined">check</span>
        </Button>
        <Button
          active={!queryResult}
          style={buttonStyle(false, !queryResult)}
          onClick={() => update('setQueryResultToFalse')}
        >
          <span className="material-icons-outlined">cancel</span>
        </Button>
      </Predicates>
    </Panel>
  );
}
