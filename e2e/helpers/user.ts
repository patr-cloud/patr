import type { ApiClient } from '@/helpers/api';
import { randomIPv4 } from '@/helpers/ip';
import { DEBUG_OTP, TURNSTILE_TOKEN } from '@/helpers/config';

export type User = {
	password: string;
	email: string;
	firstName: string;
	lastName: string;
	accessToken: string;
	refreshToken: string;
	clientIp: string;
};

export type UserHandle = User & AsyncDisposable;

export type PendingSignup = {
	password: string;
	email: string;
	firstName: string;
	lastName: string;
	clientIp: string;
};

// Calls /auth/sign-up only — no /auth/join. Used by tests that drive the
// confirm-signup flow through the UI and need a pending user_to_sign_up row.
// Retries on 429 with a fresh IP — the per-IP rate-limit bucket can be stale
// from a prior test that hit it; a fresh IP gives us a fresh bucket.
export async function createPendingSignup(
	api: ApiClient,
	overrides: Partial<PendingSignup> = {},
): Promise<PendingSignup> {
	const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
	const password = overrides.password ?? 'E2eTest!1Password';
	const email = overrides.email ?? `e2euser${suffix}@example.com`;
	const firstName = overrides.firstName ?? 'E2E';
	// Unique per user: the member-search dropdown renders the name (never the
	// email), so a shared 'E2E User' would make every result row look alike.
	const lastName = overrides.lastName ?? `User${suffix}`;
	const explicitIp = overrides.clientIp;

	let lastErr: unknown;
	for (let attempt = 1; attempt <= 4; attempt++) {
		const clientIp = explicitIp ?? randomIPv4();
		try {
			await api.request('POST', '/auth/sign-up', {
				clientIp,
				body: {
					email,
					password,
					firstName,
					lastName,
					cfTurnstileToken: TURNSTILE_TOKEN,
				},
			});
			return { password, email, firstName, lastName, clientIp };
		} catch (err) {
			lastErr = err;
			// Only retry on 429. If caller pinned an IP, don't change it.
			if (explicitIp || !String(err).includes('429')) throw err;
			await new Promise((r) => setTimeout(r, 150 * attempt));
		}
	}
	throw lastErr ?? new Error('createPendingSignup: exhausted retries');
}

// Creates a verified user AND a workspace, so the frontend's _workspaced
// route group renders the dashboard rather than the inline create-workspace
// screen. Use this when the test needs a workspace-scoped page to load.
export async function createUserWithWorkspace(
	api: ApiClient,
): Promise<UserHandle & { workspaceId: string; workspaceName: string }> {
	const user = await createUserAccount(api);
	const workspaceName = `wks-${crypto.randomUUID().replace(/-/g, '').slice(0, 12)}`;
	const resp = await api.request<{ id: string }>('POST', '/workspace', {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: { name: workspaceName },
	});
	return Object.assign(user, { workspaceId: resp.id, workspaceName });
}

// Creates a verified user and N named workspaces. Names are passed in so the
// caller knows what to switch to in the UI without race-prone DB lookups.
// First workspace becomes the active one (matches the frontend's auto-select-
// first behavior on _workspaced load).
export async function createUserWithWorkspaces(
	api: ApiClient,
	names: string[],
): Promise<UserHandle & { workspaces: { id: string; name: string }[] }> {
	if (names.length === 0) {
		throw new Error('createUserWithWorkspaces: provide at least one name');
	}
	const user = await createUserAccount(api);
	const workspaces: { id: string; name: string }[] = [];
	for (const name of names) {
		const resp = await api.request<{ id: string }>('POST', '/workspace', {
			token: user.accessToken,
			clientIp: user.clientIp,
			body: { name },
		});
		workspaces.push({ id: resp.id, name });
	}
	return Object.assign(user, { workspaces });
}

// Resolves a user's UUID via GET /user (using their own token). Cheaper than
// touching the DB and avoids leaking id into every UserHandle.
export async function getOwnUserId(
	api: ApiClient,
	user: { accessToken: string; clientIp: string },
): Promise<string> {
	const me = await api.request<{ id: string }>('GET', '/user', {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
	return me.id;
}

// Adds an existing user to `workspaceId` with the given roleIds. roleIds must
// come from the owner's seeded defaults (e.g. one of the 36 default roles
// looked up via GET /rbac/role). Looks up the invitee's user_id on the fly.
export async function addMemberToWorkspace(
	api: ApiClient,
	owner: { accessToken: string; clientIp: string },
	workspaceId: string,
	invitee: { accessToken: string; clientIp: string },
	roleIds: string[],
): Promise<void> {
	const inviteeId = await getOwnUserId(api, invitee);
	await api.request('POST', `/workspace/${workspaceId}/rbac/user/${inviteeId}`, {
		token: owner.accessToken,
		clientIp: owner.clientIp,
		body: { roles: roleIds },
	});
}

// Creates a second user, creates a role in owner's workspace with the given
// permissions, adds the user as a member with that role. Returns the new
// user + role id. This is the cornerstone of all RBAC enforcement tests.
export async function createSecondMemberWithRole(
	api: ApiClient,
	owner: UserHandle & { workspaceId: string },
	permissions: Record<string, { permissionType: 'include' | 'exclude'; resources: string[] }>,
): Promise<UserHandle & { roleId: string }> {
	const invitee = await createUserAccount(api);
	const roleName = `e2e-role-${crypto.randomUUID().slice(0, 8)}`;
	const role = await api.request<{ id: string }>(
		'POST',
		`/workspace/${owner.workspaceId}/rbac/role`,
		{
			token: owner.accessToken,
			clientIp: owner.clientIp,
			body: {
				name: roleName,
				description: roleName,
				permissions,
			},
		},
	);
	await addMemberToWorkspace(api, owner, owner.workspaceId, invitee, [role.id]);
	return Object.assign(invitee, { roleId: role.id });
}

// Creates a second user with NO roles in the workspace (still a valid user,
// just not a workspace member).
export async function createSecondUserNoMembership(api: ApiClient): Promise<UserHandle> {
	return createUserAccount(api);
}

export async function createUserAccount(api: ApiClient): Promise<UserHandle> {
	const { password, email, firstName, lastName, clientIp } = await createPendingSignup(api);

	const tokens = await api.request<{ accessToken: string; refreshToken: string }>(
		'POST',
		'/auth/join',
		{
			clientIp,
			body: {
				email,
				verificationToken: DEBUG_OTP,
				cfTurnstileToken: TURNSTILE_TOKEN,
			},
		},
	);

	const handle: UserHandle = {
		password,
		email,
		firstName,
		lastName,
		clientIp,
		accessToken: tokens.accessToken,
		refreshToken: tokens.refreshToken,
		async [Symbol.asyncDispose]() {
			// Best-effort logout. The API doesn't expose user-deletion to the user
			// themselves, so the row stays in the DB until the next `just e2e-down`
			// wipes the volume. That's fine — emails are random per test.
			try {
				await api.request('POST', '/auth/sign-out', {
					token: handle.accessToken,
					clientIp,
				});
			} catch {
				// ignore — token may already be expired, user already gone, etc.
			}
		},
	};

	return handle;
}
