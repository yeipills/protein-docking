######################## PROGRAMA 4 ####################

import numpy as np
import csv
import math
import os
import trimesh
import math
#from astropy import units as u
#import Script03_rayos_contexto as S3
import time
# from Script04 import suma_capa
from Script04 import distancia_pto_lista,calcular_modulo_pto,suma_capa,pto_en_esfera
# calcular_modulo_pto
# pto_en_esfera


def getCs(nombreProteina, nombreCrTotales, nombreRayosContexto):
    # s = time.perf_counter()
    # S3.getCR(nombreProteina)
    #dirname = os.path.dirname(__file__)
    #filename = os.path.join(dirname, 'CRSegmentos.txt')
    dirname = os.path.dirname(__file__)
    carpeta_cr = './proteinas_cr/'
    name_cr_totales = carpeta_cr+nombreCrTotales
    name_rayos_contexto = carpeta_cr+nombreRayosContexto
    fname_cr_totales = os.path.join(dirname, name_cr_totales)
    fname_rayos_contexto = os.path.join(dirname, name_rayos_contexto)

    file_cr_totales = open(fname_cr_totales, 'r')
    file_rayos_contexto = open(fname_rayos_contexto, 'r')
    Lines_cr_totales = file_cr_totales.readlines()
    Lines_rayos_context = file_rayos_contexto.readlines()

    ##############FUNCIONES DE APOYO######################

    def calculo_vol(lista_capa, type_cell, number_cs, number_ray, final):
        # start = time.time()
        pto_final = [float(final[0]), float(final[1]), float(final[2])]
        if type_cell == 'in':
            lista_capa.append([number_cs, number_ray, 1, pto_final])
        else:
            lista_capa.append([number_cs, number_ray, 0, pto_final])
        # end =  time.time()
        # cy_time = end - start
        # print("Cython time calculo_vol = {}".format(cy_time))

    def calculo_cs(lista_capa, punto, capa_interna, capa_externa, type_cell, in_out, number_cs, number_ray, final):
        '''
        @param lista_capa: lista donde se irán agregando los ptos
        @param punto: punto actual a evaluar si está dentro o fuera de la capa
        @param capa_interna: puntos de capa interna en el rayo actual
        @param capa_externa: listado puntos capa externa en el rayo actual
        @param tipo de celda: in-out o ses
        @param in_out: indica si se esta calculando un cs interno o externo, in: interno cs ,out: externo cs
        @param final: coordenada final del rayo
        '''

        pto_final = [float(final[0]), float(final[1]), float(final[2])]
        if in_out == 'in':  # cs actual es interno
            ###############EVALUANDO PARA CS INTERNO ###############
            if type_cell == 'in' or type_cell == 'ses':  # ambos sirven para calcular en cs interno
                if len(capa_interna) == 0 and len(capa_externa) == 0:  # no hay bordes de la cs actual
                    # por tanto el pto no está dentro de la capa
                    lista_capa.append([number_cs, number_ray, 0, pto_final])
                elif len(capa_interna) == 0 and len(capa_externa) > 0:
                    # no hay borde interno pero si externo, si el punto tiene menor modulo es 1, sino 0
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_externa = calcular_modulo_pto(capa_externa[0])
                    if modulo_pto <= modulo_externa:  # está dentro de la capa
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:  # no está en la capa
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
                elif len(capa_interna) > 0 and len(capa_externa) == 0:
                    # borde inferior solamente, si el modulo del borde es menor pertenece a cs
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_interna = calcular_modulo_pto(capa_interna[0])
                    if modulo_pto >= modulo_interna:
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
                else:  # hay bordes internos y externos
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_externa = calcular_modulo_pto(capa_externa[0])
                    modulo_interna = calcular_modulo_pto(capa_interna[0])
                    if modulo_interna <= modulo_externa and modulo_interna <= modulo_pto and modulo_externa >= modulo_pto:
                        # está entre la capa interna  y la externa ,1
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:  # no cumple las condiciones, no esta en la capa
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
            else:
                lista_capa.append([number_cs, number_ray, 0, pto_final])

            ############# FIN EVALUACIÓN CS INTERNO ###################
        elif in_out == 'out':
            # INICIO EVALUACIÓN CS EXTERNO
            if type_cell == 'out' or type_cell == 'ses':  # ambos sirven para calcular en cs interno
                if len(capa_interna) == 0 and len(capa_externa) == 0:  # no hay bordes de la cs actual
                    # por tanto el pto no está dentro de la capa
                    lista_capa.append([number_cs, number_ray, 0, pto_final])
                elif len(capa_interna) == 0 and len(capa_externa) > 0:
                    # no hay borde interno pero si externo, si el punto tiene menor modulo es 1, sino 0
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_externa = calcular_modulo_pto(capa_externa[0])
                    if modulo_pto <= modulo_externa:  # está dentro de la capa
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:  # no está en la capa
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
                elif len(capa_interna) > 0 and len(capa_externa) == 0:
                    # borde inferior solamente, si el modulo del borde es menor pertenece a cs
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_interna = calcular_modulo_pto(capa_interna[0])
                    if modulo_pto >= modulo_interna:
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
                else:  # hay bordes internos y externos
                    modulo_pto = calcular_modulo_pto(punto)
                    modulo_externa = calcular_modulo_pto(capa_externa[0])
                    modulo_interna = calcular_modulo_pto(capa_interna[0])
                    if modulo_interna <= modulo_externa and modulo_interna <= modulo_pto and modulo_externa >= modulo_pto:
                        # está entre la capa interna  y la externa ,1
                        lista_capa.append(
                            [number_cs, number_ray, 1, pto_final])
                    else:  # no cumple las condiciones, no esta en la capa
                        lista_capa.append(
                            [number_cs, number_ray, 0, pto_final])
            else:
                lista_capa.append([number_cs, number_ray, 0, pto_final])

            ############# FIN EVALUACIÓN  ###########

####ses ######
    def llenado_context_ses(listado, pto, capa_ses, number_cs, number_ray, type_cell, final):
        p = [float(pto[0]), float(pto[1]), float(pto[2])]
        pto_final = [float(final[0]), float(final[1]), float(final[2])]
        if type_cell == "ses":
            # indices_ses=list()
            if len(capa_ses) != 0:
                dist = distancia_pto_lista(p, capa_ses)
                if dist < 1:
                    listado.append([number_cs, number_ray, 1, pto_final])
                else:
                    listado.append([number_cs, number_ray, 0, pto_final])
            else:
                listado.append([number_cs, number_ray, 0, pto_final])
                # no hay pto ses por lo tanto es un vector de 0s para ese cs
        else:
            listado.append([number_cs, number_ray, 0, pto_final])

    # def calcular_modulo_pto(pto):
    #     s = time.perf_counter()
    #     # print("pto",pto)
    #     # pto=[x,x,x]
    #     x_pto = float(pto[0])
    #     y_pto = float(pto[1])
    #     z_pto = float(pto[2])
    #     # distancia entre ambos puntos debe ser igual al radio del rayo de contexto
    #     d_cuadrada = (x_pto)*(x_pto)+(y_pto)*(y_pto)+(z_pto)*(z_pto)
    #     modulo = math.sqrt(d_cuadrada)
    #     # print("modulo",modulo)
    #     e = time.perf_counter()
    #     print(f'Duration script4: {e-s:.4f}s')
    #     return modulo

# radii=3, centro=[x,x,x], pto[x,x,x]///x_centro=-x , x_pto=-x
    # def pto_en_esfera(radii, centro, pto):
    #     # print("radii",radii)
    #     # print("centro",centro)
    #     # print("pto",pto)
    #     x_centro = centro[0]
    #     # print("x_centro",x_centro)
    #     y_centro = centro[1]
    #     z_centro = centro[2]

    #     x_pto = pto[0]
    #     # print("x_pto",x_pto)
    #     y_pto = pto[1]
    #     z_pto = pto[2]

    #     # distancia entre ambos puntos debe ser igual al radio del rayo de contexto
    #     d_cuadrada = (x_pto-x_centro)*(x_pto-x_centro)+(y_pto-y_centro) * \
    #         (y_pto-y_centro)+(z_pto-z_centro)*(z_pto-z_centro)
    #     if d_cuadrada-(radii*radii) < 0.001:
    #         return True
    #     else:
    #         return False

    # def distancia_pto_lista(pto1, listado_ptos):
    #     # s = time.perf_counter()
    #     # distancia entre un punto y un array de puntos, devuelve el pto del array más cercano a pto 1

    #     x_1 = float(pto1[0])
    #     y_1 = float(pto1[1])
    #     z_1 = float(pto1[2])
    #     listado_d = list()
    #     # cont_distancia_pto_lista=0
    #     for pto2 in listado_ptos:
    #         x_2 = float(pto2[0])
    #         y_2 = float(pto2[1])
    #         z_2 = float(pto2[2])
    #         # d=calcular_distancia_pto_lista(x_1,y_1,z_1,x_2,y_2,z_2)
    #         d2 = (x_2-x_1)*(x_2-x_1)+(y_2-y_1)*(y_2-y_1)+(z_2-z_1)*(z_2-z_1)
    #         d = math.sqrt(d2)
    #         listado_d.append(d)
    #         # cont_distancia_pto_lista=cont_distancia_pto_lista+1
    #     listado_sort = sorted(listado_d)
    #     # e = time.perf_counter()
    #     # print(f'python time distancia_pto_lista: {e-s:.10f}s')
    #     return listado_sort[0]

    # def suma_capa(pto, dist):
    #     # s = time.perf_counter()
    #     # para sumar una distancia de modulo 1
    #     modulo = math.sqrt((pto[0]*pto[0]+pto[1]*pto[1]+pto[2]*pto[2]))
    #     # sumandole 1 al punto para obtener el in1
    #     x = pto[0]+dist*(pto[0]/modulo)
    #     y = pto[1]+dist*(pto[1]/modulo)
    #     z = pto[2]+dist*(pto[2]/modulo)
    #     # e = time.perf_counter()
    #     # print(f'python time suma_capa: {e-s:.10f}s')
    #     return [x, y, z]



    ###############FIN FUNCIONES DE APOYO #################
    # Retorna el índice de la forma de contexto y todos los inicios y fin de cada rayo
    rayos_contexto = list()
    # coordenadas_rayos_contexto=list()
    cont_suma_capa = 0
    n = 16
    for line in Lines_rayos_context:
        a = line.split(" ")
        number_cr = int(a[0])
        x_origin = float(a[1])
        y_origin = float(a[2])
        z_origin = float(a[3])
        pto_origin = [x_origin, y_origin, z_origin]
        x_end = float(a[4])
        y_end = float(a[5])
        z_end = float(a[6].replace('\n', ''))
        pto_end = [x_end, y_end, z_end]
        vector = np.linspace(pto_origin, pto_end, n)
        rayos_contexto.append([number_cr, cont_suma_capa, vector])
        cont_suma_capa = cont_suma_capa+1

    # print("rayo",rayos_contexto[0][2][0])
    # rayos_contexto=>[0,0, array de largo n]
    ##############
    # Evaluar cada fila, obtenter coordenada del SES si es que está,
    # luego al SES Sumar  y restar los modulos v/|v| para obtener las límites de las  capas, evaluando que no pase el límite

    SES_points = list()
    boleanos = list()
    coordenadas = list()
    number_cr = list()
    number_ray = list()
    contador_ray = 0
    for line in Lines_cr_totales:
        b = line.split(" ")
        number = int(b[0])
        cols1 = 4
        # para separar true y coordenadas total-4=3x+x-1 x=total-3/4 + cols1
        div = (len(b)-3)/4
        cols2 = int(3*div)+cols1
        coor = np.array(b[cols1:cols2])
        coordenadas.append(coor.reshape(int(len(coor)/3), 3))
        bol = ['True' if x == 'True\n' else x for x in b[cols2+1:len(b)]]
        bol = ['False' if x == 'False\n' else x for x in bol]
        boleanos.append(bol)
        number_cr.append(number)
        number_ray.append(contador_ray)
        contador_ray = contador_ray+1
    # print(prueba)

    # count=0
    pattern1 = list(['True', 'False'])
    pattern2 = list(['False', 'True'])
    SES_points = []
    # prueba=list()
    for i in range(len(boleanos)):
        for j in range(len(boleanos[0])):
            ev1 = boleanos[i][j] == pattern1[0] and boleanos[i][j:j +
                                                                len(pattern1)] == pattern1
            ev2 = boleanos[i][j] == pattern2[0] and boleanos[i][j:j +
                                                                len(pattern2)] == pattern2
            if ev1 or ev2:
                SES_points.append([number_cr[i], "ses", coordenadas[i][0], coordenadas[i][len(
                    coordenadas[0])-1], coordenadas[i][j], coordenadas[i][j+1], number_ray[i]])
            else:
                if boleanos[i][j] == "True":
                    # type_cell="in"
                    SES_points.append([number_cr[i], "in", coordenadas[i][0], coordenadas[i][len(
                        coordenadas[0])-1], coordenadas[i][j], coordenadas[i][j+1], number_ray[i]])

                else:
                    # type_cell="out"
                    SES_points.append([number_cr[i], "out", coordenadas[i][0], coordenadas[i][len(
                        coordenadas[0])-1], coordenadas[i][j], coordenadas[i][j+1], number_ray[i]])

    # [0][0]=>inicio rayo original [0][1]=>fin rayo original [0][2]=>inicio rayo ses [0][3]=>fin rayo ses

    # radii=3

    cs_in1 = list()
    cs_in2 = list()
    cs_in3 = list()
    cs_in4 = list()
    cs_out1 = list()
    cs_out2 = list()
    cs_out3 = list()
    cs_out4 = list()
    cs_ses = list()
    # #determinar ses, para eso se evalúan solo los puntos que son de tipo ses
    # print(SES_points[0])
    # print(coordenadas[0])
    # print("SES_points000",len(SES_points[0]))
    # print("SES_points111",SES_points[1])
    # ---------------------implementacion.-----------
    # if(len(SES_points)>0):
    #     # print("llego")
    #     # hola(6)
    #     # s = time.perf_counter()
    #     start = time.time()
    #     determinar_ses(SES_points,cs_in1,cs_in2,cs_in3,cs_in4,cs_out1,cs_out2,cs_out3,cs_out4,cs_ses)
    #     end =  time.time()
    #     cy_time = end - start
    #     print("Cython time determinar_ses = {}".format(cy_time))
    # print("llego_fin")
    # e = time.perf_counter()
    # print(f'Duration function script4: {e-s:.4f}s')
    # print("se termino")
    # -------------------------------------------------------------------------------------
    # def returnSes(SES_points):
    #     mapCreate=determinar_ses(SES_points)
    #     print("llego")
    #     return SES_points

    # print("SES_points",SES_points)
    cont_pto_tipo_ses = 0
    # s = time.perf_counter()

    for item in SES_points:

        # print("item",item)
        # print("item0",item[0])
        # print("item1",item[1])
        # print("number_ray",item[len(item)-1])
        print("cont_pto_tipo_ses", cont_pto_tipo_ses)
        cont_pto_tipo_ses = cont_pto_tipo_ses+1
        # print("item",item)
        type_cell = item[1]
        number_ray = item[len(item)-1]
        if type_cell == "ses":
            # para ver si está dentro de la recta y no sale del límite
            origen = [float(item[2][0]), float(item[2][1]), float(item[2][2])]
            # final=[float(item[3][0]),float(item[3][1]),float(item[3][2])]
            # origen=>centro de la esfera, final=>fin del rayo
            inicio = [float(item[4][0]), float(item[4][1]), float(
                item[4][2])]  # para calcular las capas
            # fin=[float(item[5][0]),float(item[5][1]),float(item[5][2])]
            # inicio=>inicio del segmento
            # fin=>fin del segmento
            # ses
            # xmean=(inicio[0]+fin[0])/2
            # ymean=(inicio[1]+fin[1])/2
            # zmean=(inicio[2]+fin[2])/2
            # ptos_ses=[xmean,ymean,zmean]
            ptos_ses = [inicio[0], inicio[1], inicio[2]]
            # s = time.perf_counter()
            # restandole 1 al punto para obtener el in1
            ptos_in1 = suma_capa(ptos_ses, -0.2)
            # restandole 1 al punto para obtener el in2
            ptos_in2 = suma_capa(ptos_ses, -0.4)
            ptos_in3 = suma_capa(ptos_ses, -0.8)
            ptos_in4 = suma_capa(ptos_ses, -1)
            # out1
            ptos_out1 = suma_capa(ptos_ses, 0.2)
            # out2
            ptos_out2 = suma_capa(ptos_ses, 0.4)
            # out3
            ptos_out3 = suma_capa(ptos_ses, 0.8)

            ptos_out4 = suma_capa(ptos_ses, 1)
            # e = time.perf_counter()
            # print(f'Duration function suma_capa: {e-s:.10f}s')
            # Para determinar si el pto de la capa calculada cae en la esfera se debe calcular la distancia
            r = 3
            # s = time.perf_counter()
            d_in1 = pto_en_esfera(r, origen, ptos_in1)
            d_in2 = pto_en_esfera(r, origen, ptos_in2)
            d_in3 = pto_en_esfera(r, origen, ptos_in3)
            d_in4 = pto_en_esfera(r, origen, ptos_in4)
            d_out1 = pto_en_esfera(r, origen, ptos_out1)
            d_out2 = pto_en_esfera(r, origen, ptos_out2)
            d_out3 = pto_en_esfera(r, origen, ptos_out3)
            d_out4 = pto_en_esfera(r, origen, ptos_out4)
            d_ses = pto_en_esfera(r, origen, ptos_ses)
            # e = time.perf_counter()
            # print(f'Duration function suma_capa: {e-s:.10f}s')

            if d_in1:
                cs_in1.append([item[0], number_ray, ptos_in1])

            if d_in2:
                cs_in2.append([item[0], number_ray, ptos_in2])

            if d_in3:
                cs_in3.append([item[0], number_ray, ptos_in3])

            if d_in4:
                cs_in4.append([item[0], number_ray, ptos_in4])

            if d_out1:
                cs_out1.append([item[0], number_ray, ptos_out1])

            if d_out2:
                cs_out2.append([item[0], number_ray, ptos_out2])

            if d_out3:
                cs_out3.append([item[0], number_ray, ptos_out3])

            if d_out4 < 1:
                cs_out4.append([item[0], number_ray, ptos_out4])

            if d_ses:
                cs_ses.append([item[0], number_ray, ptos_ses])
    # end =  time.time()
    # cy_time = end - start
    # print("Cython time determinar_ses = {}".format(cy_time))
    # e = time.perf_counter()
    # print(f'Duration function script4: {e-s:.4f}s')
    # print("se termino")

    n = 16  # cantidad de divisiones, en el paper usan 32

    ############################ CÁLCULO CONTEXT SHAPE IN2 #############################################
    cs_in1_final = list()
    cs_in2_final = list()
    cs_in3_final = list()
    cs_in4_final = list()
    cs_out1_final = list()
    cs_out2_final = list()
    cs_out3_final = list()
    cs_out4_final = list()
    cs_ses_final = list()
    cs_vol_final = list()

    cont = 0
    for item in SES_points:
        # print("item[4]",item[4])
        print("rayo", cont)
        # print("item[4]",item[4])
        cont = cont+1
        number_cs = item[0]
        number_ray = item[6]
        type_cell = item[1]
        ses = [i[2] for i in cs_ses if i[1] == number_ray]
        in1 = [i[2] for i in cs_in1 if i[1] == number_ray]
        in2 = [i[2] for i in cs_in2 if i[1] == number_ray]
        in3 = [i[2] for i in cs_in3 if i[1] == number_ray]
        in4 = [i[2] for i in cs_in4 if i[1] == number_ray]
        out1 = [i[2] for i in cs_out1 if i[1] == number_ray]
        out2 = [i[2] for i in cs_out2 if i[1] == number_ray]
        out3 = [i[2] for i in cs_out3 if i[1] == number_ray]
        out4 = [i[2] for i in cs_out4 if i[1] == number_ray]
        llenado_context_ses(
            cs_ses_final, item[4], ses, number_cs, number_ray, type_cell, item[3])
        # cs interno => capa superior es capa k-1, capa inferior es k
        calculo_vol(cs_vol_final, type_cell, number_cs, number_ray, item[3])
        calculo_cs(cs_in1_final, item[4], in1, ses,
                   type_cell, "in", number_cs, number_ray, item[3])
        calculo_cs(cs_in2_final, item[4], in2, in1,
                   type_cell, "in", number_cs, number_ray, item[3])
        calculo_cs(cs_in3_final, item[4], in3, in2,
                   type_cell, "in", number_cs, number_ray, item[3])
        calculo_cs(cs_in4_final, item[4], in4, in3,
                   type_cell, "in", number_cs, number_ray, item[3])
        # cs externo => capa superior es capa k, capa inferior es k-1
        calculo_cs(cs_out1_final, item[4], ses, out1,
                   type_cell, "out", number_cs, number_ray, item[3])
        calculo_cs(cs_out2_final, item[4], out1, out2,
                   type_cell, "out", number_cs, number_ray, item[3])
        calculo_cs(cs_out3_final, item[4], out2, out3,
                   type_cell, "out", number_cs, number_ray, item[3])
        calculo_cs(cs_out4_final, item[4], out3, out4,
                   type_cell, "out", number_cs, number_ray, item[3])
    print("fin evaluacion")
    # e = time.perf_counter()
    # print(f'Duration script4: {e-s:.4f}s')

    def escribir_archivo(nombre, lista):
        with open(nombre, 'w') as f:
            for item in lista:
                string1 = str(item[0])
                string2 = str(item[1])
                string3 = str(item[2])
                string4 = " ".join([str(elem) for elem in item[3]])
                string_total = string1+" "+string2+" "+string3+" "+string4
                # [number_cs,number_ray,0,pto_final]
                f.write("%s\n" % string_total)

    carpeta_cs = str('./proteinas_cs/')
    name_cs = carpeta_cs+str(nombreProteina)
    fname_ses = os.path.join(dirname, name_cs+'_ses.txt')
    fname_in1 = os.path.join(dirname, name_cs+'_in1.txt')
    fname_in2 = os.path.join(dirname, name_cs+'_in2.txt')
    fname_in3 = os.path.join(dirname, name_cs+'_in3.txt')
    fname_in4 = os.path.join(dirname, name_cs+'_in4.txt')
    fname_out1 = os.path.join(dirname, name_cs+'_out1.txt')
    fname_out2 = os.path.join(dirname, name_cs+'_out2.txt')
    fname_out3 = os.path.join(dirname, name_cs+'_out3.txt')
    fname_out4 = os.path.join(dirname, name_cs+'_out4.txt')
    fname_vol = os.path.join(dirname, name_cs+'_vol.txt')

    escribir_archivo(fname_ses, cs_ses_final)
    escribir_archivo(fname_in1, cs_in1_final)
    escribir_archivo(fname_in2, cs_in2_final)
    escribir_archivo(fname_in3, cs_in3_final)
    escribir_archivo(fname_in4, cs_in4_final)

    escribir_archivo(fname_out1, cs_out1_final)
    escribir_archivo(fname_out2, cs_out2_final)
    escribir_archivo(fname_out3, cs_out3_final)
    escribir_archivo(fname_out4, cs_out4_final)

    escribir_archivo(fname_vol, cs_vol_final)
    print("fin escritura cs")


# se demora una hora en correr todo
    # e = time.perf_counter()
    # print(f'Duration script4: {e-s:.4f}s')
# nombreProteina='1AHW_l_u'
# 1AHW_r_u
# nombreProteina='1AHW_r_u'
# nombreProteina = '1AHW_l_u'
# getCs(nombreProteina)
