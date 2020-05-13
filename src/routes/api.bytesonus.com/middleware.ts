import { RequestHandler } from 'express';
import checkIfUserHasPermission from '../../models/database-modules/permission';
import { errors, messages } from '../../config/errors';
import { Permission } from '../../models/interfaces/permission';
import AccessToken from '../../models/interfaces/access-token';


/* Middleware to authorize the jwt the user sends along, and check
 * if the user is allowed to perform the permission on resourceName
 *
 * Note that resourceName can use the :: seperator to perform permission
 * checks in a hierarchial fashion.
 *
 * For example,
 *
 * check(permissions.deployer.update, 'orgName::deployer::myAPI')
 *
 * will check if the create permission was granted on either the
 * orgName resource, the orgName::deploter resource, or the
 * orgName::deployer::myAPI resource
 *
* */
export default function check(
	permission: Permission,
	resourceName: string,
): RequestHandler {
	return async (req, res, next) => {
		if (!req.headers.authorization) {
			return res.status(401).json({
				success: false,
				error: errors.unauthorized,
				message: messages.unauthorized,
			});
		}

		// TODO: Implement blacklisting JWTs if the user is added to
		// a new group, since now the token will now be outdated...
		const accessToken = AccessToken.parse(req.headers.authorization); // Verify and parse the jwt
		const resourceNames = resourceName.split(':').map((_, i, resources) => resources.slice(0, i + 1).join('::'));

		if (await checkIfUserHasPermission(
			accessToken.userId,
			accessToken.groups,
			resourceNames,
			permission,
		)) {
			return next();
		}
		return res.status(401).json({
			success: false,
			error: errors.unauthorized,
			message: messages.unauthorized,
		});
	};
}
