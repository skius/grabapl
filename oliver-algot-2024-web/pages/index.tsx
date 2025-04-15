/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import type {NextPage} from 'next';
import Button from 'components/Button';
import {useState} from 'react';
import {useRouter} from 'next/router';
import styles from 'styles/index.module.scss';

const Home: NextPage = () => {
  const router = useRouter();
  const [loading, setLoading] = useState(false);

  const createNew = async () => {
    setLoading(true);
    const res = await fetch('/api/workspaces/', {
      method: 'POST',
    });
    const json = await res.json();
    await router.push(`/${json._id}`);
    setLoading(false);
  };

  return (
    <div className={styles.wrapper}>
      <div className={styles.container}>
        <img src="/logo.svg" width="260" className={styles.logo} />
        {loading ? (
          <div>Creating workspace...</div>
        ) : (
          <Button onClick={createNew} large={true}>
            Open Workspace
          </Button>
        )}
      </div>
    </div>
  );
};

export default Home;
