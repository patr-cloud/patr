FROM node:lts

WORKDIR  /usr/src/app
COPY . .

RUN npm install
RUN npm run build

WORKDIR /usr/src/app/bin
ENTRYPOINT ["node", "main.js"]
