/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {createAsyncThunk, createSlice, PayloadAction} from '@reduxjs/toolkit';
import {
  deleteOperation,
  editorSelectTableNodeForEdit,
  finishDemo,
  setDemoOperation,
  createAndOpenOperation,
} from 'features/editor/editorReducer';
import {reset} from 'features/playground/playgroundReducer';
import executeToolOnStore from 'features/tools/executeToolOnStore';
import {RootState} from 'src/store';
import {resolveTool} from 'src/resolveOperation';
import {BaseThunkAPI} from '@reduxjs/toolkit/dist/createAsyncThunk';
import {isAbstractNodeArgument, ToolArgument, ToolId} from 'features/tools/Tool';

export enum BuiltInTool {
  Cursor,
  ValueTool,
  Resizing,
}

interface ToolsState {
  selectedNodes: ToolArgument[];
  builtInTool: BuiltInTool;
  selectedTool: null | ToolId;
  selectedType: string | null;
}

const initialState: ToolsState = {
  selectedNodes: [],
  builtInTool: BuiltInTool.Cursor,
  selectedTool: null,
  selectedType: null,
};

function toggleSelectedItem<T>(selection: T[], item: T) {
  if (!selection.includes(item)) selection.push(item);
  else selection = selection.splice(selection.indexOf(item), 1);
}

/** If enough inputs have been selected, dispatches the currently selected tool.
 * @return True iff the tool was dispatched. */
function dispatchToolIfReady({
  getState,
  dispatch,
}: BaseThunkAPI<RootState, unknown>): boolean {
  const state = getState() as RootState;
  if (state.tools.selectedTool === null) return false;
  const tool = resolveTool(state.editor.operations, state.tools.selectedTool);
  if (state.tools.selectedNodes.length === tool.inputs.length) {
    executeToolOnStore(
      dispatch,
      state,
      state.tools.selectedNodes,
      state.tools.selectedTool
    );
    return true;
  }
  return false;
}

/**
 * Action that selects a node and, if the correct number of nodes for the
 * current tool have been selected, executes the tool operation.
 */
export const selectNode = createAsyncThunk(
  'tools/selectNode',
  (arg: ToolArgument, thunkAPI) =>
    dispatchToolIfReady(thunkAPI as BaseThunkAPI<RootState, unknown>)
);

/**
 * Action that selects a tool. In case the tool does not have inputs, dispatches
 * the tool immediately and deselects the tool.
 */
export const selectTool = createAsyncThunk(
  'tools/selectTool',
  (args: {id: ToolId | null; selectedType: string | null}, thunkAPI) =>
    dispatchToolIfReady(thunkAPI as BaseThunkAPI<RootState, unknown>)
);

export const executeTool = createAsyncThunk(
  'tools/executeTool',
  ({args, tool}: {args: ToolArgument[]; tool: ToolId}, thunkAPI) => {
    const api = thunkAPI as BaseThunkAPI<RootState, unknown>;
    executeToolOnStore(api.dispatch, api.getState(), args, tool);
  }
);

const toolsSlice = createSlice({
  name: 'tools',
  initialState,
  reducers: {
    selectBuiltInTool(state, action: PayloadAction<BuiltInTool>) {
      state.builtInTool = action.payload;
      state.selectedTool = null;
    },
    unselectNodes(state) {
      state.selectedNodes = [];
    },
    resetAll(state) {
      state.selectedNodes = [];
      state.selectedTool = null;
    },
    removeSelectedTool(state) {
      state.selectedTool = null;
    },
  },
  extraReducers: builder => {
    // We need to reset the selected nodes if any of these actions is
    // dispatched, as they might change the nodes available.
    builder.addCase(finishDemo, () => initialState);
    builder.addCase(setDemoOperation, () => initialState);
    builder.addCase(createAndOpenOperation, () => initialState);
    builder.addCase(reset, () => initialState);
    builder.addCase(deleteOperation, () => initialState);
    builder.addCase(editorSelectTableNodeForEdit, state => {
      state.selectedNodes = [];
      state.selectedTool = null;
    });
    builder.addCase(selectNode.pending, (state, action) => {
      toggleSelectedItem(state.selectedNodes, action.meta.arg);
    });
    builder.addCase(selectNode.fulfilled, (state, action) => {
      if (action.payload) {
        // The tool was dispatched. Deselect the nodes
        state.selectedNodes = [];
      }
    });
    builder.addCase(selectTool.pending, (state, action) => {
      if (state.selectedTool === action.meta.arg.id) {
        state.selectedTool = null;
        state.selectedType = null;
        state.selectedNodes = [];
      } else {
        state.selectedTool = action.meta.arg.id;
        state.selectedType = action.meta.arg.selectedType;
      }
    });
    builder.addCase(selectTool.fulfilled, (state, action) => {
      if (action.payload) {
        // The tool was already dispatched. Deselect the tool again
        state.selectedNodes = [];
        state.selectedTool = null;
        state.selectedType = null;
      }
    });
  },
});

export default toolsSlice.reducer;
export const {selectBuiltInTool, resetAll, removeSelectedTool, unselectNodes} =
  toolsSlice.actions;
