FROM ubuntu:lts

RUN apt update
RUN apt install -y openssh-server

#make a temp dir
RUN mkdir /temp

#copy shell script to temp dir
COPY assets/pi_tunnel/createuser.sh /temp
COPY assets/pi_tunnel/test /temp

#export credentials to env and run bash script
RUN source /temp/test
RUN  /temp/createuser.sh

#delete temp dir
RUN rm -rf /temp