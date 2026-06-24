import { defineConfig } from '@playwright/test';
import { DASHBOARD_URL } from './helpers/urls';

// Which Docker versions the @docker (real-runner) suite runs against. Comma-
// separated, default "26" so local runs use a single version; CI sets
// DOCKER_VERSIONS=24 / 25 / 26 to shard one version per parallel job.
const dockerVersions = (process.env.DOCKER_VERSIONS ?? '26')
  .split(',')
  .map((v) => v.trim())
  .filter(Boolean);

export default defineConfig({
  testDir: './specs',
  // Default to serial. The dev API + frontend stack handles concurrent test
  // contexts poorly (cargo run shared binary, Vinxi HMR, single postgres).
  // Tests that want parallelism opt in by running with `TEST_THREADS=N`.
  workers: process.env.TEST_THREADS ? Number(process.env.TEST_THREADS) : 1,
  timeout: 60_000,
  use: {
    baseURL: DASHBOARD_URL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    // @docker is a test-title tag (set in `test.describe('… @docker')`), not a
    // path — so exclude it by title with grepInvert (symmetric with the docker
    // projects' title-based `grep`). testIgnore would only match file paths and
    // would let DinD-backed tests run in the default (non-docker) shard.
    { name: 'default', grepInvert: /@docker/ },
    ...dockerVersions.map((v) => ({
      name: `docker-${v}`,
      grep: /@docker/,
      metadata: { dockerVersion: v },
    })),
  ],
});
