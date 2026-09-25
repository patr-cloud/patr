import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/secrets/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/secrets/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/secrets/$id.tsx (detail)
//
// Create and detail share the same two fields: #secret-name and #secret-value.
// On the detail page a blank value keeps the stored one.

async function waitForVisible(page: Page, selector: string): Promise<void> {
	await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// ---------- List (/secrets) ----------

export async function openSecretList(page: Page): Promise<void> {
	await page.goto('/secrets', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
	return page.getByText('No Secrets Added', { exact: true });
}

// "Add Secret" is a link to /secrets/new (header at >=1, empty-state CTA at 0).
export function addSecretLink(page: Page) {
	return page.getByRole('link', { name: /Add Secret/i });
}

export function secretRow(page: Page, name: string) {
	// List renders a mobile card grid and a desktop table (both in the DOM);
	// scope to the table so the name matches a single element at 1280 viewport.
	return page.getByRole('table').getByText(name, { exact: true });
}

// ---------- Create (/secrets/new) ----------

export async function openSecretCreate(page: Page): Promise<void> {
	await page.goto('/secrets/new', { waitUntil: 'domcontentloaded' });
	await waitForVisible(page, '#secret-name');
}

export async function fillSecretName(page: Page, name: string): Promise<void> {
	await page.locator('#secret-name').fill(name);
}

export async function fillSecretValue(page: Page, value: string): Promise<void> {
	await page.locator('#secret-value').fill(value);
}

export async function submitCreateSecret(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Add Secret|Creating\.\.\.)$/ }).click();
}

// ---------- Detail (/secrets/{id}) ----------

export async function openSecretDetail(page: Page, id: string): Promise<void> {
	await page.goto(`/secrets/${id}`, { waitUntil: 'domcontentloaded' });
	await waitForVisible(page, '#secret-name');
}

export function saveButton(page: Page) {
	return page.getByRole('button', { name: /^(Save Changes|Saving\.\.\.)$/ });
}

// ---------- Delete modal ----------

// The detail-header delete trigger (default DeleteModal trigger button). Only
// rendered when the user has secret::delete.
export function deleteTrigger(page: Page) {
	return page.getByRole('button', { name: /^Delete$/ });
}

// The modal's confirm submit (disabled until the typed name matches).
function deleteConfirm(page: Page) {
	return page.locator('button[type="submit"]', { hasText: /^Delete(ing\.\.\.)?$/ });
}

// Opens the delete modal from the detail header, types the secret name to
// satisfy the name-match confirmation, and clicks the confirm button.
export async function deleteSecretViaModal(page: Page, name: string): Promise<void> {
	await deleteTrigger(page).first().click();
	await page.getByText('Delete Secret', { exact: true }).waitFor({
		state: 'visible',
		timeout: HYDRATION_TIMEOUT,
	});
	// The confirmation input is the last text input on the page: the detail
	// form's name field sits behind the modal.
	await page.locator('input[type="text"]').last().fill(name);
	await expect(deleteConfirm(page)).toBeEnabled();
	await deleteConfirm(page).click();
}
