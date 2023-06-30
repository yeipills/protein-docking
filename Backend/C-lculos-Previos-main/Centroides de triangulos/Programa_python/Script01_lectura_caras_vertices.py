############################ PROGRAMA 1 #####################################
# Programa que lee los archivos de MSMS de las caras y vértices de los parches
# Retorna los parámetros en 2 matrices
import os
import re
import numpy as np
import time

# Path relativo de los archivos
dirname = os.path.dirname(__file__)


def definirProteina(nombreVertices, nombreCaras):
    s = time.perf_counter()
    # carpeta donde se encuentran los archivos .vert y .face del programa MSMS
    carpeta = './proteinas/'
    archivo_vert = carpeta+str(nombreVertices)  # archivo de los vértices
    archivo_face = carpeta+str(nombreCaras)  # archivo de las caras
    # obteniendo el path completo
    filename_vert = os.path.join(dirname, archivo_vert)
    filename_face = os.path.join(dirname, archivo_face)

    # Lee y almacena vértices en array
    f_vert = open(filename_vert, "r")
    f1_vert = f_vert.readlines()

    rows, cols = (len(f1_vert)-3, 11)
    arr_vert = [['']*cols]*rows  # tamaño de la matriz de los vértices

    count_vert = 0
    index_vert = 0
    # Parseando los datos y eliminando el encabezado
    for i in f1_vert:
        if count_vert > 2:
            # separacion de los valores por los espacios en blancos
            split_line = re.split(" +", i)
            arr_vert[index_vert] = split_line
            index_vert = index_vert+1
        count_vert = count_vert+1

    # Leer y guardar en variable las caras
    f_face = open(filename_face, "r")
    f1_face = f_face.readlines()
    rows2, cols2 = (len(f1_face)-3, 6)
    arr_face = [['']*cols2]*rows2

    count_face = 0
    index_face = 0
    for j in f1_face:
        if count_face > 2:
            split_line = re.split(" +", j)
            arr_face[index_face] = split_line
            index_face = index_face+1
        count_face = count_face+1

    print("Variables MSMS obtenidas correctamente")
    e = time.perf_counter()
    print(f'Duration script 1: {e-s:.4f}s')

    # print("arr_vert",arr_vert)
    # print("arr_face",arr_face)
    # return arr_vert,arr_face


# # guardar en txt
    # with open("lectura_caras.txt", "w") as txt_file:
    # for line in arr_face:
    # txt_file.write(" ".join(line)) # works with any number of elements in a lin

    # with open("lectura_vertices.txt", "w") as txt_file:
    # for line in arr_vert:
    # txt_file.write(" ".join(line)) # works with any number of elements in a lin

    return arr_vert, arr_face
