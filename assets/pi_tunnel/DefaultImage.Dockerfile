FROM ubuntu:lts

RUN apt update
RUN apt install -y openssh-server

#make a temp dir
RUN mkdir /temp

#copy shell script to temp dir
COPY assets/pi_tunnel/create-user.sh /temp

#export credentials to env and run bash script
RUN  /temp/create-user.sh

# expose port in the docker image
EXPOSE 8080 4343
#delete temp dir
RUN rm -rf /temp