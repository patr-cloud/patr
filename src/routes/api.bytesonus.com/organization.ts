import { Router } from 'express';
import { createOrganization, getOrganizationByName } from '../../models/database-modules/organization';
import check from './middleware';
import { permissions } from '../../models/interfaces/permission';
import {
	getResourceByName, grantUserResource, createResource, grantOrgResource,
} from '../../models/database-modules/resource';
import { getUserByUsername } from '../../models/database-modules/user';
import { errors } from '../../config/errors';
import getJunoModule from '../../module';

const router = Router();


/*
 * Currently only site admins can create groups
* */
router.post('/', check(permissions.Organization.create, 'site_admins'), async (req, res, next) => {
	if (!req.body.name) {
		return res.status(400).json({
			success: false,
		});
	}
	// TODO: Regex checks for name

	// TODO: Later, these resources would be
	// provisioned by the service class
	const [organization, resource, registry] = await Promise.all([
		createOrganization({
			organizationId: null,
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

	// Make user who made the organization as owner
	await grantUserResource(
		res.locals.user.userId,
		resource.resourceId,
		0,
	);
	const module = await getJunoModule();
	module.triggerHook('createOrganization', {
		name: req.body.name,
		username: res.locals.user.username,
	});
	return res.json({
		success: true,
		organization: {
			organizationId: organization.organizationId.toString('hex'),
			name: req.body.name,
		},
	});
});

router.delete('/:organizationId', (req, res, next) => {
	// TODO: Deleting an organization has to delete
	// a LOT of stuff, handle this later
});

/*
 * Grant privileges to a resource to a whole organization
 * Need to pass the roleId granted and the organizationId
 * of the organization
 *
 * Eg: /myOrg/resources/deployer::myAPI/organizations
*/
router.post('/:orgName/resources/:resourceName/organizations', (req, res, next) => {
	if (!req.body.roleId || !req.body.organization) {
		return res.status(400).json({
			success: false,
		});
	}
	res.locals.resourceName = `${req.params.orgName}::${req.params.resourceName}`;

	return check(
		permissions.Resource.grantPriveleges,
		res.locals.resourceName,
	)(req, res, next);
}, async (req, res, next) => {
	const resource = await getResourceByName(res.locals.resourceName);

	const org = await getOrganizationByName(req.body.organization);
	if (!org) {
		return res.status(400).json({
			success: false,
			error: errors.serverError,
		});
	}
	await grantOrgResource(
		org.organizationId,
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
router.post('/:orgName/resources/:resourceName?/users', (req, res, next) => {
	if (!req.body.roleId || !req.body.username) {
		return res.status(400).json({
			success: false,
		});
	}
	if (req.params.resourceName === undefined) {
		res.locals.resourceName = req.params.orgName;
	} else {
		res.locals.resourceName = `${req.params.orgName}::${req.params.resourceName}`;
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
