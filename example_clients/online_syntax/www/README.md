# Grabapl text-based IDE web demo

See [https://skius.github.io/grabapl/](https://skius.github.io/grabapl/) for the online demo.

## Building
Locally:
```bash
(cd .. && bash build.sh --release) && npm run start
```

For deployment:
```bash
rm dist/* && npm run build
rm ~/eth/msc-thesis/playground/grabapl-github-io/*
cp dist/* ~/eth/msc-thesis/playground/grabapl-github-io/
```

Then, in that repo, commit and push. (in gh-pages branch)