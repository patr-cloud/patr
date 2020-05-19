export interface Resource {
	resourceId: Buffer;
	name: string;
	type: 'organization' | 'deployer' | 'docker_registry';
}
