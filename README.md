## Pasos para instalar las dependencias en VS Code

Aquí encontrarás los pasos necesarios para instalar las siguientes dependencias en Visual Studio Code (VS Code):

1. **Python**: 
   - Ve a la página de descargas de Python en [https://www.python.org/downloads/](https://www.python.org/downloads/).
   - Descarga la versión más reciente de Python según tu sistema operativo.
   - Ejecuta el instalador descargado y sigue las instrucciones para completar la instalación.

2. **MinGW GCC**:
   - Visita la documentación oficial de Visual Studio Code sobre la configuración de MinGW GCC en [https://code.visualstudio.com/docs/cpp/config-mingw](https://code.visualstudio.com/docs/cpp/config-mingw).
   - Sigue las instrucciones proporcionadas en la documentación para instalar MinGW GCC en tu sistema.

3. **NumPy**, **Cython**, **Trimesh**, **FastAPI**, **Aiofiles**, **Requests**:
   - Abre VS Code.
   - Abre una nueva terminal en VS Code (puedes hacerlo desde el menú `Terminal` -> `Nueva terminal` o usando el atajo de teclado `Ctrl + ` `).
   - En la terminal, ejecuta los siguientes comandos, uno por uno, para instalar las bibliotecas:
     ```
      pip install numpy
      pip install Cython
      pip install trimesh
      pip install fastapi
      pip install aiofiles
      pip install requests
      pip install scipy
      pip install cythonizer
      pip install uvicorn
      pip install python-multipart
      pip install emit
      pip install flask-cors

     ```

Una vez que hayas completado estos pasos, habrás instalado todas las dependencias necesarias en VS Code. Ahora podrás utilizar estas bibliotecas en tus proyectos de Python en el entorno de desarrollo de VS Code.


# Guía de Inicio para Dockerfiles

Este README proporciona instrucciones paso a paso sobre cómo construir y ejecutar contenedores Docker utilizando los Dockerfiles proporcionados.

## Dockerfile para Python FastAPI (uvicorn)

```Dockerfile
# Use an official Python runtime as a parent image
FROM python:3.11.4

# Set the working directory in the container to /app
WORKDIR /app

# Add the current directory contents into the container at /app
ADD . /app

# Install any needed packages specified in requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Run setup.py
RUN python setup.py build_ext --inplace

# Make port 5000 available to the world outside this container
EXPOSE 5000

# Define environment variable
ENV NAME World

# Run uvicorn when the container launches
CMD ["uvicorn", "server:app", "--reload", "--host", "0.0.0.0", "--port", "5000"]
```

Para construir y ejecutar el contenedor Docker para la aplicación Python FastAPI, siga estos pasos:

1. Navegue hasta el directorio donde se encuentra el Dockerfile.

2. Ejecute el siguiente comando para construir la imagen Docker:

    ```
    docker build -t my-fastapi-app .
    ```

3. Ejecute el siguiente comando para ejecutar el contenedor Docker:

    ```
    docker run -p 5000:5000 my-fastapi-app
    ```

La aplicación ahora debería estar accesible en `http://localhost:5000`.

## Dockerfile para Node.js

```Dockerfile
# pull official base image
FROM node:18.16.1-buster

# set working directory in the container
WORKDIR /app

# copy package.json and package-lock.json to working dir
COPY /package*.json ./

# install app dependencies
RUN npm install -g npm@9.7.2 --silent
RUN npm install --silent

# copy the app code to the container
COPY . /app

# Expose the port the app runs in
EXPOSE 3000

# start the app
CMD ["npm", "run", "start"]
```

Para construir y ejecutar el contenedor Docker para la aplicación Node.js, siga estos pasos:

1. Navegue hasta el directorio donde se encuentra el Dockerfile.

2. Ejecute el siguiente comando para construir la imagen Docker:

    ```
    docker build -t my-node-app .
    ```

3. Ejecute el siguiente comando para ejecutar el contenedor Docker:

    ```
    docker run -p 3000:3000 my-node-app
    ```

La aplicación ahora debería estar accesible en `http://localhost:3000`.

