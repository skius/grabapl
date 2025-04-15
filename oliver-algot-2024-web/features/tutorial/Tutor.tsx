/* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. */
import styles from './Tutor.module.scss';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {useState} from 'react';
import {RootState} from 'src/store';
import TitleBar from 'components/TitleBar';
import {animated, useSpring} from 'react-spring';
import Button from 'components/Button';
import {setTutorialOpen} from 'features/editor/editorReducer';
import Draggable from 'react-draggable';
import LocalizedStrings from 'react-localization';
import strings from './strings.json';
import {NUMBER_TYPE_ID} from 'src/ConcreteValue';

interface Step {
  instruction: string;
  hint: string;
  achievedGoal: (state: RootState) => boolean;
  goalMessage: string;
}

type Tutorial = Step[];

const allText = new LocalizedStrings(strings);
allText.setLanguage('en');
const instructions = allText.instructions;

const tutorialLibrary: Record<string, Tutorial> = {
  tutorial1: [
    {
      instruction: instructions.instruction1,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction2,
      hint: '',
      goalMessage: 'Well done!',
      achievedGoal(state) {
        return Object.keys(state.playground.graph.nodes).length >= 2;
      },
    },
    {
      instruction: instructions.instruction3,
      hint: '',
      goalMessage: 'Excellent!',
      achievedGoal(state) {
        return (
          Object.keys(state.playground.graph.nodes).length >= 2 &&
          Object.entries(state.playground.graph.nodes).some(
            v => v[1].outgoingEdges.length > 0
          )
        );
      },
    },
    {
      instruction: instructions.instruction4,
      hint: '',
      goalMessage: 'Nice progress!',
      achievedGoal(state) {
        return Object.values(state.playground.graph.nodes).some(
          v =>
            'value' in v &&
            v.outgoingEdges.length > 0 &&
            v.value.type === NUMBER_TYPE_ID &&
            v.value.value === 1
        );
      },
    },
    {
      instruction: instructions.instruction5,
      hint: '',
      goalMessage: 'Well done!',
      achievedGoal(state) {
        return Object.entries(state.playground.graph.nodes).some(
          v => v[1].incomingEdges.length === 0 && v[1].outgoingEdges.length >= 2
        );
      },
    },
    {
      instruction: instructions.instruction6,
      hint: '',
      goalMessage: 'Well done!',
      achievedGoal(state) {
        const nodes = Object.entries(state.playground.graph.nodes);
        return (
          nodes.some(
            v =>
              v[1].incomingEdges.length === 0 && v[1].outgoingEdges.length === 2
          ) &&
          nodes.some(
            v =>
              v[1].incomingEdges.length === 1 && v[1].outgoingEdges.length === 1
          ) &&
          nodes.some(
            v =>
              v[1].incomingEdges.length === 2 && v[1].outgoingEdges.length === 0
          )
        );
      },
    },
    {
      instruction: instructions.instruction7,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction8,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        return Object.values(state.editor.operations).length >= 1;
      },
    },
    {
      instruction: instructions.instruction9,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        return Object.values(state.editor.operations).some(
          op => op.name.toUpperCase() === 'ADD TWINS'
        );
      },
    },
    {
      instruction: instructions.instruction10,
      hint: '',
      goalMessage: 'First step completed!',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'ADD TWINS'
        );
        return op ? op.inputs.length === 1 : false;
      },
    },
    {
      instruction: instructions.instruction11,
      hint: '',
      goalMessage: 'Second step completed!',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'ADD TWINS'
        );
        if (!op || op.demoSemantics!.actions.length === 0) return false;
        return op.demoSemantics!.actions.every(
          action => action.operation === 'addChild'
        );
      },
    },
    {
      instruction: instructions.instruction12,
      hint: '',
      goalMessage: 'Function completed!',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'ADD TWINS'
        );
        if (op && op.demoSemantics?.actions?.length === 2) {
          return !!op.demoSemantics?.actions?.every(
            action => action.operation === 'addChild'
          );
        }
        return false;
      },
    },
    {
      instruction: instructions.instruction13,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction14,
      hint: '',
      goalMessage: 'Well done!',
      achievedGoal(state) {
        return Object.values(state.editor.operations).some(
          op => op.name.toUpperCase() === 'CONDITIONAL DECREMENT'
        );
      },
    },
    {
      instruction: instructions.instruction15,
      hint: '',
      goalMessage: 'Great! Almost there…',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'CONDITIONAL DECREMENT'
        );
        return op ? JSON.stringify(op).includes('isZero') : false;
      },
    },
    {
      instruction: instructions.instruction16,
      hint: '',
      goalMessage: 'Function all done!',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'CONDITIONAL DECREMENT'
        );
        return op ? JSON.stringify(op).includes('decrement') : false;
      },
    },
    {
      instruction: instructions.instruction17,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        const op = Object.values(state.editor.operations).find(
          op => op.name.toUpperCase() === 'CONDITIONAL DECREMENT'
        );
        if (!op || op.inputs.length !== 1) return false;
        return (
          state.editor.operationsEditor[op.id].patternEditors[0].exampleValues[
            op.inputs[0]
          ] !== 0
        );
      },
    },
    {
      instruction: instructions.instruction18,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction19,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction20,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        const playground = state.playground.graph.nodes;
        return Object.values(playground).some(
          node =>
            node.incomingEdges.length === 0 &&
            node.outgoingEdges.length === 2 &&
            node.outgoingEdges.every(
              edge =>
                playground[edge.target].incomingEdges.length === 1 &&
                playground[edge.target].outgoingEdges.length === 0
            )
        );
      },
    },
    {
      instruction: instructions.instruction21,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        if (!state.editor.selectedOperation) return false;
        const op = state.editor.operations[state.editor.selectedOperation];
        const patterns = Object.values(op.patterns);
        return (
          op.inputs.length === 1 &&
          patterns.length === 3 &&
          patterns
            .filter(p => p.id !== op.inputs[0])
            .every(
              p =>
                p.outgoing.length === 0 &&
                p.incoming.length === 1 &&
                p.incoming[0] === op.inputs[0]
            )
        );
      },
    },
    {
      instruction: instructions.instruction22,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction23,
      hint: '',
      goalMessage: '',
      achievedGoal(state) {
        if (!state.editor.selectedOperation) return false;
        const op = state.editor.operations[state.editor.selectedOperation];

        if (
          op.demoSemantics!.actions.length !== 1 ||
          op.demoSemantics!.actions[0].operation !== 'sum'
        ) {
          return false;
        }

        const [inp0, inp1, inp2] = op.demoSemantics!.actions[0].inputs;
        if (
          inp0.type !== 'PatternMatch' ||
          inp1.type !== 'PatternMatch' ||
          inp2.type !== 'PatternMatch'
        ) {
          return false;
        }

        return (
          op.patterns[inp2.pattern].outgoing.includes(inp0.pattern) &&
          op.patterns[inp2.pattern].outgoing.includes(inp1.pattern)
        );
      },
    },
    {
      instruction: instructions.instruction24,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction25,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
    {
      instruction: instructions.instruction26,
      hint: '',
      goalMessage: '',
      achievedGoal: () => true,
    },
  ],
};

interface TutorialState {
  currentStep: number;
  showHint: boolean;
  tutorialKey: string;
}

const initialState: TutorialState = {
  currentStep: 0,
  showHint: false,
  tutorialKey: 'tutorial1',
};

export default function Tutor() {
  const [tutorState, setTutorState] = useState(initialState);
  const dispatch = useAppDispatch();
  const state = useAppSelector(state => state);

  const currentTutorial = tutorialLibrary[tutorState.tutorialKey];
  const currentStep = currentTutorial[tutorState.currentStep];
  const achievedGoal = currentStep.achievedGoal(state);

  return (
    <Draggable handle=".dragHandle">
      <div className={styles.tutor}>
        <TitleBar className="dragHandle">
          <button onClick={() => dispatch(setTutorialOpen(false))}>
            <span className="material-icons-outlined">close</span>
          </button>
          Tutorial
        </TitleBar>
        <div className={styles.inner}>{currentStep.instruction}</div>
        <div className={styles.buttonBar}>
          {achievedGoal && !!currentStep.goalMessage && (
            <div className={styles.success}>
              <span className="material-icons">check_circle</span>
              {currentStep.goalMessage}
            </div>
          )}

          {tutorState.currentStep + 1 < currentTutorial.length ? (
            <Button
              className={styles.button}
              autoWidth={true}
              disabled={!achievedGoal}
              onClick={() =>
                setTutorState({
                  ...tutorState,
                  currentStep: tutorState.currentStep + 1,
                })
              }
            >
              Next
            </Button>
          ) : (
            <span />
          )}
        </div>
        <ProgressBar
          progress={(tutorState.currentStep + 1) / currentTutorial.length}
        />
      </div>
    </Draggable>
  );
}

function ProgressBar({progress}: {progress: number}) {
  const style = useSpring({
    width: `${progress * 100}%`,
  });
  return (
    <div className={styles.progressbar}>
      <animated.div className={styles.progressbarBar} style={style} />
    </div>
  );
}
