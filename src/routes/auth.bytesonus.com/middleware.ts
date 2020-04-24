import { RequestHandler } from 'express';
import Provider from 'oidc-provider';
import oidc from './oidc/provider';

export const sessionCheck: RequestHandler = async (req, res, next) => {
	// Checks if auth.bytesonus.com was accessed through a client,
	// and not directly, which doesn't make sense
	let interaction;
	try {
		interaction = await oidc.interactionDetails(req, res);
		res.locals.interaction = interaction;
		next();
	} catch {
		res.redirect('/');
	}
};

// Temporary hack to export interaction type as @types of oidc-provider doesn't export it
type Await<T> = T extends {
	then(onfulfilled?: (value: infer U) => unknown): unknown;
} ? U : T;
export type Interaction = Await<ReturnType<Provider['interactionDetails']>>;
