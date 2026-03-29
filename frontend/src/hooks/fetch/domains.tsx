import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetVerificationRecordsForDomainResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { domainKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

type WorkspaceDomain = {
	id: string;
	name: string;
	nameserverType: string;
	isVerified: boolean;
};

type GetDomainsForWorkspaceResponse = {
	domains: WorkspaceDomain[];
};

export const useDomainsQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: domainKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<GetDomainsForWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch domains", "error");
					throw new Error(response.data.error);
				}

				return {
					domains: response.data.domains || [],
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useDomainVerificationRecordsQuery = (domainId: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const dId = domainId();
		return {
			queryKey: domainKeys.verificationRecords(wsId ?? "", dId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!dId,
			queryFn: async () => {
				const response = await httpRequest<GetVerificationRecordsForDomainResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/domain/${dId}/verification-records`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch domain verification records", "error");
					throw new Error(response.data.error);
				}

				return { records: response.data.verificationRecords || [] };
			},
		};
	});
};
