import { RequestHandler } from 'express';
import checkIfUserHasPermission from '../../models/database-modules/permission';
import { errors, messages } from '../../config/errors';
import { Permission } from '../../models/interfaces/permission';
import AccessToken from '../../models/interfaces/access-token';


const siteAdminsUUID = Buffer.from('0'.repeat(32), 'hex');

/* Middleware to authorize the jwt the user sends along, and check
 * if the user is allowed to perform the permission (or ALL of multiple permissions)
 * on the resource with name resourceName
 *
 * Note that resourceName can use the :: seperator to perform permission
 * checks in a hierarchial fashion.
 *
 * For example,
 *
 * check(permissions.deployer.update, 'orgName::deployer::myAPI')
 *
 * will check if the update permission was granted on either the
 * orgName resource, the orgName::deploter resource, or the
 * orgName::deployer::myAPI resource
 *
* */
export default function check(
	permission: Permission | Permission[],
	resourceName: string,
): RequestHandler {
	let permissions: Permission[];
	if (!Array.isArray(permission)) {
		permissions = [permission];
	} else {
		permissions = permission;
	}
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

		let accessToken: AccessToken;
		try {
			accessToken = AccessToken.parse(req.headers.authorization); // Verify and parse the jwt
		} catch (e) {
			// JWT is invalid
			return res.status(401).json({
				success: false,
				error: errors.unauthorized,
				message: messages.unauthorized,
			});
		}

		const userId = Buffer.from(accessToken.userId, 'hex');
		const userGroups = accessToken.groups.map((g) => Buffer.from(g, 'hex'));

		if (userGroups.find((groupId) => groupId.equals(siteAdminsUUID))) {
			// If the site admin group is present in the user's groups, then
			// all permissions are granted
			return next();
		}

		const granted = await checkIfUserHasPermission(
			userId,
			userGroups,
			resourceName,
			permissions,
		);
		if (granted.every((g) => g === true)) {
			return next();
		}
		return res.status(401).json({
			success: false,
			error: errors.unauthorized,
			message: messages.unauthorized,
		});
	};
}
