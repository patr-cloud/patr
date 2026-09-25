import {
	test,
	expect,
	newContext,
	loginAs,
	createUserWithWorkspace,
	createSecondMemberWithRole,
	getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import { createSecretAPI, randomSecretName } from '@/helpers/secret-api';
import {
	openSecretList,
	addSecretLink,
	secretRow,
	emptyStateHeading,
	openSecretDetail,
	deleteTrigger,
} from '@/helpers/ui/secret';

// Secret RBAC at the API layer (view/create/edit/delete gating, and a
// deployment only referencing secrets its author can view) lives in the Rust
// API suite (api/tests/api/workspace/rbac/permissions/{secret,deployment}.rs).
// Here we cover dashboard control-visibility.

type Owner = UserHandle & { workspaceId: string };

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
	return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

test.describe('secret > RBAC [UI]', () => {
	test('a view-only member sees secrets but no Add Secret CTA', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomSecretName();
		await createSecretAPI(api, owner, owner.workspaceId, name, 'viewable');
		const viewId = await permId(api, owner, 'secret::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openSecretList(page);
			await expect(secretRow(page, name)).toBeVisible({ timeout: 15_000 });
			await expect(addSecretLink(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member with the create permission sees the Add Secret CTA', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const createId = await permId(api, owner, 'secret::create');
		await using member = await createSecondMemberWithRole(api, owner, [createId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openSecretList(page);
			await expect(addSecretLink(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a view-only member sees no Delete on a secret detail', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			owner,
			owner.workspaceId,
			randomSecretName(),
			'undeletable',
		);
		const viewId = await permId(api, owner, 'secret::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openSecretDetail(page, secret.id);
			await expect(page.locator('#secret-name')).toBeVisible({ timeout: 15_000 });
			await expect(deleteTrigger(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member with the delete permission sees Delete on a secret detail', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			owner,
			owner.workspaceId,
			randomSecretName(),
			'deletable',
		);
		const viewId = await permId(api, owner, 'secret::view');
		const deleteId = await permId(api, owner, 'secret::delete');
		await using member = await createSecondMemberWithRole(api, owner, [viewId, deleteId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openSecretDetail(page, secret.id);
			await expect(deleteTrigger(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	// Listing is gated on membership alone (a runner lists secrets to catch
	// missed rotations), so a member without any secret permission still sees
	// the names — just no way to add one.
	test('a member with no secret permission still sees the list', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const name = randomSecretName();
		await createSecretAPI(api, owner, owner.workspaceId, name, 'listed');
		const viewRoles = await permId(api, owner, 'viewRoles');
		await using member = await createSecondMemberWithRole(api, owner, [viewRoles]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openSecretList(page);
			await expect(secretRow(page, name)).toBeVisible({ timeout: 15_000 });
			await expect(emptyStateHeading(page)).toBeHidden();
			await expect(addSecretLink(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});
});
