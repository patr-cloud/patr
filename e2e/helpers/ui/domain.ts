import type { Page } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/domains/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/domains/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/domains/$id.tsx (detail)

async function waitForVisible(page: Page, selector: string): Promise<void> {
	await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// ---------- List (/domains) ----------

export async function openDomainList(page: Page): Promise<void> {
	await page.goto('/domains', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
	return page.getByText('No Domains Added', { exact: true });
}

// "Add Domain" is a link to /domains/new (header at >=1, empty-state CTA at 0).
export function addDomainLink(page: Page) {
	return page.getByRole('link', { name: /Add Domain/i });
}

export function domainRow(page: Page, domain: string) {
	// List renders a mobile card grid and a desktop table (both in the DOM);
	// scope to the table so the name matches a single element at 1280 viewport.
	return page.getByRole('table').getByText(domain, { exact: true });
}

// ---------- Create (/domains/new) ----------

export async function openDomainCreate(page: Page): Promise<void> {
	await page.goto('/domains/new', { waitUntil: 'domcontentloaded' });
	await waitForVisible(page, '#domain-name');
}

export async function fillDomainName(page: Page, name: string): Promise<void> {
	await page.locator('#domain-name').fill(name);
}

export async function submitAddDomain(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Add Domain|Adding\.\.\.)$/ }).click();
}

export function requiredError(page: Page) {
	return page.getByText('Domain is required.', { exact: true });
}

export function urlLikeError(page: Page) {
	return page.getByText(/Enter a base domain only/i);
}

// The "Did you mean <suggestion>" button.
export function suggestionButton(page: Page, suggested: string) {
	return page.getByRole('button', { name: suggested, exact: true });
}

// ---------- Detail (/domains/{id}) ----------

export async function openDomainDetail(page: Page, id: string): Promise<void> {
	await page.goto(`/domains/${id}`, { waitUntil: 'domcontentloaded' });
}

export function verifyButton(page: Page) {
	return page.getByRole('button', { name: /^(Verify|Verifying\.\.\.)$/ });
}
