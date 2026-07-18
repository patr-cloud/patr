// Common imports for specs. Re-exports the things every spec needs so a single
// `import { ... } from '@/prelude'` covers most cases. Reach for the specific
// `@/fixtures/...` or `@/helpers/...` paths only when you need something not
// re-exported here.

export { test, expect, newContext } from '@/fixtures/api';
export type { ApiClient } from '@/helpers/api';

export {
	createUserAccount,
	createUserWithWorkspace,
	createUserWithWorkspaces,
	createPendingSignup,
	addMemberToWorkspace,
	createSecondMemberWithRole,
	createSecondUserNoMembership,
	getOwnUserId,
} from '@/helpers/user';
export type { User, UserHandle, PendingSignup } from '@/helpers/user';

export {
	getPermissionId,
	listPermissions,
	createRoleAPI,
	updateRoleAPI,
	deleteRoleAPI,
	listRolesAPI,
	getRoleAPI,
	setUserRolesAPI,
	removeMemberAPI,
	currentPermissionsAPI,
} from '@/helpers/api/rbac';

export { loginAs } from '@/helpers/ui/session';
export { expectUrl, expectUrlNot } from '@/helpers/ui/workspace';

export { createApiTokenAPI, patchApiTokenAPI, callWithApiToken } from '@/helpers/api-token';
export type {
	ApiTokenHandle,
	CreateApiTokenOpts,
	WorkspacePermissionInput,
} from '@/helpers/api-token';

export { RunnerHandle } from '@/helpers/runner';
export type { RunnerOpts } from '@/helpers/runner';
export { DindHandle } from '@/helpers/dind';
export type { DockerVersion } from '@/helpers/dind';

export { randomIPv4 } from '@/helpers/ip';

// Auth-flow infrastructure.
export { computeTotp } from '@/helpers/totp';
export { readMfaSetupSecret } from '@/helpers/redis';
export {
	queryUser,
	backdateSignupOtp,
	backdatePasswordResetToken,
	exhaustPasswordResetAttempts,
	backdateWebLoginExpiry,
	deleteWebLogin,
	sql,
} from '@/helpers/db';

// Shared test-harness configuration — re-exported so specs can import from
// '@/prelude' without knowing the helper layout.
export {
	DEBUG_OTP,
	HYDRATION_TIMEOUT,
	USER_AGENT,
	JWT_SECRET,
	TURNSTILE_TOKEN,
} from '@/helpers/config';
export { DASHBOARD_URL, API_DIRECT_URL, VINXI_DEV_URL } from '@/helpers/urls';
