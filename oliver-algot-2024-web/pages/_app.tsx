/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import 'styles/globals.css';
import type {AppProps} from 'next/app';

function MyApp({Component, pageProps}: AppProps) {
  return <Component {...pageProps} />;
}

export default MyApp;
