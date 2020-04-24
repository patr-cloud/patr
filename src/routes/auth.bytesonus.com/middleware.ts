import { RequestHandler } from "express";
import oidc from "./oidc/provider";

export const sessionCheck: RequestHandler = async (req, res, next) => {
    // Checks if auth.bytesonus.com was accessed through a client, and not directly, which doesn't make sense
    let interaction;
    try {
        interaction = (await oidc.interactionDetails(req, res));
        res.locals.interaction = interaction;
        next();
    } catch {
        res.redirect('/');
    }
}
