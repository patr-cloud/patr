export interface Resource {
	resourceId: string;
	name: string;
	type: 'group' | 'deployer' | 'registry';
}
