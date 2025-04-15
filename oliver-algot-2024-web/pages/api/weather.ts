/* Copyright 2022-2023 Theo Weidmann. All rights reserved. */
import {NextApiRequest, NextApiResponse} from 'next';

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  if (req.method === 'GET') {
    const coordinates = await fetch(
      `https://nominatim.openstreetmap.org/search?format=json&limit=1&q=${encodeURI(
        req.query.city as string
      )}`
    ).then(r => r.json());
    const {lat, lon} = coordinates[0];
    console.log(coordinates);
    const data = await fetch(
      `https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current_weather=true`
    ).then(r => r.json());
    res.json(
      req.query.data === 'condition'
        ? data.current_weather.weathercode > 10
          ? Math.floor(data.current_weather.weathercode / 10)
          : data.current_weather.weathercode > 3
          ? 1
          : 0
        : JSON.stringify(data.current_weather.temperature + '°')
    );
  } else {
    res.status(405).end();
  }
}

/*
0 - sunny
1 - cloudy
4 fog
5 light rain
6 rain
 */
