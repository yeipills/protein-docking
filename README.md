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
     pip install fastapi
     ```

Una vez que hayas completado estos pasos, habrás instalado todas las dependencias necesarias en VS Code. Ahora podrás utilizar estas bibliotecas en tus proyectos de Python en el entorno de desarrollo de VS Code.
