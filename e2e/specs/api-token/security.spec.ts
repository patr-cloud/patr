import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	createApiTokenAPI,
	loginAs,
} from '@/prelude';
import { openTokenList } from '@/helpers/ui/api-token';

// API-layer token security (cross-user delete/regenerate/PATCH IDOR → 404,
// cross-workspace isolation, malformed/unknown token, MFA-route absence) lives
// in the Rust API suite (api/tests/api/user/api_token.rs). Here we cover the one
// UI surface: a suspicious token name renders as plain text (no script exec).

test.describe('api token > security [UI]', () => {
	test('renders a suspicious token name as plain text (no script execution)', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		// RESOURCE_NAME_REGEX disallows <>;'`/ etc. So a true XSS payload won't
		// pass server validation. Use a still-suspicious name that DOES pass:
		// letters/digits/_-. spaces. We assert the name renders as plain text and
		// no dialog fires.
		const sneaky = `script_alert_1_${Date.now().toString(36)}`;
		const t = await createApiTokenAPI(api, user, {
			name: sneaky,
			superAdminOf: [user.workspaceId],
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		let dialogFired = false;
		page.on('dialog', () => {
			dialogFired = true;
		});
		try {
			await openTokenList(page);
			await expect(page.getByText(t.name)).toBeVisible({ timeout: 10_000 });
			expect(dialogFired).toBe(false);
		} finally {
			await context.close();
		}
	});
});
