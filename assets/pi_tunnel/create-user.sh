#!/usr/bin/env bash

echo "running script file"

source /temp/user-data
username=$TUNNEL_USERNAME
password=$TUNNEL_PASSWORD

useradd "$username"
echo $username:$password | chpasswd "$username"

echo "finished script file"