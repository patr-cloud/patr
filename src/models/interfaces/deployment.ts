import { ContainerCreateOptions } from 'dockerode';

export interface Deployment {
	deploymentId: Buffer;
	repository: string;
	tag: string;
	configuration: ContainerCreateOptions;
	serverId: Buffer;
	organizationId: Buffer;
}

export interface DeployerConfigurations {
	configurations: ContainerConfiguration[];
}

interface ContainerConfiguration {
	id: string;
	configuration: ContainerCreateOptions;
}

export interface DeployJob {
	id: string;
	image: string;
	auth?: RegAuth;
	server: DeployerServer;
	configuration?: ContainerCreateOptions;
	hostConfig?: ContainerCreateOptions['HostConfig'];
}


export interface RegAuth {
	username: string,
	password: string,
	serveraddress: string
}

export interface DeployerServer {
	protocol?: 'http' | 'https',
	host: string,
	port?: number,
	tlscacert?: string,
	tlscert?: string,
	tlskey?: string
}
