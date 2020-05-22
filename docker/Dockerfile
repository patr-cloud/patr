FROM node:lts AS node_build

WORKDIR  /usr/src/app


COPY package.json .
COPY src/package.json ./src/

RUN npm install && \
	cd src && \
	npm install

COPY . .

RUN npm run build

FROM node:lts
WORKDIR /usr/src/app

RUN apt update && \
    apt install dumb-init certbot
COPY --from=node_build /usr/src/app/bin/ .

ENTRYPOINT ["/usr/bin/dumb-init", "--"]
CMD node main.js
