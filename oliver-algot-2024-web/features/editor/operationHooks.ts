/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {useAppSelector} from 'src/hooks';
import resolveOperation from 'src/resolveOperation';
import {useMemo} from 'react';
import baseOperations from 'src/BaseOperations';
import {OperationId} from 'src/Operation';

export function useResolveOperation() {
  const operations = useAppSelector(state => state.editor.operations);
  return (id: OperationId) => resolveOperation(operations, id);
}

export function useAllOperations() {
  const ops = useAppSelector(state => state.editor.operations);
  return useMemo(
    () => [...Object.values(ops), ...Object.values(baseOperations)],
    [ops]
  );
}
