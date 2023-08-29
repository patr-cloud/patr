baseDir=$(dirname $0)

if [ -f /workspace/.devcontainer/volume/config/init-data/.bashrc ]; then
	cat /workspace/.devcontainer/volume/config/init-data/.bashrc >> ~/.bashrc
fi

mv -n ~/.cargo/bin ~/.cargo-volume/bin
mv -n ~/.cargo/env ~/.cargo-volume/env
rm -rf ~/.cargo/
ln -s ~/.cargo-volume ~/.cargo

# Setup kubeconfig
source /workspace/.env
mkdir -p ~/.kube

# TODO setup kubernetes cluster with k3d and use that kubeconfig

echo "Installing sea-orm-cli and trunk"
cargo install sea-orm-cli trunk

if [ ! -f /workspace/config/dev.json ]; then
	echo "Setting up dev.json"
	read -r -d '' jqQuery <<- EOF
	.bindAddress |= "0.0.0.0" |
	.database.host |= "postgres" |
	.database.port |= 5432 |
	.database.user |= "postgres" |
	.database.password |= "postgres" |
	.redis.host |= "redis" |
	EOF
	cat $baseDir/../../config/dev.sample.json | jq "$jqQuery" > $baseDir/../../config/dev.json
fi
