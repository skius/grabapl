/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useEffect, useState} from 'react';
import styled from 'styled-components';
import Modal from '../../components/Modal';
import styles from './IconPicker.module.scss';
import iconData from './icons.json';
import IconButton from 'components/IconButton';

const HoverButton = styled.button`
  appearance: none;
  border: 0;
  background: 0;
  border-radius: var(--small-radius);
  padding: 6px;
  display: inline-block;
  vertical-align: -5px;

  &:disabled {
    color: inherit;
    cursor: default;
  }

  &:not(:disabled):hover {
    background-color: var(--hover-gray);
  }

  &:not(:disabled):active {
    background-color: var(--interact-tint);
  }
`;

export default function IconPicker({
  icon,
  onChange,
  disabled,
}: {
  icon: string;
  onChange: (icon: string) => void;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState('');
  const [icons, setIcons] = useState<typeof iconData.icons>([]);

  useEffect(() => {
    setIcons(
      iconData.icons.filter(
        icon =>
          icon.name.includes(filter) || icon.tags.some(t => t.includes(filter))
      )
    );
  }, [filter]);

  function close() {
    setOpen(false);
    setFilter('');
  }

  return (
    <>
      <IconButton
        disabled={disabled}
        onClick={() => setOpen(true)}
        icon={icon}
      />
      <Modal
        title="Select Icon"
        isOpen={open}
        onRequestClose={() => setOpen(false)}
      >
        <input
          autoFocus={true}
          type="text"
          value={filter}
          onChange={e => setFilter(e.target.value)}
        />
        <div className={styles.icons}>
          {icons.map((icon, i) => (
            <HoverButton
              onClick={() => {
                onChange(icon.name);
                close();
              }}
              key={`${icon.name}-${i}`}
            >
              <span className="material-icons-outlined">{icon.name}</span>
            </HoverButton>
          ))}
        </div>
      </Modal>
    </>
  );
}
