#!/usr/bin/env bash

echo "running script file"
username=aniket-test
password=fancy-nigga-higga

adduser "$username"
echo $username:$password | chpasswd "$username"
# echo -e "$password" | passwd --stdin "$username"
# echo -e "$password\n$password\n" | sudo passwd $username

# expect "New password:" 
# send "$password\r" 

# expect "Retype new password:" 
# send "$password\r"

echo "finished script file"