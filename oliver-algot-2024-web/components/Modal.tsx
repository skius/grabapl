/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styles from './Modal.module.scss';
import {PropsWithChildren} from 'react';
import ReactModal from 'react-modal';

export default function Modal({
  title,
  children,
  ...props
}: PropsWithChildren<{
  title: string;
  isOpen: boolean;
  onRequestClose: () => void;
}>) {
  return (
    <ReactModal
      {...props}
      contentLabel={title}
      className={styles.content}
      overlayClassName={styles.overlay}
    >
      <div className={styles.titleBar}>
        <div className={styles.title}>{title}</div>
        <button onClick={props.onRequestClose}>
          <span className="material-icons-outlined">close</span>
        </button>
      </div>
      <div className={styles.inner}>{children}</div>
    </ReactModal>
  );
}
