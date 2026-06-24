import { test as base } from '@playwright/test';
import { type ApiClient, makeApiClient } from '@/helpers/api';
import { randomIPv4 } from '@/helpers/ip';
import { DASHBOARD_URL } from '@/helpers/urls';

type Fixtures = {
  api: ApiClient;
};

export const test = base.extend<{}, Fixtures>({
  api: [
    async ({}, use) => {
      // app.patr.cloud server proxies /api/* to the same axum routes the
      // browser-side dashboard hits, so this URL mirrors real web traffic.
      await use(makeApiClient(`${DASHBOARD_URL}/api`));
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
//
// Also bounds context.close() — under Vinxi dev the SolidStart HMR websocket
// and React-Query background polls can stall close indefinitely, eating the
// full per-test 60s timeout. Forcing a fast close keeps test runtime bounded.
export async function newContext(
  browser: import('@playwright/test').Browser,
  clientIp = randomIPv4(),
) {
  const context = await browser.newContext();

  // Only route /api/** — every routed request round-trips through Playwright's
  // IPC, and Vite's dev server fires hundreds of HMR + module requests per
  // page load. Routing them all starves Playwright's internal scheduler and
  // makes page.waitForTimeout/expect-polling take 60s instead of ms.
  await context.route(`${DASHBOARD_URL}/api/**`, async (route) => {
    const headers = { ...route.request().headers(), 'x-real-ip': clientIp };
    await route.continue({ headers });
  });

  // Bound context.close() (as the comment above promises). Under Vinxi dev the
  // SolidStart HMR websocket and React-Query background polls keep a dashboard
  // page busy, so the native close can stall indefinitely and eat the 60s test
  // timeout — e.g. after a successful onboard navigates to the dashboard.
  // Closing the pages first stops that activity so the native close returns
  // immediately; the race is a backstop, and since the pages are already closed
  // a timed-out context is inert (no polls/socket left to leak).
  const nativeClose = context.close.bind(context);
  context.close = (async () => {
    await Promise.race([
      (async () => {
        await Promise.all(context.pages().map((page) => page.close().catch(() => {})));
        await nativeClose();
      })(),
      new Promise<void>((resolve) => setTimeout(resolve, 5_000)),
    ]);
  }) as typeof context.close;

  return context;
}
