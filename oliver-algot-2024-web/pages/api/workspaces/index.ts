/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {NextApiRequest, NextApiResponse} from 'next';
import database from 'src/database';
import {initialEditorState} from 'features/editor/editorReducer';
import {initialPlaygroundState} from 'features/playground/playgroundReducer';

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  const db = await database;

  if (req.method === 'POST') {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const workspace: any = {
      editor: initialEditorState,
      playground: initialPlaygroundState,
    };
    delete workspace._id;
    await db.collection('workspaces').insertOne(workspace);
    res.json(workspace);
  } else if (req.method === 'GET') {
    const workspaces = await db
      .collection('workspaces')
      .find()
      .sort({saveTimestamp: -1})
      .project({editor: {name: 1}})
      .toArray();
    res.json(workspaces);
  } else {
    res.status(405).end();
  }
}
