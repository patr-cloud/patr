#!/usr/bin/env bash

set -e

baseDir="$(dirname $0)"

# configure the project name to avoid docker compose project name clash
if [ -f "$baseDir/.env" ]; then
    set -o allexport
    source "$baseDir/.env"
    set +o allexport
fi

# empty env file contents
> "$baseDir/.env"

# populate env file contents
cat "./.env.template" | while read line; do
    # TODO: writing to the env file is not working as expected as
    # the eval command is not escaping the value content when writing it to .env file

    printf "%q\n" $(eval echo \"$line\")
    # eval echo $line >> "$baseDir/.env"
done
