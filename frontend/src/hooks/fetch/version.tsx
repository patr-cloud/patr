import { createQuery } from "@tanstack/solid-query";
import { GetVersionResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";

export const useApiVersionQuery = () => {
	return createQuery<GetVersionResponse>(() => ({
		queryKey: ["api-version"],
		staleTime: 5 * 60 * 1000,
		meta: { errorMessage: "Failed to fetch API version" },
		queryFn: async () => {
			const response = await httpRequest<GetVersionResponse>(`${import.meta.env.VITE_BASE_URL}/api/version`, {
				method: "GET",
			});

			if (!response.ok) {
				throw new Error(response.data.error);
			}

			return response.data;
		},
	}));
};
