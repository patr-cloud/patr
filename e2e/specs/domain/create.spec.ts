import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';
import { randomDomain } from '@/helpers/domain-api';
import {
	openDomainCreate,
	fillDomainName,
	submitAddDomain,
	requiredError,
	urlLikeError,
	suggestionButton,
	domainRow,
} from '@/helpers/ui/domain';

// Domain creation is driven through the dashboard. The client blocks empty and
// URL-like input (protocol/path/query) with a "did you mean" suggestion and no
// network; everything else is POSTed and any server rejection (subdomain,
// non-ICANN, IDN, duplicate) collapses into one generic "Failed to add domain"
// alert. The precise status codes, the IDN→500 gap, global (cross-workspace)
// uniqueness, and re-add-after-delete live in the Rust API suite
// (api/tests/api/workspace/domain.rs).

async function withCreatePage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openDomainCreate(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

function trackAddPosts(page: import('@playwright/test').Page): () => number {
	let count = 0;
	page.on('request', (req) => {
		if (req.method() === 'POST' && /\/api\/workspace\/[^/]+\/domain$/.test(req.url()))
			count += 1;
	});
	return () => count;
}

test.describe('domain > create [UI]', () => {
	test('adds a domain: success toast, navigate to /domains, row visible', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const domain = randomDomain();
		await withCreatePage(browser, user, async (page) => {
			await fillDomainName(page, domain);
			await submitAddDomain(page);
			await expectToast(page, /Domain added successfully/i);
			await expectUrl(page, /\/domains$/, { timeout: 10_000 });
			await expect(domainRow(page, domain)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('normalizes uppercase input to lowercase', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const lower = randomDomain();
		await withCreatePage(browser, user, async (page) => {
			await fillDomainName(page, lower.toUpperCase());
			await submitAddDomain(page);
			await expectUrl(page, /\/domains$/, { timeout: 10_000 });
			// The stored/displayed domain is lowercased.
			await expect(domainRow(page, lower)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('a subdomain is rejected with a generic add error (no navigation)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			// A bare subdomain isn't URL-like, so it passes the client guard and 400s.
			await fillDomainName(page, `sub.${randomDomain()}`);
			await submitAddDomain(page);
			await expectToast(page, /Failed to add domain/i);
			await expectUrl(page, /\/domains\/new$/, { timeout: 5_000 });
		});
	});

	test('URL-like input is blocked client-side with a suggestion (no network)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackAddPosts(page);
			await fillDomainName(page, 'https://foo.example.com/path?q=1');
			await submitAddDomain(page);
			await expect(urlLikeError(page)).toBeVisible();
			await expect(suggestionButton(page, 'foo.example.com')).toBeVisible();
			await page.waitForTimeout(400);
			expect(posts()).toBe(0);
		});
	});

	test('empty domain: required error, no network call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withCreatePage(browser, user, async (page) => {
			const posts = trackAddPosts(page);
			await submitAddDomain(page);
			await expect(requiredError(page)).toBeVisible();
			await page.waitForTimeout(400);
			expect(posts()).toBe(0);
		});
	});
});
