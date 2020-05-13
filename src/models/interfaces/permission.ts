enum DockerRegistry {
	push = 'DockerRegisty.push',
	pull ='DockerRegistry.pull',
}

enum Deployer {
	create = 'Deployer.create',
	delete ='Deployer.delete',
}

enum Group {
	addUser = 'Group.addUser',
	deleteUser = 'Group.deleteUser',
}

enum Resource {
	grantPriveleges = 'Resource.grantPriveleges',
}

export type Permission = DockerRegistry | Deployer | Group | Resource;
export const permissions = {
	DockerRegistry,
	Deployer,
	Group,
	Resource,
};
