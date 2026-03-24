import { createSignal, JSX } from "solid-js";
import { useToast } from "~/components/toast";
import { useAuthState } from "./state-hooks";
import { useLastWorkspaceId } from "./state-hooks";

function createAsyncAction<TArgs extends unknown[], TReturn>(fn: (...args: TArgs) => Promise<TReturn>) {
	const [isLoading, setIsLoading] = createSignal(false);
	const [error, setError] = createSignal<unknown>(null);

	const execute = async (...args: TArgs): Promise<TReturn> => {
		setIsLoading(true);
		setError(null);
		try {
			return await fn(...args);
		} catch (e) {
			setError(e);
			throw e;
		} finally {
			setIsLoading(false);
		}
	};

	return { execute, isLoading, error };
}

interface AuthenticatedActionContext {
	accessToken: string;
	workspaceId: string;
}

interface LoggedInActionContext {
	accessToken: string;
}

const createLoggedInAction = (handler: (ctx: LoggedInActionContext) => Promise<void>) => {
	const [authState] = useAuthState();
	const toast = useToast();

	const { execute, isLoading, error } = createAsyncAction(async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in", "error");
			throw new Error("User is not logged in");
		}
		await handler({ accessToken: auth.accessToken });
	});

	return { execute, isLoading, error };
};

const createAuthenticatedAction = (handler: (ctx: AuthenticatedActionContext) => Promise<void>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const { execute, isLoading, error } = createAsyncAction(async () => {
		const auth = authState();
		const currentWorkspaceId = workspaceId();
		if (!auth || auth.type !== "LoggedIn" || !currentWorkspaceId) {
			toast("You must be logged in with a workspace selected", "error");
			throw new Error("User is not logged in or no workspace selected");
		}
		await handler({ accessToken: auth.accessToken, workspaceId: currentWorkspaceId });
	});

	return { execute, isLoading, error };
};

const createFormAction = (handler: (ctx: AuthenticatedActionContext) => Promise<void>, validate?: () => boolean) => {
	const { execute, isLoading, error } = createAuthenticatedAction(handler);

	const onSubmit: JSX.EventHandler<HTMLFormElement, SubmitEvent> = async (e) => {
		e.preventDefault();
		if (validate && !validate()) return;
		await execute().catch(() => {});
	};

	return { onSubmit, isLoading, error };
};

export { createAsyncAction, createAuthenticatedAction, createFormAction, createLoggedInAction };
export type { AuthenticatedActionContext, LoggedInActionContext };
