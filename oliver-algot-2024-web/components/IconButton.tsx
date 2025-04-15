/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

const IButton = styled.div<{active?: boolean}>`
  padding: 4px;
  color: var(--interaction-gray);

  &:not(:disabled):hover {
    color: black;
  }

  &:disabled {
  }

  .material-icons-outlined {
    font-size: var(--font-md);
  }

  ${({active}) => active && 'color: var(--interact-tint-lighter)'}
`;

export default function IconButton({
  onClick,
  icon,
  disabled,
  className,
  active,
}: {
  onClick?: (e: React.MouseEvent) => void;
  icon: string;
  disabled?: boolean;
  className?: string;
  active?: boolean;
}) {
  return (
    <IButton
      onClick={disabled ? undefined : onClick}
      active={active}
      className={className}
      style={{cursor: 'pointer'}}
    >
      <span className="material-icons-outlined">{icon}</span>
    </IButton>
  );
}
