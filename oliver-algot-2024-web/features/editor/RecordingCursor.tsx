/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import styled, {keyframes} from 'styled-components';

const s = '17px';
const animationOptions =
  '2s cubic-bezier(0.37, 0, 0.63, 1) infinite alternate-reverse';

const fadeLight = keyframes`
  from {
  }

  to {
    box-shadow: 0 0 1px 0 rgba(0,0,0,0.50);
  }
`;

const fadeBulb = keyframes`
  from {
    opacity: 0;
  }

  to {
    opacity: 1;
  }
`;

const RecordingCursorLi = styled.li`
  margin: 5px 0;
  list-style: none;
  position: relative;
  z-index: 0;

  &::after {
    position: absolute;
    top: 8px;
    content: ' ';
    height: 1px;
    background: linear-gradient(to right, var(--border-gray) 50%, transparent);
    width: 100%;
    display: block;
    z-index: -1;
  }
`;

const Light = styled.div`
  width: ${s};
  height: ${s};
  border-radius: 50%;
  background-image: radial-gradient(circle at 50% 50%, #e0e0e0 0%, #dddddd 50%);
  animation: ${fadeLight} ${animationOptions};
`;

const Bulb = styled.div`
  background-image: radial-gradient(circle at 50% 50%, #ff3d50 0%, #dd3545 50%);
  opacity: 0;
  width: ${s};
  height: ${s};
  border-radius: 50%;
  animation: ${fadeBulb} ${animationOptions};
`;

const Text = styled.div`
  font-size: var(--font-xs);
  color: var(--text-gray);
  margin-top: 4px;
  margin-left: 30px;
`;

export default function RecordingCursor() {
  return (
    <RecordingCursorLi>
      <Light>
        <Bulb />
      </Light>
      <Text>Actions you demonstrate will be recorded here</Text>
    </RecordingCursorLi>
  );
}
