/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styles from './OperationBox.module.scss';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {
  changeOperationIcon,
  changeOperationName,
  deleteOperation,
  setDemoOperation,
} from 'features/editor/editorReducer';
import HoverInput from 'components/HoverInput';
import IconPicker from './IconPicker';
import {Operation} from 'src/Operation';
import classNames from 'classnames';
import IconButton from 'components/IconButton';
import {useEffect, useRef, useState} from 'react';

export default function OperationBox({operation}: {operation: Operation}) {
  const dispatch = useAppDispatch();
  const selected = useAppSelector(
    state => state.editor.selectedOperation === operation.id
  );

  const [fieldNameValue, setFieldNameValue] = useState('');
  useEffect(() => setFieldNameValue(operation.name), [operation.name]);

  const inputRef = useRef<HTMLInputElement>(null);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFieldNameValue(e?.target.value);
  };

  const handleInputKeyPress = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter') {
      inputRef.current?.blur();
    }
  };

  return (
    <article
      className={classNames(styles.operation, selected && styles.selected)}
      onClick={e => {
        if (e.target instanceof Element && e.target.closest('button') !== null)
          return;
        dispatch(setDemoOperation(operation.id));
      }}
    >
      <IconPicker
        disabled={!selected}
        icon={operation.icon}
        onChange={icon =>
          dispatch(changeOperationIcon({id: operation.id, icon}))
        }
      />
      <HoverInput
        ref={inputRef}
        size={operation.name.length || 1}
        disabled={!selected}
        className={classNames(styles.h, !selected && styles.noPointerEvents)}
        value={fieldNameValue}
        onChange={handleInputChange}
        onKeyPress={handleInputKeyPress}
        onBlur={e =>
          dispatch(
            changeOperationName({name: e.target.value.trim(), id: operation.id})
          )
        }
      />
      {selected && (
        <IconButton
          onClick={e => {
            dispatch(deleteOperation(operation.id));
            e.stopPropagation();
          }}
          icon="delete"
          className={styles.delete}
        />
      )}
    </article>
  );
}
