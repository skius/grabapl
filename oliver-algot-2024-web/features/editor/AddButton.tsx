/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styled from 'styled-components';
import Button from 'components/Button';

export const RoundButton = styled.button`
  background: var(--interact-tint);
  box-shadow: 0 0 1px 0 rgba(0, 0, 0, 0.5);
  color: white;
  text-align: center;
  border-radius: var(--big-radius);
  height: 25px;
  line-height: 25px;
  font-size: var(--font-xs);
  display: flex;
  align-items: center;
  padding: var(--space-3);

  &:hover {
    background-color: var(--interact-tint);
  }
`;

export default function AddButton({
  onClick,
  label,
}: {
  onClick: () => void;
  label: string;
}) {
  return (
    <Button onClick={onClick} autoWidth={true}>
      <span className="material-icons-outlined">add</span>
      <span>{label}</span>
    </Button>
  );
}
