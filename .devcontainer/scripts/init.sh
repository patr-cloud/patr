#!/bin/bash

baseDir=$(dirname $0)

mkdir -p $baseDir/../volume/init-data

USER_ID=$(id -u)
GROUP_ID=$(id -g)
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-$USER}

echo "$USER_ID" > $baseDir/../volume/init-data/user
echo "$GROUP_ID" > $baseDir/../volume/init-data/group
echo "COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME" > $baseDir/../../.env
