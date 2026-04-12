import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetMfaSecretResponse, GetUserInfoResponse } from "~/bindings";
import { SearchForUserResponse } from "~/bindings/SearchForUserResponse";

import { useAuthState } from "~/hooks/state-hooks";
import { mfaKeys, userInfoKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useUserInfoQuery = () => {
	const [authState] = useAuthState();

	return createQuery<GetUserInfoResponse>(() => {
		const auth = authState();
		return {
			queryKey: userInfoKeys.current(),
			enabled: !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch user info" },
			queryFn: async () => {
				const response = await httpRequest<GetUserInfoResponse>(`${import.meta.env.VITE_BASE_URL}/api/user`, {
					method: "GET",
				});

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

export const useUserSearchQuery = (query: Accessor<string>) => {
	const [authState] = useAuthState();

	return createQuery<SearchForUserResponse>(() => {
		const auth = authState();
		const q = query();
		return {
			queryKey: userInfoKeys.search(q),
			enabled: !!auth && auth.type === "LoggedIn" && q.length >= 2,
			queryFn: async () => {
				const response = await httpRequest<SearchForUserResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/search?query=${encodeURIComponent(q)}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

export const useMfaSecretQuery = (enabled: Accessor<boolean>) => {
	const [authState] = useAuthState();

	return createQuery<GetMfaSecretResponse>(() => {
		const auth = authState();
		const isEnabled = enabled();
		return {
			queryKey: mfaKeys.secret(),
			enabled: !!auth && auth.type === "LoggedIn" && isEnabled,
			meta: { errorMessage: "Failed to fetch MFA secret" },
			queryFn: async () => {
				const response = await httpRequest<GetMfaSecretResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/user/mfa`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};
