# FROM ubuntu:latest
FROM linuxserver/openssh-server:amd64-latest

# RUN apt update
# RUN apt install -y openssh-server

#make a temp dir
RUN mkdir /temp

#copy shell script to temp dir
COPY create-user.sh /temp

#export credentials to env and run bash script
# expose port in the docker image
EXPOSE 8081 4343

# CMD ["bash","-c", "/temp/create-user.sh && rm /temp/create-user.sh"]