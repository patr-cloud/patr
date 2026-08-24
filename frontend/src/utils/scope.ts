/**
 * The editor's view of where a grant applies.
 *
 * The wire carries one grant per (subject, resource) row, with the workspace's
 * own id meaning the whole workspace. The pickers present that as a single
 * choice per subject — "entire workspace" or an explicit resource list — so
 * this type is the grouped form, converted at the API boundary.
 */
export type Scope = { scopeType: "workspace" } | { scopeType: "resources"; resources: string[] };

/** Groups flat `(subjectId, resourceId)` grants into one scope per subject. */
export const groupScopes = <T>(
	grants: T[],
	subjectOf: (grant: T) => string,
	resourceOf: (grant: T) => string,
	workspaceId: string
): { subjectId: string; scope: Scope }[] => {
	const bySubject = new Map<string, string[]>();
	for (const grant of grants) {
		const subjectId = subjectOf(grant);
		bySubject.set(subjectId, [...(bySubject.get(subjectId) ?? []), resourceOf(grant)]);
	}
	return [...bySubject].map(([subjectId, resources]) => ({
		subjectId,
		scope: resources.includes(workspaceId)
			? ({ scopeType: "workspace" } as const)
			: ({ scopeType: "resources", resources } as const),
	}));
};

/** Expands one grouped scope back into the resource ids it covers. */
export const scopeResources = (scope: Scope, workspaceId: string): string[] =>
	scope.scopeType === "workspace" ? [workspaceId] : scope.resources;
