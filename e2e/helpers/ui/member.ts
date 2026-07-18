import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Frontend: frontend/src/routes/_logged-in/_workspaced/workspace/members.tsx

export async function openMembersPage(page: Page): Promise<void> {
	await page.goto('/workspace/members', { waitUntil: 'domcontentloaded' });
	// UserSearchInput is the stable anchor on this page.
	await page
		.getByPlaceholder('Search for user by name or username...')
		.first()
		.waitFor({ state: 'visible', timeout: 15_000 });
}

// UserSearchInput is a custom <input type=text> with placeholder
// "Search for user by name or username...". Search triggers at >=3 chars.
export async function searchUser(page: Page, query: string): Promise<void> {
	await page.getByPlaceholder('Search for user by name or username...').fill(query);
}

export async function selectUserFromSearch(page: Page, username: string): Promise<void> {
	// The search dropdown wraps each result in a <button>. The members list
	// renders rows as <li role="button"> and also shows "@username", so a
	// plain getByText().first() races between the two when the user is already
	// a member. Scoping to <button> picks only the search-dropdown result.
	await page
		.locator('button')
		.filter({ hasText: `@${username}` })
		.first()
		.click();
}

export async function openRolesDropdown(page: Page): Promise<void> {
	// InputDropdownCheckBox renders the placeholder as the <input placeholder=...>
	// attribute (NOT text content). Click the input to open the dropdown.
	// The placeholder switches to "N role(s) selected" after selection.
	const placeholderInput = page
		.locator(
			'input[placeholder="Add roles..."], input[placeholder^="1 role selected"], input[placeholder*="roles selected"]',
		)
		.first();
	await placeholderInput.click();
}

export async function toggleRoleOption(page: Page, roleName: string): Promise<void> {
	// Dropdown options live in a max-h-60 overflow-y-scroll container; roles
	// below the fold need scrollIntoView first. Each option is an outer
	// <div class="cursor-pointer ..." onClick=...> wrapping a pointer-events-none
	// Checkbox. Playwright's actionability check trips on pointer-events: none,
	// so we click the outer cursor-pointer wrapper directly (where the onClick
	// handler lives). hasText match is anchored to the checkbox accessible name
	// via a nested locator instead of exact-text on the option (icon glyphs etc.
	// can also contribute to the option's text).
	const escaped = roleName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const option = page
		.locator('div.cursor-pointer')
		.filter({ has: page.getByRole('checkbox', { name: new RegExp(`^${escaped}$`) }) })
		.first();
	await option.scrollIntoViewIfNeeded({ timeout: 10_000 });
	await option.click();
}

export async function submitAddMember(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Add Member|Adding\.\.\.)$/ }).click();
}

export async function openMemberDetail(page: Page, fullName: string): Promise<void> {
	await page.locator('li[role="button"]').filter({ hasText: fullName }).click();
}

export async function clickEditRoles(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Edit roles$/ }).click();
}

export async function removeRoleChip(page: Page, roleName: string): Promise<void> {
	await page.getByRole('button', { name: `Remove ${roleName}` }).click();
}

export async function addRoleViaChipDropdown(page: Page, roleName: string): Promise<void> {
	// The "+ Add role..." text is an <input placeholder>, not rendered text.
	const input = page.locator('input[placeholder="+ Add role..."]').first();
	await input.click();
	// Dropdown options are pointer-events-none-wrapped Checkboxes; click the
	// outer cursor-pointer wrapper that holds the onClick.
	const escaped = roleName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const option = page
		.locator('div.cursor-pointer')
		.filter({ has: page.getByRole('checkbox', { name: new RegExp(`^${escaped}$`) }) })
		.first();
	await option.scrollIntoViewIfNeeded({ timeout: 10_000 });
	await option.click();
	// Close the dropdown so its option list doesn't overlay Save / other
	// controls. Escape on the focused input dismisses it.
	await input.press('Escape');
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
