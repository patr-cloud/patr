#!/bin/bash

baseDir=$(dirname $0)

workspaceDir=$baseDir/../..
docker run -w /workdir -v $workspaceDir:/workdir --rm ubuntu:22.04 /bin/bash -c "rm -rfv /workdir/.devcontainer/volume"
