###############PROGRAMA 3 ################################################
import numpy as np
import trimesh
import os
# contiene las coordenadas de los puntos críticos
import Script02_calculo_centroides as S2
import math
from scipy.spatial import cKDTree
# from astropy import units as u #unidades astronómicas para usar angstrom
import time
# from Script03 import computeCR


def getCR(nombreProteina, nombreVertices, nombreCaras):
    s = time.perf_counter()
############# Lectura archivo stl con la superficie de la proteína #############
    dirname = os.path.dirname(__file__)
    carpeta_stl = './proteinas_stl/'
    archivo_stl = carpeta_stl+str(nombreProteina)+'.stl'
    filename = os.path.join(dirname, archivo_stl)

    # Convirtiendo el archivo stl a malla
    mesh_SES = trimesh.exchange.load.load(filename)

    #mesh, info = pymesh.remove_isolated_vertices(mesh_SES)

    # Obteniendo los centros de los puntos de superficie
    centros_esferas = S2.getCenter(nombreVertices, nombreCaras)
    # with open('centros_ptoSuperficie.txt', 'w') as f:
    #      for item in centros_esferas:
    #          #f.write("\n ".join(item))
    #          f.write("%s\n" % item)
    #print (centros_esferas)
    # Para disminuir puntos de superficie se eliminar los que estén a una de distancia >=5
    maxdis = 10
    cp = list()
    # start = time.time()
    for item in centros_esferas:
        # print ("item",item)
        num = int(item[0])
        if num != 1000:
            kd = cKDTree(centros_esferas)
            dist, points = kd.query(item, k=100)
            # Posiciones están a una distancia menor al máximo permitido
            pos_dist = [i for i, n in enumerate(dist) if n < maxdis]
            # print ("pos_dist",pos_dist)
            # Posiciones de los puntos en la lista original
            pos_points = [points[i] for i in pos_dist]
            # print ("pos_points",pos_points)
            cp.append(centros_esferas[pos_points[0]])
            for i in pos_points:
                # Para mantener las dimensiones del array se reemplazan por un valor muy alto
                centros_esferas[i] = [1000, 1000, 1000]
    # end =  time.time()
    # cy_time = end - start
    # print("Cython time computeCR = {}".format(cy_time))

    # ------------------------------------------------------------------
    # with open('filtro_centros_ptoSuperficie.txt', 'w') as f:
         # for item in cp:
             #f.write("\n ".join(item))
             #f.write("%s\n" % item)
     # ------------------------------------------------------------------
    # print(cp)
      # origins,ends,cr,rayos_contexto=computeCR(3,10,cp)
    # start = time.time()
    # cr,rayos_contexto=computeCR(3,10,cp)
    # end =  time.time()
    # cy_time = end - start
    # print("Cython time computeCR = {}".format(cy_time))
    # print(cr)

  # Calculando CR para todos los puntos criticos
    def computeCR(radii, delta, centers):
        # print("delta",delta)
        # radii:radio de la esfera de los CR
        # delta: cantidad a generar
        # centers: centros de puntos críticos
        # n: division de cada rayo
        # Angulos usados para muestrear la esfera de forma equidistante
        # Tamaño 2pi*10^2=>1256->36^2 =>1296, 35^2=>1225
        phi = np.linspace(0, 2*math.pi, delta)
        # print("phi",phi)
        theta = np.linspace(0, 2*math.pi, delta)
        # print("theta",theta)
        # Lista con el inicio y fin de cada rayo y el índice de la esfera a la que corresponde
        rayos_contexto = list()

        cr = list()
        # Contador es el índice de la esfera
        counter = 0
        #print("puto 1")
        #s = time.perf_counter()
        for item in centers:
            #s = time.perf_counter()
            # Inicio de los rayos, todos se originan en el centro de la esfera correspondiente
            ray_origins_item = list()

            # Fin de los rayos, dependerá de los ángulos de muestreo
            ray_directions_item = list()

            for i in phi:
                for j in theta:
                    # Se calcula el término del rayo y se le suma el centro ya que no está en el origen 0,0,0
                    # MALO__ x= r cos phi sin the
                    x = radii*math.sin(i)*math.cos(j)+item[0]

                    y = radii*math.sin(i)*math.sin(j)+item[1]
                    z = radii*math.cos(i)+item[2]

                    counter = counter+1
                    # print("counter",counter)
                    ray_directions_item.append([x, y, z])

                    ray_origins_item.append(item)

                    # Conversión de las coordenadas a string para guardarlas en un archivo de texto
                    final = str(x)+" "+str(y)+" "+str(z)
                    origen = str(item[0])+" "+str(item[1])+" "+str(item[2])

                    # Agregando el rayo a la lista
                    rayos_contexto.append(str(counter)+" "+origen+" "+final)
                    # print("rayos_contexto",rayos_contexto)

            # ray_directions.append(ray_directions_item)
            # ray_origins.append(ray_origins_item)

            # Otra manera de guardar los rayos de contexto
            cr.append([counter, item, ray_directions_item])
            counter = counter+1
        print("fin calculo CR")
        # print(ray_origins_item)
        #print(f'Duration fin calculo CR: {e-s:.4f}s')
        # return ray_origins,ray_directions,cr,rayos_contexto
        return cr, rayos_contexto
        print(f'Duration fin calculo CR: {e-s:.4f}s')

        # -----------------------------------------------------------------------------------

    # def computeCR(radii, delta, centers):
    #     phi = math.pi * (3. - math.sqrt(5.))  # golden angle in radian
    #     rayos_contexto = list()

    #     cr = list()
    #     counter = 0
    #     for item in centers:
    #         ray_origins_item = list()
    #         ray_directions_item = list()

    #         for i in phi:
    #             theta = phi * i  # golden angle increment
    #             for j in theta:
    #                 y = 1 - (i/int(delta - 1)) * 2 + \
    #                     item[1]  # y goes from 1 to -1
    #                 x = radii*math.cos(j)+item[0]
    #                 z = radii*math.sin(j)+item[2]
    #                 # x=radii*math.sin(i)*math.cos(j)+item[0]#MALO__ x= r cos phi sin the

    #                 # y=radii*math.sin(i)*math.sin(j)+item[1]
    #                 # z=radii*math.cos(i)+item[2]
    #                 counter = counter+1
    #                 ray_directions_item.append([x, y, z])
    #                 ray_origins_item.append(item)
    #                 final = str(x)+" "+str(y)+" "+str(z)
    #                 origen = str(item[0])+" "+str(item[1])+" "+str(item[2])
    #                 rayos_contexto.append(str(counter)+" "+origen+" "+final)
    #         cr.append([counter, item, ray_directions_item])
    #         counter = counter+1
    #     print("fin calculo CR")
    #     return cr, rayos_contexto


# origins,ends,cr,rayos_contexto=computeCR(3,10,cp)
    start = time.time()
    cr, rayos_contexto = computeCR(3, 10, cp)
    end = time.time()
    cy_time = end - start
    print("Python time computeCR = {}".format(cy_time))
    # print(cr)
    n = 17
    count = 0
    # count2=0
    print("puto")
    cr_segments = list()
    # cr_seg=list()
    for item in cr:
        # item[0]=indice, item[1]=coord punto crítico, item[2] coordenada fin de cr
        # origen del rayo, que es el centro de la esfera/pto critico
        origen = item[1]
        finales = item[2]  # vector con las coordenadas del fin de cada rayo
        # print("item",item[0])
        # print("item1",item[1])
        # print("item2",item[2])
        print("count", count)
        count = count+1
        for rayo in finales:
            # print("rayo",rayo)-- son 100 rayos
            # count2=count2+1
            # print("count2",count2)
            # Evaluando cada rayo de la esfera, todos empiezan con el mismo origen
            # Lo que cambia es el fin del rayo
            # ----- numpy.linspace(valor-inicial, valor-final, número de valores)-------
            # Vector para dividir el rayo en segmentos
            vector = np.linspace(origen, rayo, n)
            # print("vector",vector)
            # hits=mesh_SES.contains(vector) #2069.456 segundos 34,5 minutos
            # Se evalua si cada segmento intersecta o no la esfera
            # Si no la intersecta está fuera de la esfera
            # Si lo intersecta esta dentro
            hits = mesh_SES.ray.intersects_any(ray_origins=vector[0:len(vector)-1],
                                               ray_directions=vector[1:len(vector)])  # 212,95 segundos 4 minutos
            # print("hits",hits)
            string1 = str(item[0])
            # print("string1",string1)
            string2 = " ".join([str(elem) for elem in item[1]])
            # print("string2",string2)
            string3 = " ".join(
                [str(elem[0])+" "+str(elem[1])+" "+str(elem[2]) for elem in vector])
            # print("string3",string3)
            string4 = " ".join([str(elem) for elem in hits])
            # print("string4",string4)
            cr_segments.append(string1+" "+string2+" "+string3+" "+string4)
            # print("cr_segments",cr_segments)
    # print("Fin evaluarcion segmentos")

    # Definición ubicación y nombre de los archivos de rayos
    carpeta_cr = './proteinas_cr/'
    name_cr_totales = carpeta_cr+str(nombreProteina)+str('_cr_totales.txt')
    name_rayos_contexto = carpeta_cr + \
        str(nombreProteina)+str('_rayos_contexto.txt')
    filename_cr_totales = os.path.join(dirname, name_cr_totales)
    filename_rayos_contexto = os.path.join(dirname, name_rayos_contexto)

    with open(filename_cr_totales, 'w') as f:
        for item in cr_segments:
            f.write("%s\n" % item)

    with open(filename_rayos_contexto, 'w') as f:
        for item in rayos_contexto:
            f.write("%s\n" % item)

    print("Rayos de contexto guardados correctamente")
    e = time.perf_counter()
    print(f'Duration script 3: {e-s:.4f}s')
# nombreProteina='1AHW_l_u'
# getCR(nombreProteina)
# Para asignarle un color a la malla, luego comentar
#factor, origin, direction = trimesh.transformations.scale_from_matrix(S0)
# mesh_2=mesh_SES.apply_transform(scale=5)
#v1= trimesh.transformations.scale_matrix(1.5)
# v0= trimesh.transformations.translation_matrix([0, 0, 0])
# T=trimesh.transformations.translation_from_matrix(v0)
# S=trimesh.transformations.scale_from_matrix(v1)
# M0 = trimesh.transformations.concatenate_matrices(S0,v0)
# M1=trimesh.transformations.scale_and_translate(scale=4)
#median2 = np.mean(mesh_SES2.bounds[:, :], axis=0)
# mesh_SES2.apply_translation(-1*mesh_SES2.centroid)
# mesh_SES2.T=trimesh.transformations.translation_from_matrix(v0)
#max_length_SES2 = np.max(np.abs(mesh_SES2.bounds[:, :]))
#length_SES2 = 1.1/ (max_length_SES2* 2.0)
#r2 = np.max(np.linalg.norm(mesh_SES2.vertices, axis=-1))
# mesh_SES2.apply_scale(1.7)


# mesh2=mesh_SES2.apply_transform(M0)
# mesh_SES.visual.face_colors = [10, 10, 10]
# mesh_SES2.visual.face_colors= [200, 200, 250, 100]
# scene = trimesh.Scene([mesh_SES,
#                        mesh_SES2])
# scene.show()
