import {
	test,
	expect,
	newContext,
	loginAs,
	expectUrl,
	createUserWithWorkspace,
	createUserAccount,
	addMemberToWorkspace,
	createRoleAPI,
	getPermissionId,
} from '@/prelude';
import type { UserHandle } from '@/prelude';
import {
	builtInRoleId,
	createServiceAccountAPI,
	deleteServiceAccountAPI,
	getServiceAccountAPI,
	randomServiceAccountName,
	regenerateServiceAccountTokenAPI,
	updateServiceAccountAPI,
} from '@/helpers/service-account-api';
import {
	openServiceAccountList,
	openServiceAccountDetail,
	openServiceAccountCreate,
	serviceAccountRow,
	createServiceAccountLink,
	emptyStateHeading,
	nameInput,
	saveButton,
	regenerateTokenButton,
	deleteButton,
	accessRow,
	selectBindingRole,
	submitCreateServiceAccount,
	goToServiceAccountButton,
	tokenShownOnceAlert,
	expectToast,
} from '@/helpers/ui/service-account';

// Who sees which controls, using the built-in "Service Account: Viewer",
// "Editor" and "Admin" roles every workspace is seeded with. The API's own
// RBAC rules live in the Rust API suite.

type Page = import('@playwright/test').Page;
type Owner = Awaited<ReturnType<typeof createUserWithWorkspace>>;

// A new member of the owner's workspace holding the named built-in roles.
async function memberWithRoles(
	api: import('@/prelude').ApiClient,
	owner: Owner,
	roleNames: string[],
): Promise<UserHandle> {
	const member = await createUserAccount(api);
	const roleIds = await Promise.all(
		roleNames.map((name) => builtInRoleId(api, owner, owner.workspaceId, name)),
	);
	await addMemberToWorkspace(api, owner, owner.workspaceId, member, roleIds);
	return member;
}

async function withPage(
	browser: import('@playwright/test').Browser,
	user: UserHandle,
	workspaceId: string,
	fn: (page: Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId });
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

// Order-insensitive view of a set of grants.
function sortedGrants(grants: { roleId: string; resourceId: string }[]): string[] {
	return grants.map((g) => `${g.roleId}:${g.resourceId}`).sort();
}

test.describe('service account > RBAC [UI]', () => {
	test('a viewer sees service accounts but no way to change them', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		await using member = await memberWithRoles(api, owner, ['Service Account: Viewer']);

		await withPage(browser, member, owner.workspaceId, async (page) => {
			await openServiceAccountList(page);
			await expect(serviceAccountRow(page, account.name)).toBeVisible({ timeout: 15_000 });
			await expect(createServiceAccountLink(page)).toHaveCount(0);

			await openServiceAccountDetail(page, account.id);
			await expect(nameInput(page)).toHaveValue(account.name, { timeout: 15_000 });
			await expect(nameInput(page)).toBeDisabled();
			// Without View Roles, the roles show as their IDs, read-only.
			await expect(accessRow(page, runnerViewer)).toContainText('Entire workspace');
			await expect(saveButton(page)).toHaveCount(0);
			await expect(regenerateTokenButton(page)).toHaveCount(0);
			await expect(deleteButton(page)).toHaveCount(0);
		});
	});

	test("the API refuses a viewer's create, edit, regenerate and delete", async ({ api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		await using member = await memberWithRoles(api, owner, ['Service Account: Viewer']);

		await expect(
			createServiceAccountAPI(api, member, owner.workspaceId, { roles: [runnerViewer] }),
		).rejects.toThrow(/401/);
		await expect(
			updateServiceAccountAPI(api, member, owner.workspaceId, account.id, {
				name: randomServiceAccountName(),
				roles: [runnerViewer],
			}),
		).rejects.toThrow(/401/);
		await expect(
			regenerateServiceAccountTokenAPI(api, member, owner.workspaceId, account.id),
		).rejects.toThrow(/401/);
		await expect(
			deleteServiceAccountAPI(api, member, owner.workspaceId, account.id),
		).rejects.toThrow(/401/);

		const after = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		expect(after.name).toBe(account.name);
	});

	test('an editor can edit and regenerate, but not delete', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		const before = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		await using member = await memberWithRoles(api, owner, ['Service Account: Editor']);
		const renamed = randomServiceAccountName('e2e-sa-renamed');

		await withPage(browser, member, owner.workspaceId, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await expect(nameInput(page)).toHaveValue(account.name, { timeout: 15_000 });
			await expect(nameInput(page)).toBeEnabled();
			await expect(regenerateTokenButton(page)).toBeVisible();
			await expect(deleteButton(page)).toHaveCount(0);

			// Without View Roles the roles can't be changed, but saving the rest
			// keeps them as they were.
			await expect(page.getByText(/You need permission to view roles/)).toBeVisible();
			await expect(accessRow(page, runnerViewer)).toBeVisible();
			await nameInput(page).fill(renamed);
			await saveButton(page).click();
			await expectToast(page, /Service account updated successfully/);
		});

		const after = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		expect(after.name).toBe(renamed);
		expect(sortedGrants(after.roleBindings)).toEqual(sortedGrants(before.roleBindings));
	});

	test('with View Roles too, an editor can change the roles', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		await using member = await memberWithRoles(api, owner, [
			'Service Account: Editor',
			'Workspace: Viewer',
		]);

		await withPage(browser, member, owner.workspaceId, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await expect(
				page.getByRole('button', { name: 'Remove Runner: Viewer', exact: true }),
			).toBeVisible({ timeout: 15_000 });
			await expect(page.getByText(/You need permission to view roles/)).toHaveCount(0);
		});
	});

	test("without View Roles, the create form says so and won't submit", async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		await using member = await memberWithRoles(api, owner, ['Service Account: Editor']);

		await withPage(browser, member, owner.workspaceId, async (page) => {
			await openServiceAccountCreate(page);
			await expect(page.getByText(/You need permission to view roles/)).toBeVisible({
				timeout: 15_000,
			});
			await expect(page.getByPlaceholder('Select a role')).toHaveCount(0);
			await expect(
				page.getByRole('button', { name: 'Create Service Account', exact: true }),
			).toBeDisabled();
		});
	});

	test('a creator who cannot view it is sent to the list', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const permissions = await Promise.all(
			['serviceAccount::create', 'viewRoles'].map((name) =>
				getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name),
			),
		);
		const role = await createRoleAPI(api, owner, owner.workspaceId, {
			name: `e2e-sa-creator-${crypto.randomUUID().slice(0, 8)}`,
			permissions,
		});
		await using member = await createUserAccount(api);
		await addMemberToWorkspace(api, owner, owner.workspaceId, member, [role.id]);

		await withPage(browser, member, owner.workspaceId, async (page) => {
			await openServiceAccountCreate(page);
			await nameInput(page).fill(randomServiceAccountName());
			await selectBindingRole(page, 'Runner: Viewer');
			await submitCreateServiceAccount(page);
			await expect(tokenShownOnceAlert(page)).toBeVisible({ timeout: 10_000 });
			await goToServiceAccountButton(page).click();
			await expectUrl(page, /\/workspace\/service-accounts$/, { timeout: 10_000 });
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('the Roles page links a service account only for those who can view it', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		const usersTab = `/workspace/roles/${runnerViewer}?tab=users`;

		await withPage(browser, owner, owner.workspaceId, async (page) => {
			await page.goto(usersTab, { waitUntil: 'domcontentloaded' });
			await expect(
				page.getByText(/Read-only here\. Edit a service account's roles/),
			).toBeVisible({ timeout: 15_000 });
			const link = page.getByRole('link', { name: account.name, exact: true });
			await expect(link).toBeVisible();
			await link.click();
			await expectUrl(page, new RegExp(`/workspace/service-accounts/${account.id}$`), {
				timeout: 10_000,
			});
		});

		await using member = await memberWithRoles(api, owner, ['Workspace: Viewer']);
		await withPage(browser, member, owner.workspaceId, async (page) => {
			await page.goto(usersTab, { waitUntil: 'domcontentloaded' });
			await expect(page.getByText(account.name, { exact: true })).toBeVisible({
				timeout: 15_000,
			});
			await expect(page.getByRole('link', { name: account.name, exact: true })).toHaveCount(
				0,
			);
		});
	});
});
