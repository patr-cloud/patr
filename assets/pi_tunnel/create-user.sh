#!/usr/bin/env bash

echo "running script file"

# export env variables
source /temp/user-data
username=$TUNNEL_USERNAME
password=$TUNNEL_PASSWORD

useradd "$username"
echo $username:$password | chpasswd "$username"

echo sed -i -r 's/PasswordAuthentication no/PasswordAuthentication yes/g' /etc/ssh/sshd_config

echo "finished script file"