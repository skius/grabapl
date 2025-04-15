/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import Node from 'features/tools/Node';
import {
  GraphToolLabeling,
  useCurrentTool,
  useGraphToolLabeling,
} from 'features/tools/hooks';
import {useAppSelector} from 'src/hooks';
import {
  allTypeIds,
  ANY_TYPE_ID,
  ConcreteValue,
  getType,
  makeValue,
  NUMBER_TYPE_ID,
  OPERATION_TYPE,
  OPERATION_TYPE_ID,
  Type,
} from 'src/ConcreteValue';
import {useEffect, useState} from 'react';
import styles from './Palette.module.scss';
import classNames from 'classnames';
import {toolArgumentKey} from 'features/tools/Tool';
import IconButton from 'components/IconButton';
import {useAllOperations} from 'features/editor/operationHooks';

export default function Palette() {
  const tool = useCurrentTool();
  const inputCount = useAppSelector(state => state.tools.selectedNodes.length);
  const toolLabels = useGraphToolLabeling();
  const allOperations = useAllOperations();

  const [enableEdit, setEnableEdit] = useState(false);
  const [type, setType] = useState<Type<any> | null>(null);
  const [values, setValues] = useState(
    allTypeIds().reduce((r: Record<string, ConcreteValue[]>, id: string) => {
      const type: Type<any> = getType(id);
      r[id] = type.literalSuggestions.map(v => makeValue(v, type));
      return r;
    }, {})
  );

  const hidePalette =
    tool?.id !== 'editorChangeExampleValue' || inputCount !== 1;
  // !tool ||
  // (tool.type === 'Pattern' &&
  //   tool.inputTypes[inputCount] !== OPERATION_TYPE_ID) ||
  // inputCount >= tool.inputs.length;

  useEffect(() => {
    if (hidePalette) return;
    const typeId = tool.inputTypes[inputCount];
    setType(getType(typeId === ANY_TYPE_ID ? NUMBER_TYPE_ID : typeId));
  }, [tool, inputCount]);

  if (hidePalette || !type) return null;
  const inputType = tool.inputTypes[inputCount];

  return (
    <div className={styles.palette}>
      <IconButton
        onClick={() => setEnableEdit(e => !e)}
        icon="format_shapes"
        className={styles.editButton}
        active={enableEdit}
      />

      {inputType === ANY_TYPE_ID && (
        <div className={styles.types}>
          {allTypeIds().map(id =>
            id === ANY_TYPE_ID ? null : (
              <button
                key={id}
                className={classNames(
                  styles.typeButton,
                  type.id === id && styles.active
                )}
                onClick={() => setType(getType(id))}
              >
                {getType(id).name}
              </button>
            )
          )}
        </div>
      )}

      <div className={styles.literals}>
        {values[type.id].map((value, index) => (
          <LiteralNode
            toolLabels={toolLabels}
            key={index}
            value={value}
            setValue={newVal =>
              setValues({
                ...values,
                [type.id]: values[type.id].map((v, i) =>
                  i === index ? newVal : v
                ),
              })
            }
            enableEdit={enableEdit}
          />
        ))}
        {type === OPERATION_TYPE &&
          allOperations.map(op => {
            const value = makeValue(op.id, OPERATION_TYPE);
            return (
              <LiteralNode
                toolLabels={toolLabels}
                key={toolArgumentKey({value})}
                value={value}
                setValue={() => {}}
                enableEdit={enableEdit}
              />
            );
          })}
      </div>
    </div>
  );
}

function LiteralNode({
  toolLabels,
  value,
  setValue,
  enableEdit,
}: {
  toolLabels: GraphToolLabeling | null;
  enableEdit: boolean;
  value: ConcreteValue;
  setValue: (value: ConcreteValue) => void;
}) {
  return (
    <Node
      toolLabels={toolLabels}
      argument={{value}}
      computedStyle={{computedWidth: 60, computedHeight: 60}}
      defaultColor="var(--text-gray)"
      concreteValue={value}
      overrideEditHandler={enableEdit ? setValue : undefined}
    />
  );
}
