import { ContainerCreateOptions } from 'dockerode';

export interface Deployment {
	deploymentId: string;
	repository: string;
	tag: string;
	configuration: ContainerCreateOptions; // Replace with configuration interface
	serverId: string;
}
