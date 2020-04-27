export interface Deployment {
	repository: string;
	tag: string;
	serverId: string;
	configuration: object; // Replace with configuration interface
}
