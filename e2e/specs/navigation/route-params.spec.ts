import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';

// Detail pages paste their path params into API URLs, and TanStack decodes
// `%2F` in params. So each case smuggles `..%2F<another resource>%2F<target>`
// into a param: the page has to 404 before anything fetches the target.

const randomId = () => crypto.randomUUID().replaceAll('-', '');

const cases: { page: string; path: (target: string) => string }[] = [
	{ page: 'deployment page', path: (target) => `/deployments/..%2Frunner%2F${target}` },
	{ page: 'runner page', path: (target) => `/runners/..%2Fdeployment%2F${target}` },
	{ page: 'secret page', path: (target) => `/secrets/..%2Fdeployment%2F${target}` },
	{ page: 'domain page', path: (target) => `/domains/..%2Fdeployment%2F${target}` },
	{
		page: 'container repository page',
		path: (target) => `/container-registry/..%2Fdeployment%2F${target}`,
	},
	{
		page: 'manifest page, by its repository',
		path: (target) =>
			`/container-registry/..%2Fdeployment%2F${target}/manifest/sha256:${'0'.repeat(64)}`,
	},
	{
		page: 'manifest page, by its digest',
		path: (target) =>
			`/container-registry/${randomId()}/manifest/..%2F..%2F..%2Fdeployment%2F${target}`,
	},
	{ page: 'role page', path: (target) => `/workspace/roles/..%2F..%2Fdeployment%2F${target}` },
	{
		page: 'service account page',
		path: (target) => `/workspace/service-accounts/..%2Frunner%2F${target}`,
	},
	{
		page: 'API token page',
		path: (target) => `/profile/api-tokens/..%2F..%2Fworkspace%2F${target}`,
	},
];

test.describe('navigation > route params [UI]', () => {
	for (const { page: name, path } of cases) {
		test(`the ${name} 404s on a param that leaves its API path`, async ({ browser, api }) => {
			await using owner = await createUserWithWorkspace(api);
			const context = await newContext(browser, owner.clientIp);
			await loginAs(context, owner, { workspaceId: owner.workspaceId });
			const page = await context.newPage();
			try {
				const target = randomId();
				const calls: string[] = [];
				page.on('request', (req) => {
					if (req.url().includes('/api/') && req.url().includes(target)) {
						calls.push(req.url());
					}
				});
				await page.goto(path(target), { waitUntil: 'domcontentloaded' });
				await expect(page).toHaveTitle(/Not Found/, { timeout: 10_000 });
				expect(calls).toEqual([]);
			} finally {
				await context.close();
			}
		});
	}
});
