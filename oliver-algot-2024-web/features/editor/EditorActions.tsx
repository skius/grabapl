import React, {useEffect} from 'react';
import classNames from 'classnames';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import styles from './EditorActions.module.scss';
import {selectTool} from 'features/tools/toolsReducer';
import {useState} from 'react';
import {EditorTool, EditorToolId, editorTools} from './OperationEditor';
import {Operation} from 'src/Operation';
import {fromActionStack} from './editorReducer';

const keyToTool: Record<string, EditorToolId> = {
  ArrowLeft: 'editorBackward',
  ArrowRight: 'editorForward',
  ArrowDown: 'editorInto',
  ArrowUp: 'editorOut',
};

const disabled: Record<EditorToolId, boolean> = {};
const canSelect: Record<EditorToolId, boolean> = {};

export default function EditorActions() {
  const dispatch = useAppDispatch();
  const toolsArray = Object.values(editorTools);

  const operation = useAppSelector(
    state => state.editor.operations[state.editor.selectedOperation!]
  );

  const selectedNodes = useAppSelector(state => state.tools.selectedNodes);

  const currentEditor = useAppSelector(state => {
    const editor = state.editor.operationsEditor[operation.id];
    return editor.patternEditors[editor.currentEditorIndex];
  });

  const {actionStack} = currentEditor;

  for (const tool of toolsArray) {
    disabled[tool.id] = !isEnabled(
      tool.id,
      actionStack,
      currentEditor.reachablePaths,
      operation
    );
    canSelect[tool.id] = selectedNodes.length <= 1;
  }

  // Assuming the first two tools are the ones you want to bind to arrow keys
  const handleArrowKeyPress = (event: KeyboardEvent) => {
    if (!(event.key in keyToTool)) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const tool = keyToTool[event.key];
    if (!tool) return;

    if (!disabled[tool] && canSelect[tool]) {
      dispatch(selectTool({id: tool, selectedType: null}));
    }
  };

  useEffect(() => {
    window.addEventListener('keydown', handleArrowKeyPress);

    return () => {
      window.removeEventListener('keydown', handleArrowKeyPress);
    };
  }, []);

  return (
    <div className={styles.bar}>
      {toolsArray.map((tool, idx) => (
        <EditorItem
          onClick={() =>
            dispatch(selectTool({id: tool.id, selectedType: null}))
          }
          tool={tool}
          key={idx}
          disabled={disabled[tool.id]}
          canBeSelected={canSelect[tool.id]}
        />
      ))}
    </div>
  );
}

function isEnabled(
  tool: EditorToolId,
  actionStack: number[],
  paths: string[],
  operation: Operation
) {
  switch (tool) {
    case 'editorForward':
      return (
        fromActionStack(actionStack) !==
        fromActionStack([operation.demoSemantics!.actions.length])
      );
    case 'editorBackward':
      return fromActionStack(actionStack) !== fromActionStack([0]);
    case 'editorInto': {
      const idx = paths.indexOf(fromActionStack(actionStack));
      if (idx !== -1) {
        return paths.at(idx + 1)?.startsWith(paths[idx]);
      } else {
        return false;
      }
    }
    case 'editorOut':
      return true;
    default:
      return true;
  }
}

function EditorItem({
  tool,
  onClick,
  disabled,
  canBeSelected,
  selected,
}: {
  onClick: () => void;
  tool: EditorTool;
  disabled: boolean;
  canBeSelected: boolean;
  selected?: boolean;
}) {
  const isSelected = useAppSelector(
    state => state.tools.selectedTool === tool.id || selected
  );

  const [isHovered, setIsHovered] = useState(false);

  return (
    <button
      disabled={disabled}
      onClick={canBeSelected ? onClick : undefined}
      className={classNames(
        isSelected ? styles.buttonActive : styles.button,
        isHovered && canBeSelected && !disabled && styles.hovered,
        (!canBeSelected || disabled) && styles.unselectable
      )}
      onMouseEnter={() => disabled || setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      title={tool.name}
    >
      <div style={{padding: '5px'}} className="material-icons">
        {tool.icon}
      </div>
    </button>
  );
}
