/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useAppDispatch} from 'src/hooks';
import {addCondition, removeCondition} from 'features/editor/editorReducer';
import Button from 'components/Button';
import {ActionCondition} from 'src/DemoSemantics';

export type ComparisonOperator = '==' | '!=' | '<' | '<=' | '>' | '>=';
export function strForComp(comp: ComparisonOperator) {
  switch (comp) {
    case '==':
      return '=';
    case '!=':
      return '≠';
    case '<':
      return '<';
    case '<=':
      return '≤';
    case '>':
      return '>';
    case '>=':
      return '≥';
  }
}

export default function PredicateButton({
  queryAppId,
  style,
  isActiveCondition,
}: {
  queryAppId: string;
  style?: boolean | ComparisonOperator;
  isActiveCondition?: ActionCondition;
}) {
  const dispatch = useAppDispatch();
  const active = isActiveCondition && isActiveCondition.result === style;

  return (
    <Button
      active={active}
      style={{
        marginTop: '12px',
        background: active
          ? style === undefined
            ? 'var(--yellow)'
            : typeof style === 'string'
            ? 'var(--blue)'
            : style
            ? 'var(--green)'
            : 'var(--red)'
          : 'var(--border-gray)',
      }}
      onClick={() =>
        dispatch(
          active
            ? removeCondition(queryAppId)
            : addCondition({queryApp: queryAppId, result: style || false})
        )
      }
    >
      <span className="material-icons-outlined">
        {style === undefined
          ? 'help'
          : typeof style === 'string'
          ? strForComp(style)
          : style
          ? 'check_circle'
          : 'cancel'}
      </span>
    </Button>
  );
}
