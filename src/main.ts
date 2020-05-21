#!/usr/bin/env node

/**
 * Module dependencies.
 */
import { createServer, Server } from 'http';
import { ContainerCreateOptions } from 'dockerode';
import app from './app';
import { port as listenPort } from './config/config';
import initialise from './initialiser';

import getJunoModule from './module';
import { updateDeploymentConfig } from './models/database-modules/deployment';
import { DeployerConfigurations } from './models/interfaces/deployment';

const packageJson = require('./package.json');

/**
 * Normalize a port into a number, string, or false.
 */

function normalizePort(val: string) {
	const intVal = parseInt(val, 10);

	if (Number.isNaN(intVal)) {
		// named pipe
		return val;
	}

	if (intVal >= 0) {
		// port number
		return intVal;
	}

	return false;
}

/**
 * Get port from environment and store in Express.
 */

const port = normalizePort(listenPort);
app.set('port', port);

/**
 * Create HTTP server.
 */

let server: Server = null;
/**
 * Event listener for HTTP server "error" event.
 */

function onError(error: any) {
	if (error.syscall !== 'listen') {
		throw error;
	}

	const bind = typeof port === 'string'
		? `Pipe ${port}`
		: `Port ${port}`;

	// handle specific listen errors with friendly messages
	switch (error.code) {
		case 'EACCES':
			console.error(`${bind} requires elevated privileges`);
			process.exit(1);
			break;
		case 'EADDRINUSE':
			console.error(`${bind} is already in use`);
			process.exit(1);
			break;
		default:
			throw error;
	}
}

/**
 * Event listener for HTTP server "listening" event.
 */

function onListening() {
	const addr = server.address();
	const bind = typeof addr === 'string' ? `pipe ${addr}` : `port ${addr.port}`;
	console.log(`Application listening on ${bind}`);
}

async function main() {
	await initialise();
	const module = await getJunoModule();
	await module.initialize('bytesonus_api', packageJson.version);

	module.declareFunction('configUpdate', async (args: DeployerConfigurations) => {
		await Promise.all(args.configurations.map(
			(container: {id: string, configuration: ContainerCreateOptions}) => updateDeploymentConfig(
				Buffer.from(container.id, 'hex'),
				container.configuration,
			),
		));
	});

	module.declareFunction('createRegistryBot', (args: {organization: string}) => ({
		username: 'username',
		password: 'password',
	}));
	server = createServer(app);

	/**
	 * Listen on provided port, on all network interfaces.
	 */

	server.listen(port);
	server.on('error', onError);
	server.on('listening', onListening);
}

main();
