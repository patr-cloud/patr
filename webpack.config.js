const glob = require("glob");
const path = require("path");
const webpack = require('webpack');

const UglifyJsPlugin = require("uglifyjs-webpack-plugin");

const tsSrcFolder = path.join(__dirname, "src/static/scripts/");
const tsBuildFolder = path.join(__dirname, "bin/static/scripts/");
const tsGlob = tsSrcFolder + "*.ts";
const dev = "development";

const entryPoints = ['login.ts', 'register.ts'];

function isExternal(module) {
    var context = module.context;

    if (typeof context !== 'string') {
        return false;
    }

    return context.indexOf('node_modules') !== -1;
}

function getEntries() {
    const entries = {};
    for (const file of entryPoints) {
        const rootName = file.substring(0, file.lastIndexOf("."));
        entries[rootName] = path.join(tsSrcFolder, file);
    }
    return entries;
}

module.exports = (env, argv) => ({
    watch: argv.mode === dev,
    entry: getEntries(),
    module: {
        rules: [{
            test: /\.tsx?$/,
            use: {
                loader: 'ts-loader',
                options: {
                    configFile: "tsconfig.client.json"
                }
            },
            exclude: /node_modules/,
        }]
    },
    resolve: {
        extensions: [".ts", ".js"]
    },
    output: {
        filename: "[name].bundle.js",
        path: path.resolve(__dirname, argv.mode === dev ? tsSrcFolder : tsBuildFolder)
    },
    optimization: {
        runtimeChunk: 'single',
        splitChunks: {
            cacheGroups: {
                vendor: {
                    test: /[\\/]node_modules[\\/]/,
                    name: 'vendors',
                    enforce: true,
                    chunks: 'all'
                }
            }
        }
    },
});
