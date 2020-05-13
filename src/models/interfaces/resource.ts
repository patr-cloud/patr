export interface Resource {
	resourceId: Buffer;
	name: string;
	type: 'group' | 'deployer' | 'registry';
}
