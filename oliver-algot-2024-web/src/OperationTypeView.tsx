import {OPERATION_TYPE} from 'src/ConcreteValue';
import {useResolveOperation} from 'features/editor/operationHooks';
import {OperationId} from 'src/Operation';

// This is for breaking a cyclic dependency

function OperationView({value}: {value: OperationId}) {
  const operation = useResolveOperation()(value);
  if (!operation) return <>non-existent operation</>;
  return (
    <div>
      <span
        className="material-icons-outlined"
        style={{fontSize: 'var(--font-xs)'}}
      >
        {operation.icon}
      </span>
      <div style={{fontSize: 'var(--font-xs)'}}>{operation.name}</div>
    </div>
  );
}

OPERATION_TYPE.view = OperationView;
OPERATION_TYPE.editableView = OperationView;
