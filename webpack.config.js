const path = require("path");
const CopyPlugin = require('copy-webpack-plugin');

const srcFolder = path.join(__dirname, "src");
const buildFolder = path.join(__dirname, "bin");

const scriptsFolder = "static/scripts";

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
		entries[rootName] = path.join(srcFolder, scriptsFolder, file);
	}
	return entries;
}

module.exports = (env, argv) => ({
	watch: argv.mode === dev,
	entry: getEntries(),
	plugins: argv.mode === dev ? [] : [
		new CopyPlugin([
			{
				from: 'src/static',
				to: 'static',
				ignore: ['*.ts']
			},
			{
				from: 'src/views',
				to: 'views'
			},
			{
				from: 'src/config',
				to: 'config',
				ignore: ["*.ts"]
			}
		]),
	],
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
		filename: path.join(scriptsFolder, "[name].bundle.js"),
		path: argv.mode === dev ? srcFolder: buildFolder,
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
