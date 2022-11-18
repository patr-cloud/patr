#!/usr/bin/dumb-init /bin/sh
if [ -f /var/run/docker.pid ]; then
    rm /var/run/docker.pid
fi
dockerd