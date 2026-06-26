import type { ApiClient } from '@/helpers/api';
import { API_DIRECT_URL, DASHBOARD_URL } from '@/helpers/urls';
import { USER_AGENT } from '@/helpers/config';

// REST helpers for the domain feature.
//
// Domains are stored split into name + tld but the API returns the full domain
// (CONCAT(name,'.',tld)). They are GLOBALLY unique by (name, tld) among
// non-deleted rows — so the same domain cannot exist in two workspaces. Domain
// handling is unified — every domain is externally managed; the old
// Patr-controlled vs user-controlled split (and its nameserver type) is gone.

type Creds = { accessToken: string; clientIp: string };

const base = (ws: string) => `/workspace/${ws}/domain`;

// A unique, valid root ICANN domain. The name label is lowercase alnum (the DB
// CHECK on the label allows [a-z0-9-], no leading/trailing hyphen).
export function randomDomain(prefix = 'e2e'): string {
	return `${prefix}${crypto.randomUUID().replace(/-/g, '').slice(0, 12)}.com`;
}

export async function addDomainAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	domain?: string,
): Promise<{ id: string; domain: string }> {
	const name = domain ?? randomDomain();
	const resp = await api.request<{ id: string }>('POST', base(workspaceId), {
		token: user.accessToken,
		clientIp: user.clientIp,
		body: { domain: name },
	});
	return { id: resp.id, domain: name };
}

export type DomainInfo = {
	id: string;
	name: string;
	isVerified: boolean;
	lastVerified: string | null;
};

export async function getDomainInfoAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	domainId: string,
): Promise<DomainInfo> {
	// The response flattens WithId<WorkspaceDomain> at the top level
	// ({ id, name, isVerified, lastVerified }).
	return api.request<DomainInfo>('GET', `${base(workspaceId)}/${domainId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

export async function deleteDomainAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	domainId: string,
): Promise<void> {
	await api.request('DELETE', `${base(workspaceId)}/${domainId}`, {
		token: user.accessToken,
		clientIp: user.clientIp,
	});
}

// Verify does a real public DNS TXT lookup that can't succeed offline → returns
// { verified: false } and never sets is_verified. The only path to a verified
// domain in e2e is the markDomainVerified DB backdoor.
export async function verifyDomainAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	domainId: string,
): Promise<boolean> {
	const resp = await api.request<{ verified: boolean }>(
		'POST',
		`${base(workspaceId)}/${domainId}/verify`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.verified;
}

export async function isDomainValidAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	domain: string,
): Promise<boolean> {
	const resp = await api.request<{ valid: boolean }>(
		'GET',
		`${base(workspaceId)}/is-valid?domain=${encodeURIComponent(domain)}`,
		{ token: user.accessToken, clientIp: user.clientIp },
	);
	return resp.valid;
}

export async function listDomainsAPI(
	api: ApiClient,
	user: Creds,
	workspaceId: string,
	query = '',
	opts: { direct?: boolean } = {},
): Promise<{ domains: DomainInfo[]; totalCount: number | null }> {
	const baseUrl = opts.direct ? API_DIRECT_URL : `${DASHBOARD_URL}/api`;
	const res = await fetch(`${baseUrl}${base(workspaceId)}${query}`, {
		headers: {
			'X-Real-IP': user.clientIp,
			'User-Agent': USER_AGENT,
			Authorization: `Bearer ${user.accessToken}`,
		},
	});
	const text = await res.text();
	if (!res.ok) throw new Error(`listDomainsAPI → ${res.status}: ${text.slice(0, 300)}`);
	const header = res.headers.get('x-total-count');
	const body = JSON.parse(text) as { domains: DomainInfo[] };
	return { domains: body.domains, totalCount: header === null ? null : Number(header) };
}
