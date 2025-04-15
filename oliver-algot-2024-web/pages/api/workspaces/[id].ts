/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {NextApiRequest, NextApiResponse} from 'next';
import database from 'src/database';
import {ObjectId} from 'mongodb';

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  const db = await database;
  const _id = new ObjectId(req.query.id as string);

  if (req.method === 'PUT') {
    const newState = req.body;
    delete newState._id;
    newState.saveTimestamp = Date.now();
    await db.collection('workspaces').replaceOne({_id}, newState);
    res.json({status: 'ok'});
  } else if (req.method === 'GET') {
    res.json(await db.collection('workspaces').findOne({_id}));
  } else {
    res.status(405).end();
  }
}

export const config = {
  api: {
    bodyParser: {
      sizeLimit: '200mb',
    },
    responseLimit: false,
  },
};
