export interface Deployment {
	deploymentId: string;
	repository: string;
	tag: string;
	configuration: object; // Replace with configuration interface
}
