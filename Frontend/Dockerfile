# pull official base image
FROM node:18.16.1-buster

# set working directory in the container
WORKDIR /app

# copy package.json and package-lock.json to working dir
COPY /package*.json ./

# install app dependencies
RUN npm install -g npm@9.7.2 --silent
RUN npm install --silent

# Install three.js, three-stdlib, and styled-components
RUN npm install three three-stdlib styled-components --silent

# copy the app code to the container
COPY . /app

# Expose the port the app runs in
EXPOSE 3000

# start the app
CMD ["npm", "run", "start"]
