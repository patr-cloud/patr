import { test as base } from '@playwright/test';
import { type ApiClient, makeApiClient } from '@/helpers/api';
import { randomIPv4 } from '@/helpers/ip';

type Fixtures = {
  api: ApiClient;
};

export const test = base.extend<{}, Fixtures>({
  api: [
    async ({}, use) => {
      // app.patr.cloud server proxies /api/* to the same axum routes the
      // browser-side dashboard hits, so this URL mirrors real web traffic.
      await use(makeApiClient('http://localhost:3001/api'));
    },
    { scope: 'worker' },
  ],
});

// Re-export expect for convenience in specs.
export { expect } from '@playwright/test';

// Per-test browser context with X-Real-IP injected on requests to our API
// only. Setting it via `extraHTTPHeaders` would send it on every request
// including cross-origin ones (fonts.gstatic.com, challenges.cloudflare.com),
// which trips CORS preflight failures and breaks Cloudflare Turnstile.
export async function newContext(
  browser: import('@playwright/test').Browser,
  clientIp = randomIPv4(),
) {
  const context = await browser.newContext();

  await context.route('http://localhost:3001/**', async (route) => {
    const headers = { ...route.request().headers(), 'x-real-ip': clientIp };
    await route.continue({ headers });
  });

  return context;
}
