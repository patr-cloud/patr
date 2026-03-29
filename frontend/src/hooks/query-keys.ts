export const deploymentKeys = {
	all: (workspaceId: string) => ["deployments", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...deploymentKeys.all(workspaceId), "list", page, count] as const,
	detail: (workspaceId: string, id: string) => [...deploymentKeys.all(workspaceId), "detail", id] as const,
};

export const runnerKeys = {
	all: (workspaceId: string) => ["runners", workspaceId] as const,
	list: (workspaceId: string) => [...runnerKeys.all(workspaceId), "list"] as const,
	pagedList: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...runnerKeys.all(workspaceId), "list", page, count] as const,
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
};

export const memberKeys = {
	all: (workspaceId: string) => ["members", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...memberKeys.all(workspaceId), "list", page, count] as const,
};

export const apiTokenKeys = {
	all: () => ["apiTokens"] as const,
	list: (page: string | undefined, count: string | undefined) =>
		[...apiTokenKeys.all(), "list", page, count] as const,
};

export const containerRegistryKeys = {
	all: (workspaceId: string) => ["containerRegistry", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...containerRegistryKeys.all(workspaceId), "list", page, count] as const,
};

export const domainKeys = {
	all: (workspaceId: string) => ["domains", workspaceId] as const,
	list: (workspaceId: string, page: string | undefined, count: string | undefined) =>
		[...domainKeys.all(workspaceId), "list", page, count] as const,
	verificationRecords: (workspaceId: string, domainId: string) =>
		[...domainKeys.all(workspaceId), "verificationRecords", domainId] as const,
};

export const permissionKeys = {
	all: (workspaceId: string) => ["permissions", workspaceId] as const,
	list: (workspaceId: string) => [...permissionKeys.all(workspaceId), "list"] as const,
};

export const userPermissionKeys = {
	all: (workspaceId: string) => ["userPermissions", workspaceId] as const,
	current: (workspaceId: string) => [...userPermissionKeys.all(workspaceId), "current"] as const,
};
