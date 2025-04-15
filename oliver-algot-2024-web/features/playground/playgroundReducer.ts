/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {createSlice, PayloadAction} from '@reduxjs/toolkit';
import {ConcreteGraph} from 'src/ConcreteGraph';
import {fetchWorkspace} from 'src/fetchWorkspace';

export interface PlaygroundState {
  graph: ConcreteGraph;
  actions: {
    state: ConcreteGraph;
  }[];
}

export const initialPlaygroundState: PlaygroundState = {
  graph: {
    nextId: 1,
    nodes: {},
  },
  actions: [],
};

const playgroundSlice = createSlice({
  name: 'playground',
  initialState: initialPlaygroundState,
  reducers: {
    performedOperation: (
      state,
      {payload: {newGraph}}: PayloadAction<{newGraph: ConcreteGraph}>
    ) => ({
      ...initialPlaygroundState,
      graph: newGraph,
      actions: [...state.actions, {state: state.graph}],
    }),
    undo: state =>
      state.actions.length > 0
        ? {
            ...initialPlaygroundState,
            graph: state.actions[state.actions.length - 1].state,
            actions: state.actions.slice(0, -1),
          }
        : state,
    reset: () => initialPlaygroundState,
  },
  extraReducers: builder => {
    builder.addCase(fetchWorkspace.fulfilled, (state, action) => {
      const {playground} = action.payload;
      return playground;
    });
  },
});

export default playgroundSlice.reducer;

export const {performedOperation, reset, undo} = playgroundSlice.actions;
