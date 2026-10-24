import { test, expect, newContext, createUserWithWorkspace, loginAs, expectUrl } from '@/prelude';
import {
	builtInRoleId,
	createServiceAccountAPI,
	deleteServiceAccountAPI,
	getServiceAccountAPI,
	listServiceAccountsAPI,
} from '@/helpers/service-account-api';
import {
	openServiceAccountList,
	serviceAccountsTab,
	emptyStateHeading,
	emptyStateDescription,
	createServiceAccountLink,
	serviceAccountRow,
	listHeadings,
} from '@/helpers/ui/service-account';
import { openMembersPage, personRow } from '@/helpers/ui/member';

// The list lives in Workspace Settings, between Members and Roles. Listing
// rules at the API layer (view filtering, page bounds) live in the Rust API
// suite — api/tests/api/workspace/service_account.rs.

type Page = import('@playwright/test').Page;
type Owner = Awaited<ReturnType<typeof createUserWithWorkspace>>;

async function withPage(
	browser: import('@playwright/test').Browser,
	user: Owner,
	fn: (page: Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('service account > list [UI]', () => {
	test('with none yet, the empty state offers to create one', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openServiceAccountList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 10_000 });
			await expect(emptyStateDescription(page)).toBeVisible();
			await createServiceAccountLink(page).last().click();
			await expectUrl(page, /\/workspace\/service-accounts\/new$/, { timeout: 10_000 });
		});
	});

	test('shows each name and description, and no ID column', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const described = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
			description: 'Deploys from GitHub Actions',
		});
		const bare = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountList(page);
			await expect(serviceAccountRow(page, described.name)).toContainText(
				'Deploys from GitHub Actions',
				{ timeout: 10_000 },
			);
			await expect(serviceAccountRow(page, bare.name)).toContainText('No description');
			await expect(listHeadings(page)).toHaveText(['Name', 'Description']);
			await expect(page.getByText(described.id)).toHaveCount(0);
			await expect(page.getByText(bare.id)).toHaveCount(0);
			await expect(createServiceAccountLink(page)).toBeVisible();
		});
	});

	test('a row opens its service account', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountList(page);
			await serviceAccountRow(page, account.name).click();
			await expectUrl(page, new RegExp(`/workspace/service-accounts/${account.id}$`), {
				timeout: 10_000,
			});
		});
	});

	test('the tab sits between Members and Roles', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			await openMembersPage(page);
			const tabs = page.getByRole('link', {
				name: /^(General|Members|Service Accounts|Roles)$/,
			});
			await expect(tabs).toHaveText(['General', 'Members', 'Service Accounts', 'Roles']);
			await serviceAccountsTab(page).click();
			await expectUrl(page, /\/workspace\/service-accounts$/, { timeout: 10_000 });
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('list and get responses never carry a token', async ({ api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		expect(account.token).toMatch(/^patr_sa_/);

		const list = await listServiceAccountsAPI(owner, owner.workspaceId);
		expect(list.serviceAccounts.map((sa) => sa.id)).toContain(account.id);
		expect(list.raw).not.toContain(account.token);
		expect(list.raw).not.toMatch(/"token"/);

		const info = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		const raw = JSON.stringify(info);
		expect(raw).not.toContain(account.token);
		expect(Object.keys(info)).not.toContain('token');
	});

	test('paging recovers when the last one on a page is deleted', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		for (let i = 0; i < 21; i++) {
			await createServiceAccountAPI(api, owner, owner.workspaceId, { roles: [roleId] });
		}
		// The dashboard pages 20 at a time, so page 1 holds exactly one.
		const firstPage = await listServiceAccountsAPI(
			owner,
			owner.workspaceId,
			'?page=0&count=20',
		);
		const secondPage = await listServiceAccountsAPI(
			owner,
			owner.workspaceId,
			'?page=1&count=20',
		);
		expect(secondPage.serviceAccounts).toHaveLength(1);
		const last = secondPage.serviceAccounts[0];

		await withPage(browser, owner, async (page) => {
			await openServiceAccountList(page, '?page=1');
			await expect(serviceAccountRow(page, last.name)).toBeVisible({ timeout: 10_000 });

			// Older links quoted the page number, and still open the same page.
			const quoted = await page.context().newPage();
			await openServiceAccountList(quoted, '?page=%221%22');
			await expect(serviceAccountRow(quoted, last.name)).toBeVisible({ timeout: 10_000 });

			await deleteServiceAccountAPI(api, owner, owner.workspaceId, last.id);

			// A fresh tab, rather than a reload, which hangs under the dev server.
			const fresh = await page.context().newPage();
			await openServiceAccountList(fresh, '?page=1');
			await expect(serviceAccountRow(fresh, firstPage.serviceAccounts[0].name)).toBeVisible({
				timeout: 10_000,
			});
			await expect(emptyStateHeading(fresh)).toHaveCount(0);
			await expectUrl(fresh, /[?&]page=0(&|$)/, { timeout: 10_000 });
		});
	});

	test('the members list never shows service accounts', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		await withPage(browser, owner, async (page) => {
			await openMembersPage(page);
			await expect(personRow(page, owner.email)).toBeVisible({ timeout: 10_000 });
			await expect(page.getByText(account.name)).toHaveCount(0);
		});
	});
});
