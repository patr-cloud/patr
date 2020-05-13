import { Permission, permissions } from './permission';

export const roles: Role[] = [
	{
		roleId: 0,
		name: 'owner',
		permissions: [],
	},
	{
		roleId: 1,
		name: 'docker_registry.bot',
		permissions: [
			permissions.DockerRegistry.push,
			permissions.DockerRegistry.pull,
		],
	},
];

export interface Role {
	roleId: number,
	name: string,
	permissions: Permission[],
}
