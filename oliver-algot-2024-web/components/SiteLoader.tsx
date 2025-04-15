/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styles from './SiteLoader.module.scss';

export default function () {
  return (
    <div className={styles.container}>
      <div className={styles.loader} />
      <div>Preparing your workspace...</div>
    </div>
  );
}
