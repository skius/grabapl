import styles from './ContextBar.module.scss';
import {AppDispatch} from 'src/store';
import IconButton from './IconButton';
import {useAppDispatch} from 'src/hooks';
import {resetAll} from 'features/tools/toolsReducer';

export default function ({
  title,
  onClose,
  undo,
  reset,
}: {
  title: string;
  onClose?: () => void;
  undo: () => Parameters<AppDispatch>[0];
  reset?: () => Parameters<AppDispatch>[0];
}) {
  const dispatch = useAppDispatch();

  return (
    <div className={styles.bar}>
      <div>{onClose && <IconButton onClick={onClose} icon="close" />}</div>

      <h1>{title}</h1>

      <div className={styles.buttons}>
        {reset && (
          <IconButton onClick={() => dispatch(reset())} icon="delete_forever" />
        )}
        <IconButton
          onClick={() => {
            dispatch(undo());
            dispatch(resetAll());
          }}
          icon="undo"
        />
      </div>
    </div>
  );
}
