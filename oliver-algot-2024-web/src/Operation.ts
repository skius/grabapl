/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {DemoSemantics, Pattern, PatternId} from 'src/DemoSemantics';
import {ConcreteValue} from 'src/ConcreteValue';

export type OperationId = string & {readonly brand?: unique symbol};

export interface Operation {
  type: 'Operation';
  id: OperationId;
  name: string;
  icon: string;
  inputs: PatternId[];
  inputTypes: string[];
  patterns: Record<PatternId, Pattern>;
  demoSemantics?: DemoSemantics;
  documentation?: string;
  literals?: ConcreteValue[];
  isQuery: boolean;
  isUserDefined: boolean;
}
