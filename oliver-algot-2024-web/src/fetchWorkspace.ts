import {createAsyncThunk} from '@reduxjs/toolkit';
import {PlaygroundState} from 'features/playground/playgroundReducer';
import {EditorState} from 'features/editor/editorReducer';

export interface Workspace {
  _id: string;
  playground: PlaygroundState;
  editor: EditorState;
}

export const fetchWorkspace = createAsyncThunk(
  'editor/fetchWorkspace',
  async (workspaceId: string) => {
    const res = await fetch(`/api/workspaces/${workspaceId}`);
    return (await res.json()) as Workspace;
  }
);
