export const apiEnvironmentKeys = {
	all: () => ["apiEnvironment"] as const,
};

export const deploymentKeys = {
	all: (workspaceId: string) => ["deployments", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...deploymentKeys.all(workspaceId), "list", page, count] as const,
	detail: (workspaceId: string, id: string) => [...deploymentKeys.all(workspaceId), "detail", id] as const,
	metrics: (workspaceId: string, deploymentId: string, interval: string) =>
		[...deploymentKeys.all(workspaceId), "metrics", deploymentId, interval] as const,
};

export const runnerKeys = {
	all: (workspaceId: string) => ["runners", workspaceId] as const,
	list: (workspaceId: string) => [...runnerKeys.all(workspaceId), "list"] as const,
	pagedList: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...runnerKeys.all(workspaceId), "list", page, count] as const,
	detail: (workspaceId: string, id: string) => [...runnerKeys.all(workspaceId), "detail", id] as const,
	deployments: (workspaceId: string, runnerId: string, page: number, count: number) =>
		[...runnerKeys.all(workspaceId), "deployments", runnerId, page, count] as const,
	metrics: (workspaceId: string, runnerId: string, interval: string) =>
		[...runnerKeys.all(workspaceId), "metrics", runnerId, interval] as const,
};

export const workspacesKeys = {
	all: () => ["workspaces"] as const,
	list: () => [...workspacesKeys.all(), "list"] as const,
};

export const workspaceKeys = {
	all: (workspaceId: string) => ["workspace", workspaceId] as const,
	info: (workspaceId: string) => [...workspaceKeys.all(workspaceId), "info"] as const,
};

export const roleKeys = {
	all: (workspaceId: string) => ["roles", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...roleKeys.all(workspaceId), "list", page, count] as const,
	allRoles: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...roleKeys.all(workspaceId), "allRoles", page, count] as const,
	detail: (workspaceId: string, roleId: string) => [...roleKeys.all(workspaceId), "detail", roleId] as const,
	users: (workspaceId: string, roleId: string) => [...roleKeys.all(workspaceId), "users", roleId] as const,
};

export const memberKeys = {
	all: (workspaceId: string) => ["members", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...memberKeys.all(workspaceId), "list", page, count] as const,
};

export const inviteKeys = {
	all: (workspaceId: string) => ["invites", workspaceId] as const,
	list: (workspaceId: string) => [...inviteKeys.all(workspaceId), "list"] as const,
};

export const apiTokenKeys = {
	all: () => ["apiTokens"] as const,
	list: (page: string | undefined, count: string | undefined) =>
		[...apiTokenKeys.all(), "list", page, count] as const,
	detail: (id: string) => [...apiTokenKeys.all(), "detail", id] as const,
};

export const containerRegistryKeys = {
	all: (workspaceId: string) => ["containerRegistry", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...containerRegistryKeys.all(workspaceId), "list", page, count] as const,
	detail: (workspaceId: string, id: string) => [...containerRegistryKeys.all(workspaceId), "detail", id] as const,
	manifests: (workspaceId: string, repoId: string) =>
		[...containerRegistryKeys.all(workspaceId), "manifests", repoId] as const,
	manifestDetail: (workspaceId: string, repoId: string, digestOrTag: string) =>
		[...containerRegistryKeys.all(workspaceId), "manifestDetail", repoId, digestOrTag] as const,
	tags: (workspaceId: string, repoId: string, search?: string) =>
		[...containerRegistryKeys.all(workspaceId), "tags", repoId, search] as const,
	ports: (workspaceId: string, repoId: string, digestOrTag: string) =>
		[...containerRegistryKeys.all(workspaceId), "ports", repoId, digestOrTag] as const,
	usage: (workspaceId: string) => [...containerRegistryKeys.all(workspaceId), "usage"] as const,
};

export const domainKeys = {
	all: (workspaceId: string) => ["domains", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...domainKeys.all(workspaceId), "list", page, count] as const,
	detail: (workspaceId: string, id: string) => [...domainKeys.all(workspaceId), "detail", id] as const,
	verificationRecords: (workspaceId: string, domainId: string) =>
		[...domainKeys.all(workspaceId), "verificationRecords", domainId] as const,
};

export const managedUrlKeys = {
	all: (workspaceId: string) => ["managedUrls", workspaceId] as const,
	list: (workspaceId: string, domainId: string) => [...managedUrlKeys.all(workspaceId), "list", domainId] as const,
};

export const permissionKeys = {
	all: (workspaceId: string) => ["permissions", workspaceId] as const,
	list: (workspaceId: string) => [...permissionKeys.all(workspaceId), "list"] as const,
};

export const userPermissionKeys = {
	all: (workspaceId: string) => ["userPermissions", workspaceId] as const,
	current: (workspaceId: string) => [...userPermissionKeys.all(workspaceId), "current"] as const,
};

export const userInfoKeys = {
	all: () => ["userInfo"] as const,
	current: () => [...userInfoKeys.all(), "current"] as const,
};

export const mfaKeys = {
	all: () => ["mfa"] as const,
	secret: () => [...mfaKeys.all(), "secret"] as const,
};

export const logKeys = {
	all: (workspaceId: string) => ["logs", workspaceId] as const,
	initial: (workspaceId: string, restUrl: string, search: string) =>
		[...logKeys.all(workspaceId), restUrl, search] as const,
};

export const resourceKeys = {
	all: (workspaceId: string) => ["resources", workspaceId] as const,
	list: (workspaceId: string, resourceType: string) => [...resourceKeys.all(workspaceId), resourceType] as const,
	// Keyed on the sorted ID list so an unchanged set of resources reuses the cache.
	info: (workspaceId: string, resourceIds: readonly string[]) =>
		[...resourceKeys.all(workspaceId), "info", resourceIds] as const,
};
