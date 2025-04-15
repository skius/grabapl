/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

const ScrollView = styled.div`
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1;
  overflow: auto;
  display: flex;
  flex-direction: column;
  overscroll-behavior: contain;
  -webkit-overflow-scrolling: touch;
`;

export default ScrollView;
