/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {configureStore} from '@reduxjs/toolkit';
import editorReducer from 'features/editor/editorReducer';
import playgroundReducer from 'features/playground/playgroundReducer';
import toolsReducer from 'features/tools/toolsReducer';

const store = configureStore({
  reducer: {
    editor: editorReducer,
    playground: playgroundReducer,
    tools: toolsReducer,
  },
  middleware: getDefaultMiddleware =>
    getDefaultMiddleware({
      immutableCheck: false,
      serializableCheck: false,
    }),
});

store.subscribe(() => console.log('State after dispatch: ', store.getState()));

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

export default store;
