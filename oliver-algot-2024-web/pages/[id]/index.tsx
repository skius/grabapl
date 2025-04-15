/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import type {NextPage} from 'next';
import WorkspaceSidebar from 'features/editor/WorkspaceSidebar';
import {Provider} from 'react-redux';
import store from 'src/store';
import DemonstrationView from 'features/editor/DemonstrationView';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {useRouter} from 'next/router';
import {useEffect} from 'react';
import debounce from 'lodash.debounce';
import SiteLoader from 'components/SiteLoader';
import ToolsSidebar from 'features/tools/ToolsSidebar';
import Head from 'next/head';
import {fetchWorkspace} from 'src/fetchWorkspace';
import ContextBar from 'components/ContextBar';
import {reset, undo} from 'features/playground/playgroundReducer';
import {produce} from 'immer';
import 'react-grid-layout/css/styles.css';
import Playground from 'features/playground/Playground';
import Tutor from 'features/tutorial/Tutor';

function App() {
  const router = useRouter();
  const name = useAppSelector(state => state.editor.name);
  const tutorialOpen = useAppSelector(state => state.editor.tutorialOpen);
  const graph = useAppSelector(state => state.playground.graph);

  const dispatch = useAppDispatch();
  useEffect(() => {
    if (router.query.id) dispatch(fetchWorkspace(router.query.id as string));
  }, [router.query]);

  const loading = useAppSelector(state => state.editor.loading);

  return loading ? (
    <SiteLoader />
  ) : (
    <div className="container">
      <Head>
        <title>{name} - Algot</title>
      </Head>
      <WorkspaceSidebar />
      <div className="preview">
        <DemonstrationView />
        <ContextBar
          title="State View"
          reset={reset}
          undo={undo}
          // toggleDebug={debugToggleDebug}
        />
        <Playground graph={graph} />
      </div>
      <ToolsSidebar />
      {tutorialOpen && <Tutor />}
    </div>
  );
}

const Home: NextPage = () => {
  const router = useRouter();

  const writetoDB = async () => {
    const workspaceId = router.query.id as string;
    const state = produce(store.getState(), state => {
      state.playground.actions = [];
      Object.values(state.editor.operationsEditor).forEach(
        ed => (ed.undoSnapshots = [])
      );
    });
    const res = await fetch(`/api/workspaces/${workspaceId}`, {
      method: 'PUT',
      body: JSON.stringify(state),
      headers: {
        'Content-Type': 'application/json',
      },
    });
    await res.json();
  };

  useEffect(
    () =>
      router.query.id
        ? store.subscribe(debounce(writetoDB, 5_000, {maxWait: 30_000}))
        : undefined,
    [router.query.id]
  );

  return (
    <Provider store={store}>
      <App />
    </Provider>
  );
};

export default Home;
