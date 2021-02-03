FROM ubuntu:lts

RUN apt update
RUN apt install -y openssh-server

#make a temp dir
RUN mkdir /temp

#copy shell script to temp dir
COPY assets/pi_tunnel/create-user.sh /temp
COPY assets/pi_tunnel/user-data /temp

#export credentials to env and run bash script
RUN source /temp/user-data
RUN  /temp/create-user.sh

#delete temp dir
RUN rm -rf /temp