/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

export type BevelType =
  | 'Operation'
  | 'Query'
  | 'Pattern'
  | 'BuiltIn'
  | 'NewOperation'
  | 'EditorTool'
  | 'AddInput'
  | 'NewPattern';

const BevelBox = styled.div<{type: BevelType; info?: boolean}>`
  background: ${({type, info}) => {
    switch (type) {
      case 'Query':
        return info ? 'var(--query-light)' : 'var(--query)';
      case 'Pattern':
        return 'var(--pattern-match)';
      case 'AddInput':
        return 'var(--node-color)';
      case 'BuiltIn':
        return 'var(--text-gray)';
      case 'NewOperation':
        return 'var(--purple)';
      case 'NewPattern':
        return 'var(--node-color)';
      default:
        return info ? 'var(--action-light)' : 'var(--action)';
    }
  }};
  box-shadow: ${({info}) =>
    info ? '0 0 2px rgba(0, 0, 0, 0.14)' : '0 0 1px 0 rgba(0, 0, 0, 0.5)'};
  border-radius: ${({type}) =>
    type === 'Query' ? '25px' : 'var(--small-radius)'};
  height: ${({info}) => (info ? '45px' : '28px')};
  width: ${({info}) => (info ? '45px' : '28px')};
  color: white;
  text-align: center;

  .material-icons-outlined {
    line-height: ${({info}) => (info ? '45px' : '28px')};
    ${({info}) => (info ? '' : 'font-size: 18px;')}
    color: ${({type, info}) => {
      if (!info) return 'white';
      switch (type) {
        case 'Query':
          return 'var(--query)';
        case 'EditorTool':
          return 'black';
        default:
          return 'var(--action)';
      }
    }};
`;

export default function Bevel({
  icon,
  type,
  info,
}: {
  icon: string;
  type: BevelType;
  info?: boolean;
}) {
  return (
    <BevelBox type={type} info={info}>
      <span className="material-icons-outlined">{icon}</span>
    </BevelBox>
  );
}
