import { ContainerCreateOptions } from 'dockerode';

export interface Deployment {
	deploymentId: Buffer;
	repository: string;
	tag: string;
	configuration: ContainerCreateOptions; // Replace with configuration interface
	hostConfig?: ContainerCreateOptions['HostConfig'];
	serverId: string;
}


export interface DeployJob {
	id: string;
	image: string;
	auth?: RegAuth;
	server: Server;
	configuration?: ContainerCreateOptions;
	hostConfig?: ContainerCreateOptions['HostConfig'];
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
