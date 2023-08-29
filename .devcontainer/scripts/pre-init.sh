baseDir=$(dirname $0)

mkdir -p $baseDir/../volume/config/init-data
mkdir -p $baseDir/../volume/config/docker-registry
mkdir -p $baseDir/../volume/config/nginx-certs
mkdir -p $baseDir/../volume/data/cargo
mkdir -p $baseDir/../volume/data/docker-registry
mkdir -p $baseDir/../volume/data/postgres
mkdir -p $baseDir/../volume/data/dockerd

# Setup init-data

USER_ID=$(id -u)
GROUP_ID=$(id -g)
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-$USER}

echo "$USER_ID" > $baseDir/../volume/config/init-data/user
echo "$GROUP_ID" > $baseDir/../volume/config/init-data/group
echo "COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME" > $baseDir/../../.env
touch $baseDir/../volume/config/init-data/.bashrc
