export interface Deployment {
	deploymentId: string;
	repository: string;
	tag: string;
	configuration: string | object; // Replace with configuration interface
}
