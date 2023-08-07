## Protein Docking

"Protein Docking" es un proyecto destinado a optimizar el algoritmo de docking para facilitar su uso en la investigación biomédica. Con un enfoque en mejorar la interfaz de usuario y resolver problemas de compatibilidad, este proyecto está diseñado para acelerar el proceso de investigación y es especialmente útil para profesionales en campos como la química y la biología. Se trata de una aplicación web intuitiva que no implica costos adicionales, utilizando los equipos y recursos ya existentes.

## Software stack
El proyecto Protein Docking es una aplicación web que se ejecuta sobre el siguiente software:

### Backend
- Python 3.8
- FastAPI
- Flask
- Cython
- Uvicorn

#### Dependencias de Python/Cython:
- Numpy
- Cython
- Trimesh
- Aiofiles
- Requests
- Scipy
- Cythonizer
- Python-multipart
- Emit

#### Dependencias de Flask:
- Flask-CORS
- Flask-RESTful
- Flask-SocketIO

### Frontend
- NodeJS
- React
- PrimeReact

#### Dependencias de NodeJS:
- Npm

#### Lenguajes
- JavaScript (JS)
- Cascading Style Sheets (CSS)

## Configuraciones de Ejecución para Entorno de Desarrollo
- Primero, clonaremos el repositorio

```
git clone https://github.com/yeipills/tesis.git 
```

- Segundo, nos movemos  dentro de la carpeta "tesis", para entrar a las demas carpetas se usa el mismo comando
  
```
cd tesis
```

- Este path es el que se utiliza para llegar al backend 
```
cd Backend\C-lculos-Previos-main\Centroides de triangulos\Programa_python 
```

Pasos para iniciar el backend, una vez situados en la carpeta Programa_python

1. Ejecute el siguiente comando para construir la imagen Docker:

    ```
    docker build -t my-fastAPI-app .
    ```

2. Ejecute el siguiente comando para ejecutar el contenedor Docker:

    ```
    docker run -p 5000:5000 my-fastAPI-app
    ```

- La aplicación ahora debería estar accesible en `http://localhost:5000`.


Este path es el que se utiliza para llegar al socket del backend 
```
cd Backend\C-lculos-Previos-main\Centroides de triangulos\Programa_python\socket
```
Pasos para iniciar el backend, una vez situados en la carpeta socket

1. Ejecute el siguiente comando para construir la imagen Docker:

    ```
    docker build -t my-socket .
    ```

2. Ejecute el siguiente comando para ejecutar el contenedor Docker:

    ```
    docker run -p 8000:8000 my-socket
    ```
    
- La aplicación ahora debería estar accesible en `http://localhost:8000`.


Este path es el que se utiliza para llegar al frontend
```
cd Frontend
```

1. Ejecute el siguiente comando para construir la imagen Docker:

    ```
    docker build -t my-node-app .
    ```

2. Ejecute el siguiente comando para ejecutar el contenedor Docker:

    ```
    docker run -p 3000:3000 my-node-app
    ```

La aplicación ahora debería estar accesible en `http://localhost:3000`.

## Configuraciones de Ejecución para Entorno de Produccción


## Pasos para instalar las dependencias en VS Code

Aquí encontrarás los pasos necesarios para instalar las siguientes dependencias en Visual Studio Code (VS Code) y Pycharm:

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
      pip install Flask-Cors

     ```

Una vez que hayas completado estos pasos, habrás instalado todas las dependencias necesarias en VS Code. Ahora podrás utilizar estas bibliotecas en tus proyectos de Python en el entorno de desarrollo de VS Code.


# Guía de Inicio para Dockerfiles

Este README proporciona instrucciones paso a paso sobre cómo construir y ejecutar contenedores Docker utilizando los Dockerfiles proporcionados.

## Dockerfile para Python FastAPI (uvicorn)

```Dockerfile
# Utiliza una versión oficial de Python como imagen base
FROM python:3.11.4

# Establece el directorio de trabajo en el contenedor a /app
WORKDIR /app

# Añade el contenido del directorio actual en el contenedor en /app
ADD . /app

# Instala los paquetes necesarios especificados en requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Ejecuta setup.py
RUN python setup.py build_ext --inplace

# Hace disponible el puerto 5000 al mundo exterior de este contenedor
EXPOSE 5000

# Define la variable de entorno
ENV NAME World

# Ejecuta uvicorn cuando se lanza el contenedor
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
# Utiliza una imagen base oficial de Node.js
FROM node:18.16.1-buster

# Establece el directorio de trabajo en el contenedor a /app
WORKDIR /app

# Copia package.json y package-lock.json al directorio de trabajo
COPY /package*.json ./

# Instala las dependencias de la aplicación
RUN npm install -g npm@9.7.2 --silent
RUN npm install --silent

# Copia el código de la aplicación al contenedor
COPY . /app

# Expone el puerto en el que se ejecuta la aplicación
EXPOSE 3000

# Inicia la aplicación
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

