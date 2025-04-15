/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

const TitleBar = styled.div`
  background-color: var(--interact-tint);
  text-align: center;
  font-weight: 500;
  font-size: var(--font-sm);
  color: white;
  padding: 0 16px;
  line-height: var(--top-bar-height);
  height: var(--top-bar-height);
  box-shadow: 0 0 3px 1px rgba(0, 0, 0, 0.14);
  position: relative;
  user-select: none;

  button {
    position: absolute;
    left: 12px;
    top: 6px;
    color: white;
  }
`;

export default TitleBar;
