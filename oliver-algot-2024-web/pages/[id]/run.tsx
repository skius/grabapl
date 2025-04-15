/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import type {NextPage} from 'next';
import {Provider} from 'react-redux';
import store from 'src/store';
import Playground from 'features/playground/Playground';
import {useAppDispatch, useAppSelector} from 'src/hooks';
import {useRouter} from 'next/router';
import {useEffect} from 'react';
import SiteLoader from 'components/SiteLoader';
import Head from 'next/head';
import {fetchWorkspace} from 'src/fetchWorkspace';

function App() {
  const router = useRouter();
  const name = useAppSelector(state => state.editor.name);
  const graph = useAppSelector(state => state.playground.graph);

  const dispatch = useAppDispatch();
  useEffect(() => {
    if (router.query.id) dispatch(fetchWorkspace(router.query.id as string));
  }, [router.query]);

  const loading = useAppSelector(state => state.editor.loading);

  return loading ? (
    <SiteLoader />
  ) : (
    <>
      <Head>
        <title>{name} - Algot</title>
      </Head>
      <div className="runContainer">
        <Playground graph={graph} />
      </div>
    </>
  );
}

const Home: NextPage = () => {
  return (
    <Provider store={store}>
      <App />
    </Provider>
  );
};

export default Home;
