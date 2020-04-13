#!/usr/bin/env node
var childProcess = require('child_process');
var fs = require('fs');

console.log('Building application...');
var env = process.env;
env.NODE_ENV = 'production';
childProcess.execSync('gulp', {
	env: env,
	stdio: [0, 1, 2]
});

console.log('Correcting package.json...');
var pkg = JSON.parse(fs.readFileSync('./src/package.json').toString());
if(pkg.scripts && pkg.scripts.start)
	pkg.scripts.start = `NODE_ENV=production ${pkg.scripts.start.replace('ts-node', 'node').replace('.ts', '.js')}`;
if(pkg.devDependencies)
	delete pkg.devDependencies;
fs.writeFileSync('./bin/package.json', JSON.stringify(pkg, null, '\t'));

console.log('Installing production dependencies...');
childProcess.execSync('npm install --only=production', {
	cwd: './bin',
	env: env,
	stdio: [0, 1, 2]
});
