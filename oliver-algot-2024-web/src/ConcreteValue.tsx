/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {OperationId} from 'src/Operation';
import {ComponentType, useRef} from 'react';
import styles from './ConcreteValue.module.scss';
import IconButton from 'components/IconButton';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ConcreteValue = {type: string; value: any};
export const ANY_TYPE_ID = 'any.algot';
export const STRING_TYPE_ID = 'string.algot';
export const NUMBER_TYPE_ID = 'number.algot';
export const OPERATION_TYPE_ID = 'operation.algot';
export const COLOR_TYPE_ID = 'color.algot';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const typeRegister: Record<string, Type<any>> = {};

export interface Type<Payload> {
  id: string;
  name: string;
  // A function for which we have: for all a, b: Payload. hash(a) === hash(b) ==> a equals b with very high probability.
  hash: (p: Payload) => string;
  view: ComponentType<{value: Payload}>;
  literalSuggestions: Payload[];
  editableView: ComponentType<{
    value: Payload;
    setValue: (p: Payload) => void;
  }>;
  stringRepresentation: (p: Payload) => string;
}

function registerType<Payload>(t: Type<Payload>): Type<Payload> {
  typeRegister[t.id] = t;
  return t;
}

export function hashValue(value: ConcreteValue) {
  return typeRegister[value.type].hash(value.value);
}

export function renderValue(value: ConcreteValue) {
  const View = typeRegister[value.type].view;
  return <View value={value.value} />;
}

export function renderEditable(
  value: ConcreteValue,
  setValue: (v: ConcreteValue) => void
) {
  const type = typeRegister[value.type];
  if (!type.editableView) return 'cannot be edited';
  return (
    <type.editableView
      value={value.value}
      setValue={v => setValue({type: value.type, value: v})}
    />
  );
}

export function stringRepresentation(value: ConcreteValue) {
  return typeRegister[value.type].stringRepresentation(value.value);
}

export function getType(typeId: string) {
  return typeRegister[typeId];
}

export function extractPayload<Payload>(
  value: ConcreteValue,
  type: Type<Payload>
): Payload {
  if (value.type !== type.id)
    throw `Expected a value of type ${type.name} but found ${
      getType(value.type)?.name || value.type
    }.`;
  return value.value as Payload;
}

export function makeValue<Payload>(
  payload: Payload,
  type: Type<Payload>
): ConcreteValue {
  return {type: type.id, value: payload};
}

export function allTypeIds(): string[] {
  return Object.keys(typeRegister);
}

export const ANY_TYPE = registerType<OperationId>({
  id: ANY_TYPE_ID,
  name: 'value',
  hash: p => p,
  view: () => null,
  editableView: () => null,
  stringRepresentation: () => '',
  literalSuggestions: [],
});

export const OPERATION_TYPE = registerType<OperationId>({
  id: OPERATION_TYPE_ID,
  name: 'Operation',
  hash: p => p,
  view: () => null,
  editableView: () => null,
  stringRepresentation: () => 'operation',
  literalSuggestions: [],
});

export const STRING_TYPE = registerType<string>({
  id: STRING_TYPE_ID,
  name: 'Text',
  hash: p => p,
  view: ({value}) => <div className={styles.textView}>{value}</div>,
  stringRepresentation: p => p,
  literalSuggestions: ['hello', 'world'],
  editableView: function StringLiteralPicker({value, setValue}) {
    return (
      <input
        type="text"
        value={value}
        className={styles.editableInput}
        onInput={e => setValue(e.currentTarget.value)}
      />
    );
  },
});

export const NUMBER_TYPE = registerType<number>({
  id: NUMBER_TYPE_ID,
  name: 'Number',
  hash: p => p.toString(),
  view: ({value}) => <div className={styles.textView}>{value}</div>,
  stringRepresentation: p => p.toString(),
  literalSuggestions: [0, 1, 2, 4, 7, 13, 24, 44, 81, 149, 274, 504],
  editableView: function NumberLiteralPicker({value, setValue}) {
    return (
      <input
        type="text"
        className={styles.editableInput}
        value={value}
        onInput={e => setValue(Number(e.currentTarget.value))}
      />
    );
  },
});

export const COLOR_TYPE = registerType<string>({
  id: COLOR_TYPE_ID,
  name: 'Color',
  hash: p => p,
  view: ({value}) => (
    <div
      style={{
        width: '24px',
        height: '24px',
        border: '1px solid white',
        backgroundColor: value,
      }}
    />
  ),
  stringRepresentation: p => p,
  literalSuggestions: [
    '#000000',
    '#ffffff',
    'transparent',
    '#55efc4',
    '#00b894',
    '#00cec9',
    '#81ecec',
    '#74b9ff',
    '#a29bfe',
    '#0984e3',
    '#6c5ce7',
    '#ffeaa7',
    '#fab1a0',
    '#e17055',
    '#fdcb6e',
    '#d63031',
    '#ff7675',
    '#fd79a8',
    '#e84393',
    '#F6F7F7',
    '#dfe6e9',
    '#b2bec3',
    '#636e72',
    '#2d3436',
  ],
  editableView: function ColorLiteralPicker({value, setValue}) {
    return (
      <input
        type="color"
        value={value}
        onInput={e => setValue(e.currentTarget.value)}
      />
    );
  },
});

function Photo({value}: {value: string}) {
  return (
    <img
      src={value}
      style={{
        width: 'var(--node-width)',
        height: 'var(--node-height)',
        objectFit: 'contain',
      }}
    />
  );
}

registerType<string>({
  id: 'photo.ui.algot',
  name: 'Photo',
  hash: p => p,
  view: Photo,
  stringRepresentation: () => 'photo',
  literalSuggestions: ['/panda.jpg', '/panda2.jpg', '/panda3.jpg'],
  editableView: function PhotoLiteralPicker({value, setValue}) {
    const fileInput = useRef<HTMLInputElement>(null);
    return (
      <>
        <Photo value={value} />
        <IconButton
          className={styles.photoButton}
          onClick={() => fileInput.current?.click()}
          icon="photo_library"
        />
        <input
          type="file"
          accept="image/*"
          ref={fileInput}
          style={{display: 'none'}}
          onChange={e => {
            if (!e.currentTarget.files?.[0]) return;
            const reader = new FileReader();
            reader.onload = function () {
              setValue(reader.result as string);
            };
            reader.readAsDataURL(e.currentTarget.files[0]);
          }}
        />
      </>
    );
  },
});
