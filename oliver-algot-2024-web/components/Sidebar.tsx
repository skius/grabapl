/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styles from 'components/Sidebar.module.scss';
import {PropsWithChildren} from 'react';
import classNames from 'classnames';
import {Tab} from '@headlessui/react';

export default function Sidebar({
  children,
  left,
  className,
  style,
}: PropsWithChildren<{
  left: boolean;
  className?: string;
  style?: React.CSSProperties;
}>) {
  return (
    <section
      className={classNames(styles.sidebar, left && styles.left, className)}
      style={style}
    >
      {children}
    </section>
  );
}

export function Tabs({
  tabs,
  borderTop,
}: {
  tabs: [string, string, boolean?][];
  borderTop?: boolean;
}) {
  return (
    <Tab.List
      className={classNames(styles.tabbar, borderTop && styles.tabbarBorderTop)}
    >
      {tabs.map(([icon, label, invisible]) => (
        <Tab
          key={label}
          className={({selected}) =>
            classNames(
              styles.tab,
              selected && styles.tabActive,
              invisible && styles.tabInvisible
            )
          }
        >
          {({selected}) => (
            <>
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  gap: '3px',
                }}
              >
                <span
                  className={`material-icons${selected ? '' : '-outlined'}`}
                >
                  {icon}
                </span>
                <span>{label}</span>
              </div>
            </>
          )}
        </Tab>
      ))}
    </Tab.List>
  );
}

export function TabPanel({
  children,
  space = true,
  visible = true,
  scroll = true,
}: PropsWithChildren<{
  space?: boolean;
  visible?: boolean;
  scroll?: boolean;
}>) {
  return (
    <Tab.Panel
      className={space ? styles.tabContent : ''}
      style={{
        display: visible ? 'block' : 'none',
        overflowY: scroll ? 'scroll' : 'hidden',
      }}
    >
      {children}
    </Tab.Panel>
  );
}
