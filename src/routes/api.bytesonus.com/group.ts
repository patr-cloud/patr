import { Router } from 'express';
import { createGroup, getGroupByName } from '../../models/database-modules/group';
import check from './middleware';
import { permissions } from '../../models/interfaces/permission';
import {
	getResourceByName, grantUserResource, createResource, grantGroupResource,
} from '../../models/database-modules/resource';
import { getUserByUsername } from '../../models/database-modules/user';
import { errors } from '../../config/errors';

const router = Router();


/*
 * Currently only site admins can create groups
* */
router.post('/', check(permissions.Group.create, 'site_admins'), async (req, res, next) => {
	if (!req.body.name) {
		return res.status(400).json({
			success: false,
		});
	}
	// TODO: Regex checks for name

	// TODO: Later, these resources would be
	// provisioned by the service class
	const data = await Promise.all([
		createGroup({
			groupId: null,
			name: req.body.name,
		}),
		createResource({
			resourceId: null,
			name: `${req.body.name}::deployer`,
			type: 'deployer',
		}),
		createResource({
			resourceId: null,
			name: `${req.body.name}::docker_registry`,
			type: 'docker_registry',
		}),
	]);
	return res.json({
		success: true,
		group: {
			groupId: data[0].groupId.toString('hex'),
			name: req.body.name,
		},
	});
});

router.delete('/:groupId', (req, res, next) => {
	// TODO: Deleting an organization has to delete
	// a LOT of stuff, handle this later
});

/*
 * Grant privileges to a resource to a whole group
 * Need to pass the roleId granted and the groupId
 * of the group
 *
 * Eg: /myOrg/resources/deployer::myAPI/groups
*/
router.post('/:groupName/resources/:resourceName/groups', (req, res, next) => {
	if (!req.body.roleId || !req.body.group) {
		return res.status(400).json({
			success: false,
		});
	}
	res.locals.resourceName = `${req.params.groupName}::${req.params.resourceName}`;

	return check(
		permissions.Resource.grantPriveleges,
		res.locals.resourceName,
	)(req, res, next);
}, async (req, res, next) => {
	const resource = await getResourceByName(res.locals.resourceName);

	const group = await getGroupByName(req.body.group);
	if (!group) {
		return res.status(400).json({
			success: false,
			error: errors.serverError,
		});
	}
	await grantGroupResource(
		group.groupId,
		resource.resourceId,
		req.body.roleId,
	);
	return res.json({
		success: true,
	});
});


/* Grant privileges to a resource to a single user
 * Needs to pass the roleId granted and the userId
 * of the user
 *
 * Eg: /myOrg/resources/deployer::myAPI/users
 *
 * Note that the same route can also be used to
 * add users to a group resource itself, by omitting
 * the resourceName parameter. So to add a user to\
 * myOrg, we would use:
 * /myOrg/resources/users
*/
router.post('/:groupName/resources/:resourceName?/users', (req, res, next) => {
	if (!req.body.roleId || !req.body.username) {
		return res.status(400).json({
			success: false,
		});
	}
	if (req.params.resourceName === undefined) {
		res.locals.resourceName = req.params.groupName;
	} else {
		res.locals.resourceName = `${req.params.groupName}::${req.params.resourceName}`;
	}

	return check(
		permissions.Resource.grantPriveleges,
		res.locals.resourceName,
	)(req, res, next);
}, async (req, res, next) => {
	const resource = await getResourceByName(res.locals.resourceName);

	const user = await getUserByUsername(req.body.username);

	if (!user) {
		return res.status(400).json({
			success: false,
			error: errors.serverError,
		});
	}
	await grantUserResource(
		user.userId,
		resource.resourceId,
		req.body.roleId,
	);
	return res.json({
		success: true,
	});
});
export default router;
