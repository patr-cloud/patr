import { Request, Response, NextFunction } from 'express';
import { errors, messages } from '../../config/errors';
import HashSet = require('hashset');
import { AccessTokenData } from '../../models/interfaces/access-token-data';

// let bannedTokens = new HashSet();

export function tokenAuthenticator(options: string | Array<string | Array<string>>) {
	if (options === undefined) {
		options = '*';
	}
	const expressMiddleware = async (req: Request, res: Response, next: NextFunction) => {
		try {
			if (!req.headers.authorization) {
				if (options == null) {
					req.tokenData = null;
					next();
					return;
				}
				res.status(401).json({
					success: false,
					error: errors.unauthorized,
					message: messages.unauthorized
				});
				return;
			}

			const accessObject = await AccessTokenData.parse(req.headers.authorization);
			if (accessObject.exp <= Date.now()) {
				res.status(401).json({
					success: false,
					error: errors.expired,
					message: messages.expired
				});
				return;
			}
			const groups = accessObject.groups;
			let allowed = false;

			if (typeof options === 'string') {
				// If there's only one string, check if groups contain it
				if (options === '*' || groups.includes(options)) {
					allowed = true;
				}
			} else if (options instanceof Array) {
				// If there're multiple strings, iterate through all of them
				// and check if either of them are there
				for (let i = 0; i < options.length; i++) {
					const option = options[i];
					if (typeof option === 'string') {
						// If an item is a string, check if it's there in groups
						// If either one of them works, allow it and exit the loop
						if (groups.includes(option)) {
							allowed = true;
							break;
						}
					} else if (option instanceof Array) {
						// If an item is an array, check if all of them are there in groups
						// If they are, then allow it and exit the loop
						allowed = option.length > 0;
						option.forEach(item => {
							if (!groups.includes(item)) {
								allowed = false;
							}
						});
						if (allowed) {
							break;
						}
					}
				}
			} else {
				allowed = true;
			}

			if (allowed !== true) {
				res.status(403).json({
					success: false,
					error: errors.unprivileged,
					message: messages.unprivileged
				});
				return;
			}

			req.tokenData = accessObject;

			// TODO token banning and stuff

			next();
		} catch (err) {
			res.status(500).json({
				success: false,
				error: errors.serverError,
				message: messages.serverError
			});
			throw err;
		}
	};
	return expressMiddleware;
}
