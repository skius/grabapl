/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

const Button = styled.button<{
  active?: boolean;
  autoWidth?: boolean;
  large?: boolean;
}>`
  cursor: pointer;
  background-image: linear-gradient(
    180deg,
    var(--interact-tint-lighter) 0%,
    var(--interact-tint) 100%
  );
  border-radius: var(--small-radius);
  color: white;
  text-align: center;
  font-size: var(--font-sm);
  padding: ${({large}) => (large ? '16px' : '')} 10px;
  display: flex;
  justify-content: center;
  align-items: center;
  width: ${({autoWidth}) => (autoWidth ? 'auto' : '100%')};

  &:disabled {
    background: var(--border-gray);
  }

  &:not(:disabled):active${({active}) => (active ? ', &' : '')} {
    box-shadow: inset 0 0 4px 0 rgba(0, 0, 0, 0.5);
  }

  .material-icons-outlined {
    font-size: var(--font-std);
    margin-right: var(--space-1);
  }
`;

export default Button;
