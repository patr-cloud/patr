enum DockerRegistry {
	push = 'DockerRegisty.push',
	pull ='DockerRegistry.pull',
}

enum Deployer {
	create = 'Deployer.create',
	delete ='Deployer.delete',
	addDomain ='Deployer.addDomain',
	removeDomain = 'Deployer.removeDomain',
	verifyDomain = 'Deployer.verifyDomain',
}

enum Organization {
	create = 'Organization.create',
	addUser = 'Organization.addUser',
	deleteUser = 'Organization.deleteUser',
}

enum Resource {
	grantPriveleges = 'Resource.grantPriveleges',
}

export type Permission = DockerRegistry | Deployer | Organization | Resource;
export const permissions = {
	DockerRegistry,
	Deployer,
	Organization,
	Resource,
};
