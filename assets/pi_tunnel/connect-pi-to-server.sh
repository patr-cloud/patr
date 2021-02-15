#!/usr/bin/env bash

#get local device information
localPort=localPortVariable
localHostName=localHostNameVaribale

#get remote server information
exposedServerPort=exposedServerPortVariable
serverSSHPort=serverSSHPortVariable
serverHostNameOrIpAddress=serverHostNameOrIpAddressVariable
serverUserName=serverUserNameVariable

#command to be executed
ssh -R $exposedServerPort:$localHostName:$localPort -p $serverSSHPort $serverUserName@$serverHostNameOrIpAddress
