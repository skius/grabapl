#!/bin/bash

# Set the copyright notice
COPYRIGHT="\/\* Copyright 2022-2023 Theo Weidmann and others. All rights reserved. \*\/"

# Use find to locate the specified files and prepend the notice to them
find components/BlockLabel.ts \
     features/graphView/computeLayout.ts \
     features/graphView/GraphView.tsx \
     features/tools/ToolsSidebar.tsx \
     features/tutorial/Tutor.module.scss \
     features/tutorial/Tutor.tsx \
     features/tutorial/strings.json \
     features/workspace/AddButton.tsx \
     features/workspace/AbstractNode.module.scss \
     features/workspace/AbstractNode.tsx \
     features/workspace/OperationBox.module.scss \
     features/workspace/OperationBox.tsx \
     components/Sidebar.tsx \
     src/BaseOperations.ts \
     styles/globals.css \
     -type f -exec sed -i "" "1s/^/$COPYRIGHT\n/" {} \;
