import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { addDomainAPI, randomDomain } from '@/helpers/domain-api';
import { openDomainList, emptyStateHeading, addDomainLink, domainRow } from '@/helpers/ui/domain';

// List ordering/pagination and is-domain-valid (available/existing/subdomain/
// non-ICANN) at the API layer live in the Rust API suite
// (api/tests/api/workspace/domain.rs). Here we cover only the dashboard surface.

async function withList(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openDomainList(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('domain > list [UI]', () => {
	test('empty state shows heading and an add CTA', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withList(browser, user, async (page) => {
			await expect(emptyStateHeading(page)).toBeVisible();
			await expect(addDomainLink(page).first()).toBeVisible();
		});
	});

	test('lists an added domain by its full name', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const domain = randomDomain();
		await addDomainAPI(api, user, user.workspaceId, domain);
		await withList(browser, user, async (page) => {
			await expect(domainRow(page, domain)).toBeVisible();
			await expect(emptyStateHeading(page)).toBeHidden();
		});
	});
});
