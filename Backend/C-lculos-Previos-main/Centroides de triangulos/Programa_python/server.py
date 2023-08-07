from fastapi import FastAPI, File, UploadFile, BackgroundTasks, Form
from fastapi.responses import JSONResponse, FileResponse
from fastapi.middleware.cors import CORSMiddleware
import shutil
import aiofiles
import requests
import os


from Script03_rayos_contexto import getCR
from Script04_evaluacion_capas import getCs

app = FastAPI()

app.socketServer = "http://localhost:5000"

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/")
def read_root():
    return {"Backend": "Proyecto titulo"}


@app.post("/cargarParteUno")
async def create_upload_files(background_tasks: BackgroundTasks,
                              proteinaStl: UploadFile = File(...),
                              proteinaVertices: UploadFile = File(...),
                              proteinaCaras: UploadFile = File(...),
                              ):
    try:
        os.makedirs("./proteinas_stl", exist_ok=True)
        os.makedirs("./proteinas", exist_ok=True)

        proteinaStlPath = os.path.join("./proteinas_stl", proteinaStl.filename)
        proteinaVerticesPath = os.path.join("./proteinas", proteinaVertices.filename)
        proteinaCarasPath = os.path.join("./proteinas", proteinaCaras.filename)

        async with aiofiles.open(proteinaStlPath, 'wb') as out_file:
            while content := await proteinaStl.read(1024):
                await out_file.write(content)

        async with aiofiles.open(proteinaVerticesPath, 'wb') as out_file:
            while content := await proteinaVertices.read(1024):
                await out_file.write(content)

        async with aiofiles.open(proteinaCarasPath, 'wb') as out_file:
            while content := await proteinaCaras.read(1024):
                await out_file.write(content)

        background_tasks.add_task(
            runScript3, proteinaStl=proteinaStl.filename.split(".")[0],
            nombreVertices=proteinaVertices.filename,
            nombreCaras=proteinaCaras.filename
        )

        return JSONResponse(content={
            "Result": "OK"
        }, status_code=200)

    except IOError as ex:

        return JSONResponse(content={
            "Result": "ERROR", "error":  "Error al cargar el archivo %s" % ex
        }, status_code=409)


@app.post("/cargarParteDos")
async def create_upload_files(background_tasks: BackgroundTasks,
                              nombreProteina=Form(...),
                              crTotales: UploadFile = File(...),
                              rayosContexto: UploadFile = File(...),
                              ):
    try:
        os.makedirs("./proteinas_cr", exist_ok=True)

        crTotalesPath = os.path.join("./proteinas_cr", crTotales.filename)
        rayosContextoPath = os.path.join("./proteinas_cr", rayosContexto.filename)

        async with aiofiles.open(crTotalesPath, 'wb') as out_file:
            while content := await crTotales.read(1024):
                await out_file.write(content)

        async with aiofiles.open(rayosContextoPath, 'wb') as out_file:
            while content := await rayosContexto.read(1024):
                await out_file.write(content)

        background_tasks.add_task(
            runScript4, nombreProteina=nombreProteina,
            crTotales=crTotales.filename,
            rayosContexto=rayosContexto.filename)

        return JSONResponse(content={
            "Result": "OK"
        }, status_code=200)

    except IOError as ex:

        return JSONResponse(content={
            "Result": "ERROR", "error":  "Error al cargar el archivo %s" % ex
        }, status_code=409)


@app.get("/descargaParteUno")
async def descarga_parte_uno(background_tasks: BackgroundTasks):
    zipDir = shutil.make_archive(
        base_name=os.path.join(".", "resultadosParteUno"), format="zip", root_dir=os.path.join(".", "proteinas_cr"))
    background_tasks.add_task(deleteZipResults, zipName=zipDir)
    return FileResponse(os.path.join(".", "resultadosParteUno.zip"), media_type='application/octet-stream', filename="resultadosParteUno.zip")


@app.get("/descargaParteDos")
async def descarga_parte_dos(background_tasks: BackgroundTasks):
    zipDir = shutil.make_archive(
        base_name=os.path.join(".", "resultadosParteDos"), format="zip", root_dir=os.path.join(".", "proteinas_cs_unity"))
    background_tasks.add_task(deleteZipResults, zipName=zipDir)
    return FileResponse(os.path.join(".", "resultadosParteDos.zip"), media_type='application/octet-stream', filename="resultadosParteDos.zip")


def runScript3(proteinaStl: str, nombreVertices: str, nombreCaras: str):
    getCR(proteinaStl, nombreVertices, nombreCaras)
    print("Termina parte uno y emite")
    requests.get(url="%s/terminaParteUno" % app.socketServer)


def runScript4(nombreProteina: str, crTotales: str, rayosContexto: str):
    getCs(nombreProteina, crTotales, rayosContexto)
    print("Termina parte dos y emite")
    requests.get(url="%s/terminaParteDos" % app.socketServer)


def deleteZipResults(zipName=""):
    os.unlink(zipName)
