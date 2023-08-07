######################## PROGRAMA 2 ###################
# Tiene las funciones que retornan los valores de MSMS
import Script01_lectura_caras_vertices as S1
import time


def getCenter(nombreVertices, nombreCaras):
    s = time.perf_counter()
    # Retorno de array con vértices y caras de las proteínas
    arr_vert, arr_face = S1.definirProteina(nombreVertices, nombreCaras)
    # print("arr_vert",arr_vert)

    # matriz que guarda las coordenadas de los vértices
    vertices = [[0]*3]*len(arr_vert)
    # print("vertices",vertices)
    # Guardando sólo las coordenadas de los vértices
    for i in range(0, len(arr_vert)):
        fila = arr_vert[i]
        # Se guardan las coordenas x,y,z de cada vértice
        vertices[i] = [float(fila[1]), float(fila[2]), float(fila[3])]
        # viendo si normalizadas sirven
        # vertices[i]=[float(fila[4]),float(fila[5]),float(fila[6])]

    centroides = list()  # donde se almacenarán las coordenadas calculadas de los centroides
    centros = list()

    for i in range(len(arr_face)):
        fila = arr_face[i]
        type_face = float(fila[4])
        if type_face != 1:
            # solo se calculan centroides a parches de tipo cara reentrante o de contacto
            # indices vertices
            indvert1 = int(fila[1])
            indvert2 = int(fila[2])
            indvert3 = int(fila[3])
            # vertices
            vertice1 = vertices[indvert1]
            vertice2 = vertices[indvert2]
            vertice3 = vertices[indvert3]

            centroX = (float(vertice1[0]) +
                       float(vertice2[0])+float(vertice3[0]))/3
            centroY = (float(vertice1[1]) +
                       float(vertice2[1])+float(vertice3[1]))/3
            centroZ = (float(vertice1[2]) +
                       float(vertice2[2])+float(vertice3[2]))/3
            centroides.append(str(centroX)+" "+str(centroY)+" "+str(centroZ))
            centros.append([centroX, centroY, centroZ])

    # Exportación de las coordenadas de los centroides para usarlos en unity

    with open('centroidesPCS.txt', 'w') as f:
        for item in centroides:
            #f.write("\n ".join(item))
            f.write("%s\n" % item)

    print('Centroides exportados correctamente')
    e = time.perf_counter()
    print(f'Duration script 2: {e-s:.4f}s')
    # print("centros",centros)
    return centros


# def getCenter():
#     return centros
