import { ContainerCreateOptions } from 'dockerode';

export interface Deployment {
	deploymentId: string;
	repository: string;
	tag: string;
	configuration: ContainerCreateOptions; // Replace with configuration interface
	serverId: string;
}


export interface DeployJob {
	id: string;
	image: string;
	auth?: RegAuth;
	server: Server;
	configuration?: ContainerCreateOptions;
}


export interface RegAuth {
	username: string,
	password: string,
	serveraddress: string
}

export interface Server {
	protocol?: 'http' | 'https',
	host: string,
	port?: number,
	tlscacert?: string,
	tlscert?: string,
	tlskey?: string
}
