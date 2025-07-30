#!/bin/bash

set -e

(cd .. && ./build.sh --release)
rm dist/* && npm run build
(cd ~/eth/msc-thesis/playground/grabapl-github-io && git checkout gh-pages && git pull)
rm ~/eth/msc-thesis/playground/grabapl-github-io/public/playground/*
cp dist/* ~/eth/msc-thesis/playground/grabapl-github-io/public/playground/
(cd ~/eth/msc-thesis/playground/grabapl-github-io/public/playground && git add . && git commit -m "update playground" && git push)
