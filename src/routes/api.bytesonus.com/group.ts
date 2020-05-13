import { Router } from 'express';
import { createGroup } from '../../models/database-modules/group';
import check from './middleware';
import { permissions } from '../../models/interfaces/permission';
import { getResourceByName, grantUserResource } from '../../models/database-modules/resource';

const router = Router();


router.post('/', async (req, res, next) => {
	if (!req.body.name) {
		return res.status(400).json({
			success: false,
		});
	}
	// TODO: Regex checks for name
	const group = await createGroup({
		groupId: '',
		name: req.body.name,
	});
	return res.json({
		success: true,
		group,
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
	// TODO: Add group to resource
	// Same as next route mostly
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
	if (!req.body.roleId || !req.body.userId) {
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
	await grantUserResource(req.body.userId, resource.resourceId, req.body.roleId);
	res.json({
		success: true,
	});
});
export default router;
