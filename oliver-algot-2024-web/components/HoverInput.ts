/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled from 'styled-components';

/**
 * A text input that normally looks like a label. If the user hovers it,
 * it changes its style to indicate it can be edited.
 */
const HoverInput = styled.input`
  appearance: none;
  border: 0;
  background: 0;
  border-radius: var(--small-radius);
  padding: var(--space-2);
  color: black !important;

  &:focus {
    outline: 2px solid var(--interact-tint);
    background: white !important;
  }
`;

export default HoverInput;
