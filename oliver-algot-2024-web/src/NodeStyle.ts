/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {OperationId} from 'src/Operation';

export interface NodeStyle {
  nodeBackground?: string;
  nodeWidth?: number;
  nodeHeight?: number;
  flexDirection?: 'row' | 'column' | 'none';
  flexAlign?:
    | 'start'
    | 'end'
    | 'center'
    | 'stretch'
    | 'space-between'
    | 'space-around';
  flexWrap?: 'wrap' | 'nowrap' | 'wrap-reverse';
  borderRadius?: number;
  padding?: number;
  margin?: number;
  marginLeft?: number;
  marginTop?: number;
  onClick?: OperationId;
  onRightClick?: OperationId;
  onMouseEnter?: OperationId;
  onMouseLeave?: OperationId;
  editable?: boolean;
  hidden?: boolean;
  fontColor?: string;
  fontSize?: number;
  borderWidth?: number;
  borderColor?: string;
  textAlign?: 'left' | 'center' | 'right';
}
