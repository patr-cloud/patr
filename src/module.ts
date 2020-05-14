import JunoModule from 'juno-node';

const moduleCreationPromise = JunoModule.default(process.env.JUNO_SOCK);
let module: JunoModule = null;
let loaded = false;
export default async function getJunoModule() {
	if (!loaded) {
		module = await moduleCreationPromise;
		loaded = true;
	}
	return module;
}
