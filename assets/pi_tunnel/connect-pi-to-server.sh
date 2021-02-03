#!/usr/bin/env bash

#get local device information
localPort=localPortVariable
localHostName=localHostNameVaribale

#get remote server information
serverPort=serverPortVariable
serverHostNameOrIpAddress=serverHostNameOrIpAddressVariable
serverUserName=serverUserNameVariable

#command to be executed
ssh -R $serverPort:$localHostName:$localPort $serverUserName@$serverHostNameOrIpAddress