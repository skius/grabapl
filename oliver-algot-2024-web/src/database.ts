/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {MongoClient} from 'mongodb';

const clientPromise = new MongoClient('mongodb://127.0.0.1:27017').connect();
const database = clientPromise.then(c => c.db('algot'));

export default database;
