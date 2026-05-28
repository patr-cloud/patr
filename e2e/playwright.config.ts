import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './specs',
  // Default to serial. The dev API + frontend stack handles concurrent test
  // contexts poorly (cargo run shared binary, Vinxi HMR, single postgres).
  // Tests that want parallelism opt in by running with `TEST_THREADS=N`.
  workers: process.env.TEST_THREADS ? Number(process.env.TEST_THREADS) : 1,
  timeout: 60_000,
  use: {
    baseURL: 'http://localhost:3001',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    { name: 'default', testIgnore: /@docker/ },
    { name: 'docker-26', grep: /@docker/, metadata: { dockerVersion: '26' } },
    { name: 'docker-25', grep: /@docker/, metadata: { dockerVersion: '25' } },
    { name: 'docker-24', grep: /@docker/, metadata: { dockerVersion: '24' } },
  ],
});
