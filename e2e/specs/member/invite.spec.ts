import {
	test,
	expect,
	newContext,
	createUserAccount,
	createUserWithWorkspace,
	listRolesAPI,
	loginAs,
	getOwnUserId,
} from '@/prelude';
import { DEBUG_OTP } from '@/helpers/config';
import { sql } from '@/helpers/db';
import {
	openMembersPage,
	openRolesDropdown,
	toggleRoleOption,
	fillInviteEmail,
	submitInvite,
	inviteRow,
	copyInviteLink,
	resendInvite,
	revokeInvite,
	expectToast,
} from '@/helpers/ui/member';
import { openSignupPage, fillSignupForm, submitSignup } from '@/helpers/ui/signup';
import { fillOtp, submitConfirm } from '@/helpers/ui/confirm';
import { fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';

type InviteRow = { id: string; email: string; workspace_id: string };

type Invite = { id: string; acceptPath: string };

// Patr UUIDs are non-hyphenated everywhere (URLs, API payloads), but Postgres
// renders `uuid` columns hyphenated — so strip them here or the id won't match
// what the API expects.
async function inviteRowsFor(email: string): Promise<InviteRow[]> {
	return sql<InviteRow>(
		`SELECT REPLACE(id::text, '-', '') AS id, email, workspace_id
		 FROM workspace_user_invite WHERE email = $1`,
		[email],
	);
}

async function withUI(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openMembersPage(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

// Sends an invite through the UI and returns the created invite's id plus the
// path of its accept link. The token is stored hashed, so the link the API hands
// back on invite is the only place it can ever be read.
async function inviteViaUI(
	page: import('@playwright/test').Page,
	email: string,
	roleName: string,
): Promise<Invite> {
	await fillInviteEmail(page, email);
	await openRolesDropdown(page);
	await toggleRoleOption(page, roleName);

	const [response] = await Promise.all([
		page.waitForResponse(
			(r) =>
				r.request().method() === 'POST' &&
				new URL(r.url()).pathname.endsWith('/rbac/user/invite'),
		),
		submitInvite(page),
	]);
	await expectToast(page, /Invite sent/i);

	const { acceptUrl } = (await response.json()) as { acceptUrl: string };
	const { pathname, search } = new URL(acceptUrl);

	const rows = await inviteRowsFor(email);
	expect(rows).toHaveLength(1);
	return { id: rows[0]!.id, acceptPath: `${pathname}${search}` };
}

test.describe('member > invite [UI]', () => {
	test('invites an email and lists it as pending', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		await withUI(browser, owner, async (page) => {
			await inviteViaUI(page, invitee.email, viewerRole.name);
			await expect(page.getByText('Pending invitations')).toBeVisible();
			await expect(inviteRow(page, invitee.email)).toBeVisible();
		});
	});

	test('invitee accepts from the link and becomes a member', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		let invite: Invite;
		await withUI(browser, owner, async (page) => {
			invite = await inviteViaUI(page, invitee.email, viewerRole.name);
		});

		// The invitee opens the emailed link while logged in.
		const context = await newContext(browser, invitee.clientIp);
		await loginAs(context, invitee);
		const page = await context.newPage();
		try {
			await page.goto(invite!.acceptPath, { waitUntil: 'domcontentloaded' });

			// Confirmation screen names the workspace and does NOT auto-join.
			await expect(
				page.getByRole('heading', { name: /You've been invited to join/i }),
			).toBeVisible({ timeout: 15_000 });
			expect(await inviteRowsFor(invitee.email)).toHaveLength(1);

			await page.getByRole('button', { name: /^Join /i }).click();

			await expect
				.poll(async () => inviteRowsFor(invitee.email), { timeout: 15_000 })
				.toHaveLength(0);
			const inviteeId = await getOwnUserId(api, invitee);
			const membership = await sql(
				'SELECT user_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2',
				[owner.workspaceId, inviteeId],
			);
			expect(membership).toHaveLength(1);
		} finally {
			await context.close();
		}
	});

	test('re-inviting the same email is rejected', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		await withUI(browser, owner, async (page) => {
			await inviteViaUI(page, invitee.email, viewerRole.name);

			await fillInviteEmail(page, invitee.email);
			await openRolesDropdown(page);
			await toggleRoleOption(page, viewerRole.name);
			await submitInvite(page);
			await expectToast(page, /already been invited/i);

			expect(await inviteRowsFor(invitee.email)).toHaveLength(1);
		});
	});

	test('inviting an existing member is rejected', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		await withUI(browser, owner, async (page) => {
			// The owner is already in the workspace.
			await fillInviteEmail(page, owner.email);
			await openRolesDropdown(page);
			await toggleRoleOption(page, viewerRole.name);
			await submitInvite(page);
			await expectToast(page, /already belongs to a member/i);

			expect(await inviteRowsFor(owner.email)).toHaveLength(0);
		});
	});

	test('copies the invite link to the clipboard', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		const context = await newContext(browser, owner.clientIp);
		await context.grantPermissions(['clipboard-read', 'clipboard-write']);
		await loginAs(context, owner, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openMembersPage(page);
			const { id: inviteId } = await inviteViaUI(page, invitee.email, viewerRole.name);

			await copyInviteLink(page, invitee.email);
			const copied = await page.evaluate(() => navigator.clipboard.readText());
			expect(copied).toContain(`inviteId=${inviteId}`);
			expect(copied).toContain('/accept-invite');
		} finally {
			await context.close();
		}
	});

	test('resends an invite, keeping a single pending row', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		await withUI(browser, owner, async (page) => {
			const { id: inviteId } = await inviteViaUI(page, invitee.email, viewerRole.name);
			await resendInvite(page, invitee.email);
			// Deliberately not matching "Invite sent" too — that toast is still on
			// screen from the invite above, so an alternation would pass without
			// the resend ever succeeding.
			await expectToast(page, /Invite resent/i);

			const rows = await inviteRowsFor(invitee.email);
			expect(rows).toHaveLength(1);
			expect(rows[0]!.id).toBe(inviteId);
		});
	});

	test('revokes an invite and the link stops working', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		let invite: Invite;
		await withUI(browser, owner, async (page) => {
			invite = await inviteViaUI(page, invitee.email, viewerRole.name);
			await revokeInvite(page, invitee.email);
			await expectToast(page, /Invite revoked/i);
			await expect
				.poll(async () => inviteRowsFor(invitee.email), { timeout: 15_000 })
				.toHaveLength(0);
		});

		const context = await newContext(browser, invitee.clientIp);
		await loginAs(context, invitee);
		const page = await context.newPage();
		try {
			await page.goto(invite!.acceptPath, { waitUntil: 'domcontentloaded' });
			await expect(page.getByRole('heading', { name: /Invalid invite/i })).toBeVisible({
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	test('rejects an invite opened by the wrong account', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await using invitee = await createUserAccount(api);
		await using bystander = await createUserAccount(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		let invite: Invite;
		await withUI(browser, owner, async (page) => {
			invite = await inviteViaUI(page, invitee.email, viewerRole.name);
		});

		const context = await newContext(browser, bystander.clientIp);
		await loginAs(context, bystander);
		const page = await context.newPage();
		try {
			await page.goto(invite!.acceptPath, { waitUntil: 'domcontentloaded' });
			// Preview succeeds (the token is valid) — the ownership check bites on join.
			await page.getByRole('button', { name: /^Join /i }).click();
			await expect(page.getByRole('heading', { name: /Wrong account/i })).toBeVisible({
				timeout: 15_000,
			});

			// Still pending, and the bystander did not join.
			expect(await inviteRowsFor(invitee.email)).toHaveLength(1);
			const bystanderId = await getOwnUserId(api, bystander);
			const membership = await sql(
				'SELECT user_id FROM workspace_user WHERE workspace_id = $1 AND user_id = $2',
				[owner.workspaceId, bystanderId],
			);
			expect(membership).toHaveLength(0);
		} finally {
			await context.close();
		}
	});
});

// The signed-out path: the invite is stashed in sessionStorage so it survives
// the sign-up detour, then resumed once the new account exists.
test.describe('member > invite > sign-up handoff [UI] @racy', () => {
	test('a brand-new email signs up from the link and lands back on accept', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const roles = await listRolesAPI(api, owner, owner.workspaceId);
		const viewerRole = roles.find((r) => /Workspace: Viewer/i.test(r.name))!;

		// An address with no Patr account behind it.
		const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
		const username = `e2einvitee${suffix}`;
		const email = `${username}@example.com`;
		const password = 'E2eTest!1Password';

		let invite: Invite;
		await withUI(browser, owner, async (page) => {
			invite = await inviteViaUI(page, email, viewerRole.name);
		});

		const context = await newContext(browser);
		const page = await context.newPage();
		try {
			await page.goto(invite!.acceptPath, { waitUntil: 'domcontentloaded' });

			// Signed out: the page offers sign-up and stashes the invite.
			await expect(
				page.getByRole('heading', { name: /You've been invited to a workspace/i }),
			).toBeVisible({ timeout: 15_000 });
			const stashed = await page.evaluate(() =>
				sessionStorage.getItem('pendingWorkspaceInvite'),
			);
			expect(stashed).toContain(invite!.id);

			await page.getByRole('button', { name: /Create an account/i }).click();

			await openSignupPage(page);
			await fillSignupForm(page, {
				username,
				firstName: 'E2E',
				lastName: 'Invitee',
				email,
				password,
			});
			await submitSignup(page);

			await fillOtp(page, DEBUG_OTP);
			await submitConfirm(page);

			// Confirming a sign-up does not log you in — it drops you on /login.
			// Logging in is what resumes the invite: login.tsx reads the stash and
			// redirects here, so this also covers that handoff.
			await fillLoginForm(page, { userId: username, password });
			await submitLogin(page);
			await waitForLoggedIn(page);

			await expect(
				page.getByRole('heading', { name: /You've been invited to join/i }),
			).toBeVisible({ timeout: 20_000 });
			await page.getByRole('button', { name: /^Join /i }).click();

			await expect
				.poll(async () => inviteRowsFor(email), { timeout: 20_000 })
				.toHaveLength(0);
			const membership = await sql(
				`SELECT wu.user_id
				 FROM workspace_user wu
				 JOIN "user" u ON u.id = wu.user_id
				 WHERE wu.workspace_id = $1 AND u.username = $2`,
				[owner.workspaceId, username],
			);
			expect(membership).toHaveLength(1);
		} finally {
			await context.close();
		}
	});
});
