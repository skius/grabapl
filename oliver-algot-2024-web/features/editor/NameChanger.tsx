import Modal from 'components/Modal';
import {useEffect, useState} from 'react';
import {nameForAbstractNode} from 'src/AbstractNodeUtils';
import {PatternMatchAbstractNodeDescriptor} from 'src/DemoSemantics';
import styles from 'features/editor/AbstractNode.module.scss';
import {Operation} from 'src/Operation';
import {useAppDispatch} from 'src/hooks';
import {changePatternNodeName} from './editorReducer';
import styled from 'styled-components';
import Button from 'components/Button';

const RoundButton = styled.button`
  background: var(--interact-tint);
  box-shadow: 0 0 1px 0 rgba(0, 0, 0, 0.5);
  color: white;
  text-align: center;
  border-radius: var(--big-radius);
  height: 25px;
  line-height: 25px;
  font-size: var(--font-xs);
  align-items: center;
  vertical-align: middle;

  &:hover {
    background-color: var(--interact-tint);
  }
`;

const InputField = styled.input`
  vertical-align: middle;
`;

const Container = styled.div`
  display: flex;
  flex-direction: row;
  justify-content: space-between;
`;

function OkButton({onClick}: {onClick: () => void}) {
  return (
    <Button onClick={onClick} autoWidth={true}>
      <span>OK</span>
    </Button>
  );
}

export function NameField({name}: {name: string}) {
  return <div className={styles.nameField}>{name}</div>;
}

export default function NameChanger({
  abstractNode,
  operation,
}: {
  abstractNode: PatternMatchAbstractNodeDescriptor;
  operation: Operation;
}) {
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState(
    operation.patterns[abstractNode.pattern].name
  );

  function close() {
    setOpen(false);
  }

  useEffect(() => {
    if (!open) {
      dispatch(
        changePatternNodeName({
          operation: operation.id,
          pattern: abstractNode.pattern,
          name: filter,
        })
      );
    }
  }, [open]);

  const dispatch = useAppDispatch();

  return (
    <>
      <div className={styles.input} onClick={() => setOpen(true)}>
        {nameForAbstractNode(abstractNode, operation)}
      </div>
      <Modal
        title="Change Input Node Name"
        isOpen={open}
        onRequestClose={() => setOpen(false)}
      >
        <Container>
          <InputField
            autoFocus={true}
            type="text"
            value={filter}
            onChange={e => {
              const input = e.target.value;
              if (input.length > 5) return;
              setFilter(e.target.value);
            }}
          />
          <OkButton onClick={() => close()} />
        </Container>
      </Modal>
    </>
  );
}
