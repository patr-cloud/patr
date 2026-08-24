import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend:
//   frontend/src/routes/_logged-in/_workspaced/workspace/members.tsx
//   frontend/src/routes/_logged-in/_workspaced/workspace/members_/invite.tsx
//
// The members page is one list of people — the owner first, then pending
// invites, then members — beside a detail panel. Row-level actions (resend,
// revoke, edit, remove) all live in that panel, so every helper here selects
// the row first and then acts on the panel.

export async function openMembersPage(page: Page): Promise<void> {
	await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
	// Anchor on the workspace header's Members tab rather than any control:
	// the actions are gated behind modifyRoles, so anchoring on one would hang
	// for a viewer-only member.
	await page
		.getByRole('link', { name: 'Members', exact: true })
		.first()
		.waitFor({ state: 'visible', timeout: 15_000 });
}

// Inviting is its own page, reached from the header button.
export async function openInvitePage(page: Page): Promise<void> {
	await page.goto('/workspace/members/invite', { waitUntil: 'domcontentloaded' });
	await page
		.getByPlaceholder('someone@example.com')
		.waitFor({ state: 'visible', timeout: 15_000 });
}

export async function fillInviteEmail(page: Page, email: string): Promise<void> {
	await page.getByPlaceholder('someone@example.com').fill(email);
}

export async function submitInvite(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Send invite|Sending\.\.\.)$/ }).click();
}

/**
 * Picks an option from an `InputDropdown`. The list is portalled to the body
 * with `position: fixed`, so it is not a descendant of the input — scope the
 * option lookup to the page, not to the trigger.
 */
async function pickFromDropdown(page: Page, trigger: ReturnType<Page['locator']>, label: string) {
	await trigger.click();
	const option = page
		.locator('div.cursor-pointer')
		.filter({ hasText: new RegExp(`^${label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`) })
		.first();
	await option.scrollIntoViewIfNeeded({ timeout: 10_000 });
	await option.click();
}

/** The role dropdown of the last binding row — the one just added. */
export async function selectBindingRole(page: Page, roleName: string): Promise<void> {
	await pickFromDropdown(page, page.getByPlaceholder('Select a role').last(), roleName);
}

/** A person's row in the list, located by email — invites and members alike. */
export function personRow(page: Page, text: string) {
	return page.locator('li[role="button"]').filter({ hasText: text }).first();
}

export async function selectPerson(page: Page, text: string): Promise<void> {
	await personRow(page, text).click();
}

/** Kept as `inviteRow` for the invite specs; an invite is just a person row. */
export const inviteRow = personRow;

export async function copyInviteLink(page: Page, email: string): Promise<void> {
	await selectPerson(page, email);
	await page.getByRole('button', { name: /Copy link/i }).click();
}

export async function resendInvite(page: Page, email: string): Promise<void> {
	await selectPerson(page, email);
	await page.getByRole('button', { name: /^Resend$/ }).click();
}

// Revoke is a two-step confirm: the trash icon arms it, then "Revoke" commits.
export async function revokeInvite(page: Page, email: string): Promise<void> {
	await selectPerson(page, email);
	await page.getByRole('button', { name: 'Revoke invite' }).click();
	await page.getByRole('button', { name: /^Revoke$/ }).click();
}

export async function openMemberDetail(page: Page, fullName: string): Promise<void> {
	await selectPerson(page, fullName);
}

export async function clickEditRoles(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Edit access$/ }).click();
}

export async function removeRoleChip(page: Page, roleName: string): Promise<void> {
	await page.getByRole('button', { name: `Remove ${roleName}` }).click();
}

/**
 * Adds a role to the person being edited: "Add role" appends an empty binding
 * row, then its own dropdown picks the role. (The old chip dropdown was a
 * multi-select over every role; a binding row carries one role and its scope.)
 */
export async function addRoleViaChipDropdown(page: Page, roleName: string): Promise<void> {
	await page.getByRole('button', { name: /^Add role$/ }).click();
	await selectBindingRole(page, roleName);
}

export async function saveMemberRoles(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Save$/ }).click();
}

export async function cancelMemberRolesEdit(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Cancel$/ }).click();
}

export async function clickRemoveMember(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Remove member$/ }).click();
}

export async function confirmRemoveMember(page: Page): Promise<void> {
	// Inline confirm: the only button labeled "Remove" inside the confirm box.
	await page.getByRole('button', { name: /^Remove$/ }).click();
}

export async function expectToast(page: Page, matcher: RegExp, timeout = 10_000): Promise<void> {
	await expect(page.getByText(matcher).first()).toBeVisible({ timeout });
}
