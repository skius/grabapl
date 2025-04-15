/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useAppSelector} from 'src/hooks';
import resolveOperation from 'src/resolveOperation';

/**
 * Returns the current operation under demonstration.
 */
export default function useOuD() {
  return useAppSelector(state =>
    state.editor.selectedOperation
      ? resolveOperation(
          state.editor.operations,
          state.editor.selectedOperation
        )
      : null
  );
}
