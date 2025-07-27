const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
    entry: "./bootstrap.ts",
    devtool: 'inline-source-map',
    experiments: {
        asyncWebAssembly: true
    },
    resolve: {
        extensions: ['.tsx', '.ts', '.js'],
    },
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "bootstrap.js",
        // NOTE: necessary for github pages, since node_modules_* files are not served.
        chunkFilename: "include_[name].[contenthash].js",
    },
    module: {
        rules: [
            {
                test: /\.css$/,
                use: ['style-loader', 'css-loader']
            },
            {
                test: /\.ttf$/,
                type: 'asset/resource'
            },
            {
                test: /\.tsx?$/,
                use: 'ts-loader',
                exclude: /node_modules/,
            },
        ]
    },
    mode: "development",
    plugins: [
        new CopyWebpackPlugin(['index.html']),
    ],
};
