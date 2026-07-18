import { execa } from 'execa';
import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL, DASHBOARD_URL, REGISTRY_HOST } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

// A tiny public HTTP server image used as the default deployment payload. Listens
// on :80 and echoes request info, so a test can assert the response came from
// the running container.
export const DEFAULT_TEST_IMAGE = 'traefik/whoami:latest';

export type ContainerRepo = { id: string; name: string };

// A lowercase, DB-CHECK-valid repo name. The create handler's preprocess regex
// (^[a-zA-Z0-9\-_ .]{4,255}$) is looser than the table's CHECK constraint
// (^[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*...$), so anything with uppercase/space/
// leading punctuation passes preprocess but 500s at INSERT. Default fixture
// names must satisfy the stricter DB regex.
export function randomRepoName(prefix = 'e2erepo'): string {
	return `${prefix}-${crypto.randomUUID().replace(/-/g, '').slice(0, 10)}`;
}

type Creds = { accessToken: string; clientIp: string };

const repoBase = (workspaceId: string) => `/workspace/${workspaceId}/container-registry`;

// Create a container repository via the API. Push does not auto-create repos.
export async function createContainerRepo(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	name?: string,
): Promise<ContainerRepo> {
	const repoName = name ?? randomRepoName();
	const resp = await api.request<{ id: string }>('POST', repoBase(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: { name: repoName },
	});
	return { id: resp.id, name: repoName };
}

export type RepositoryInfo = {
	name: string;
	size: number;
	lastUpdated: string;
	created: string;
};

export async function getRepoInfoAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
): Promise<RepositoryInfo> {
	const resp = await api.request<{ repository: RepositoryInfo }>(
		'GET',
		`${repoBase(workspaceId)}/${repoId}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.repository;
}

export type RepositoryListItem = RepositoryInfo & { id: string };

// Lists repositories, returning both the rows and the x-total-count header (the
// list endpoint returns membership-filtered rows; the header is the filtered
// total). The api.request helper doesn't expose response headers, so this uses
// raw fetch. User JWTs must go through the dashboard proxy (the direct
// entrypoint treats a Bearer token as an API token → malformedApiToken); set
// `direct: true` for API-token auth, which must use the direct entrypoint.
export async function listReposAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	query = '',
	opts: { direct?: boolean } = {},
): Promise<{ repositories: RepositoryListItem[]; totalCount: number | null }> {
	const baseUrl = opts.direct ? API_DIRECT_URL : `${DASHBOARD_URL}/api`;
	const res = await fetch(`${baseUrl}${repoBase(workspaceId)}${query}`, {
		headers: {
			'X-Real-IP': user.clientIp,
			'User-Agent': USER_AGENT,
			Authorization: `Bearer ${user.accessToken}`,
		},
	});
	const text = await res.text();
	if (!res.ok) {
		throw new Error(`listReposAPI → ${res.status}: ${text.slice(0, 300)}`);
	}
	const header = res.headers.get('x-total-count');
	const body = JSON.parse(text) as { repositories: RepositoryListItem[] };
	return { repositories: body.repositories, totalCount: header === null ? null : Number(header) };
}

export async function deleteRepoAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
): Promise<void> {
	await api.request('DELETE', `${repoBase(workspaceId)}/${repoId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export type ManifestInfo = {
	digest: string;
	size: number;
	platform: string;
	created: string;
	tags: string[];
};

export async function listManifestsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
	query = '',
): Promise<ManifestInfo[]> {
	const resp = await api.request<{ manifests: ManifestInfo[] }>(
		'GET',
		`${repoBase(workspaceId)}/${repoId}/manifest${query}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.manifests;
}

export type TagInfo = { tag: string; digest: string; lastUpdated: string };

export async function listTagsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
	query = '',
): Promise<TagInfo[]> {
	const resp = await api.request<{ tags: TagInfo[] }>(
		'GET',
		`${repoBase(workspaceId)}/${repoId}/tag${query}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.tags;
}

// Delete a manifest by digest OR tag name. Note: deleting by tag name → 404
// (the handler matches on manifest_digest); only digest deletes succeed.
export async function deleteManifestAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
	digestOrTag: string,
): Promise<void> {
	await api.request('DELETE', `${repoBase(workspaceId)}/${repoId}/manifest/${digestOrTag}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export async function exposedPortsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	repoId: string,
	digestOrTag: string,
): Promise<number[]> {
	const resp = await api.request<{ ports: number[] }>(
		'GET',
		`${repoBase(workspaceId)}/${repoId}/manifest/${digestOrTag}/exposed-ports`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.ports;
}

// Push an image into the Patr registry under {workspaceId}/{repoName}:{tag} via
// the DinD daemon (`dockerHost`). The DinD has a socat bridge so that
// `registry.patr.cloud` resolves to the host API's registry; pushing through the
// same daemon the runner pulls with means no host-side registry resolution and
// no production code override. Auth is the docker-login token flow (Basic
// user=patr, password=<api token> → the realm issues a bearer token).
export async function pushImageToPatrRegistry(opts: {
	dockerHost: string;
	workspaceId: string;
	repoName: string;
	tag: string;
	apiToken: string;
	sourceImage?: string;
}): Promise<{ ref: string }> {
	const source = opts.sourceImage ?? DEFAULT_TEST_IMAGE;
	const ref = `${REGISTRY_HOST}/${opts.workspaceId}/${opts.repoName}:${opts.tag}`;
	const docker = (args: string[]) => execa('docker', ['-H', opts.dockerHost, ...args]);

	await docker(['pull', source]);
	await docker(['tag', source, ref]);
	await docker(['login', REGISTRY_HOST, '-u', 'patr', '-p', opts.apiToken]);
	await docker(['push', ref]);

	return { ref };
}

export type DockerResult = { ok: boolean; stdout: string; stderr: string };

// `docker login` against the Patr registry, returning the result without
// throwing so auth-negative cases (wrong username, bad/expired/revoked token)
// can assert the failure.
export async function dockerLoginPatr(
	dockerHost: string,
	apiToken: string,
	username = 'patr',
): Promise<DockerResult> {
	const res = await execa(
		'docker',
		['-H', dockerHost, 'login', REGISTRY_HOST, '-u', username, '-p', apiToken],
		{ reject: false },
	);
	return { ok: res.exitCode === 0, stdout: res.stdout, stderr: res.stderr };
}

// Tag a (already-pulled or to-be-pulled) source image to the Patr ref and push,
// returning the result without throwing. Used for negative push cases (no-push
// permission → NameUnknown, nonexistent repo, cross-workspace).
export async function tryPushImage(opts: {
	dockerHost: string;
	workspaceId: string;
	repoName: string;
	tag: string;
	apiToken: string;
	sourceImage?: string;
}): Promise<DockerResult & { ref: string }> {
	const source = opts.sourceImage ?? DEFAULT_TEST_IMAGE;
	const ref = `${REGISTRY_HOST}/${opts.workspaceId}/${opts.repoName}:${opts.tag}`;
	const docker = (args: string[]) =>
		execa('docker', ['-H', opts.dockerHost, ...args], { reject: false });
	await docker(['pull', source]);
	await docker(['tag', source, ref]);
	await docker(['login', REGISTRY_HOST, '-u', 'patr', '-p', opts.apiToken]);
	const res = await docker(['push', ref]);
	return { ok: res.exitCode === 0, stdout: res.stdout, stderr: res.stderr, ref };
}

// docker login + pull the given Patr ref back, returning the result without
// throwing (so pull-permission negatives can assert failure).
export async function pullImageFromPatrRegistry(opts: {
	dockerHost: string;
	workspaceId: string;
	repoName: string;
	tag: string;
	apiToken: string;
}): Promise<DockerResult & { ref: string }> {
	const ref = `${REGISTRY_HOST}/${opts.workspaceId}/${opts.repoName}:${opts.tag}`;
	const docker = (args: string[]) =>
		execa('docker', ['-H', opts.dockerHost, ...args], { reject: false });
	await docker(['login', REGISTRY_HOST, '-u', 'patr', '-p', opts.apiToken]);
	// Remove any local copy so the pull actually hits the registry.
	await docker(['rmi', '-f', ref]);
	const res = await docker(['pull', ref]);
	return { ok: res.exitCode === 0, stdout: res.stdout, stderr: res.stderr, ref };
}
