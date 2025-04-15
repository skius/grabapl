import {
  editorBackward,
  editorForward,
  editorInto,
  editorOut,
} from './editorReducer';
import {AppDispatch} from 'src/store';
import {AbstractNodeDescriptor} from 'src/DemoSemantics';

export type EditorToolId = string & {readonly brand?: unique symbol};

export interface EditorTool {
  type: 'EditorTool';
  inputs: {name: string}[];
  inputTypes: string[];
  id: EditorToolId;
  icon: string;
  name: string;
  perform: (dispatch: AppDispatch, nodes: AbstractNodeDescriptor[]) => void;
  documentation?: string;
}

export const editorTools: Record<EditorToolId, EditorTool> = {
  editorBackward: {
    type: 'EditorTool',
    inputs: [],
    inputTypes: [],
    id: 'editorBackward',
    icon: 'arrow_back',
    name: 'Step Back',
    perform: dispatch => dispatch(editorBackward()),
    documentation: 'Step back in editor',
  },
  editorForward: {
    type: 'EditorTool',
    inputs: [],
    inputTypes: [],
    id: 'editorForward',
    icon: 'arrow_forward',
    name: 'Step Forward',
    perform: dispatch => dispatch(editorForward()),
    documentation: 'Step forward in editor',
  },
  editorInto: {
    type: 'EditorTool',
    inputs: [],
    inputTypes: [],
    id: 'editorInto',
    icon: 'arrow_downward',
    name: 'Step Into Action',
    perform: dispatch => dispatch(editorInto()),
    documentation: 'Step into operation',
  },
  editorOut: {
    type: 'EditorTool',
    inputs: [],
    inputTypes: [],
    id: 'editorOut',
    icon: 'arrow_upward',
    name: 'Step Out Of Action',
    perform: dispatch => dispatch(editorOut()),
    documentation: 'Step out of operation',
  },
};
