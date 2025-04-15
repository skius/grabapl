import {useState} from 'react';
import styled from 'styled-components';

const ToolTipText = styled.div<{x: number; y: number}>`
  position: absolute;
  top: ${({y}) => y}px;
  left: ${({x}) => x}px;
  width: 500px;
  height: 500px;
  background-color: yellow;
  color: red;
  padding: 0.5rem;
  border-radius: 4px;
`;

export function ToolTip({
  text,
  children,
}: React.PropsWithChildren<{text: string}>) {
  const [showToolTip, setShowToolTip] = useState(false);
  const [position, setPosition] = useState({x: 0, y: 0});

  return (
    <div
      style={{position: 'absolute', width: '100%', height: '100%'}}
      onMouseMove={e => setPosition({x: e.clientX, y: e.clientY})}
      onMouseEnter={() => setShowToolTip(true)}
      onMouseLeave={() => setShowToolTip(false)}
    >
      {showToolTip && (
        <ToolTipText x={position.x} y={position.y}>
          {text}
        </ToolTipText>
      )}
      {children}
    </div>
  );
}
