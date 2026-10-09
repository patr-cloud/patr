import type { Page } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/runners/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/runners/new.tsx (create)
//   frontend/src/routes/_logged-in/_workspaced/runners/$id.tsx (detail: info/metrics/logs)

async function waitForVisible(page: Page, selector: string): Promise<void> {
	await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// ---------- List (/runners) ----------

export async function openRunnerList(page: Page): Promise<void> {
	await page.goto('/runners', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
	return page.getByText('No Runners Added', { exact: true });
}

// "Add Runner" is a link to /runners/new (header button at >=1, empty-state CTA at 0).
export function addRunnerLink(page: Page) {
	return page.getByRole('link', { name: /Add Runner/i });
}

export function runnerRow(page: Page, name: string) {
	// List renders a mobile card grid and a desktop table (both in the DOM);
	// scope to the table so the name matches a single element at 1280 viewport.
	return page.getByRole('table').getByText(name, { exact: true });
}

// ---------- Create (/runners/new) ----------

export async function openRunnerCreate(page: Page): Promise<void> {
	await page.goto('/runners/new', { waitUntil: 'domcontentloaded' });
	await waitForVisible(page, '#runner-name');
}

export async function fillRunnerName(page: Page, name: string): Promise<void> {
	await page.locator('#runner-name').fill(name);
}

export async function submitCreateRunner(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Create Runner|Creating Runner\.\.\.)$/ }).click();
}

export function nameErrorAlert(page: Page) {
	return page.getByText('Runner name is required.', { exact: true });
}

// ---------- Detail (/runners/{id}) ----------

export async function openRunnerDetail(page: Page, id: string, tab?: string): Promise<void> {
	const suffix = tab === undefined ? '' : `?tab=${tab}`;
	await page.goto(`/runners/${id}${suffix}`, { waitUntil: 'domcontentloaded' });
}

// Status chip text: "Connected" when connected, "Unreachable" otherwise. The DOM
// text is lowercase (the chip capitalizes it with CSS), so match case-insensitively.
export function statusBadge(page: Page, text: 'Connected' | 'Unreachable') {
	return page.getByText(new RegExp(`^${text}$`, 'i'));
}

export function infoTab(page: Page) {
	return page.getByRole('button', { name: 'Info', exact: true });
}

export function deleteRunnerButton(page: Page) {
	return page.getByRole('button', { name: 'Delete', exact: true });
}

export function metricsTab(page: Page) {
	return page.getByRole('button', { name: 'Metrics', exact: true });
}

export function logsTab(page: Page) {
	return page.getByRole('button', { name: 'Logs', exact: true });
}
