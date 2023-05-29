
cimport cython
#import numpy as np
#cimport numpy as np
from libc.math cimport  sqrt
import numpy as np
import time


cpdef distancia_pto_lista(pto1,listado_ptos):
        #s = time.perf_counter()
    #distancia entre un punto y un array de puntos, devuelve el pto del array más cercano a pto 1
        cdef double x_1,y_1,z_1,x_2,y_2,z_2,d
        cdef list pto2
        listado_sort=[]
        listado_d=[]

        x_1=float(pto1[0])
        y_1=float(pto1[1])
        z_1=float(pto1[2])

        for pto2 in listado_ptos:
            x_2=float(pto2[0])
            y_2=float(pto2[1])
            z_2=float(pto2[2])
            d2=(x_2-x_1)*(x_2-x_1)+(y_2-y_1)*(y_2-y_1)+(z_2-z_1)*(z_2-z_1)
            d=sqrt(d2)
            listado_d.append(d)
            
        listado_sort=sorted(listado_d)
        #e = time.perf_counter()
        #print(f'python time distancia_pto_lista: {e-s:.10f}s') 
        return listado_sort[0]


#cpdef float calcular_distancia_pto_lista( double x_2, double y_2, double z_2 ,double x_1,double y_1,double z_1):
    #cdef double d2,dr
    #d2=(x_2-x_1)*(x_2-x_1)+(y_2-y_1)*(y_2-y_1)+(z_2-z_1)*(z_2-z_1)
    #dr=sqrt(d2)
    #return dr

@cython.boundscheck(False)
cpdef calcular_modulo_pto(pto):
    #start = time.time() 
    #s = time.perf_counter()
    cdef double  modulo,d_cuadrada,x_pto,y_pto,z_pto
    x_pto=float(pto[0])
    y_pto=float(pto[1])
    z_pto=float(pto[2])
    d_cuadrada=(x_pto)*(x_pto)+(y_pto)*(y_pto)+(z_pto)*(z_pto)
    modulo=sqrt(d_cuadrada)
    e = time.perf_counter()
    #print(f'Duration script4: {e-s:.10f}s') 
    #end =  time.time()
    #cy_time = end - start
    #print("Cython time calcular_modulo_pto = {}".format(cy_time))
    return modulo


@cython.boundscheck(False)
cpdef pto_en_esfera(radii,centro,pto):
        
        #s = time.perf_counter() 
        cdef double d_cuadrada,x_centro,y_centro,z_centro,x_pto,y_pto,z_pto

        x_centro=centro[0]
        y_centro=centro[1]
        z_centro=centro[2]
        
        x_pto=pto[0]
        y_pto=pto[1]
        z_pto=pto[2]
        
        #distancia entre ambos puntos debe ser igual al radio del rayo de contexto
        d_cuadrada=(x_pto-x_centro)*(x_pto-x_centro)+(y_pto-y_centro)*(y_pto-y_centro)+(z_pto-z_centro)*(z_pto-z_centro)
        if d_cuadrada-(radii*radii)<0.001:
                return True
        else:
                return False
        #e = time.perf_counter()
        #print(f'Duration script4 pto_en_esfera: {e-s:.10f}s') 

@cython.boundscheck(False)
@cython.wraparound(False)
@cython.nonecheck(False)
@cython.cdivision(True) 
@cython.profile(True)
cpdef suma_capa(pto,dist):
        #s = time.perf_counter()
        cdef modulo,x,y,z

        modulo=sqrt((pto[0]*pto[0]+pto[1]*pto[1]+pto[2]*pto[2])) #para sumar una distancia de modulo 1
        #sumandole 1 al punto para obtener el in1
        x=pto[0]+dist*(pto[0]/modulo)
        y=pto[1]+dist*(pto[1]/modulo)
        z=pto[2]+dist*(pto[2]/modulo)
        #e = time.perf_counter()
        #print(f'Duration script4: {e-s:.10f}s') 
        return [x,y,z]                


@cython.boundscheck(False)  # Deactivate bounds checking
@cython.wraparound(False)   # Deactivate negative indexing.

cpdef float determinar_ses(list SES_points,list cs_in1,list cs_in2,list cs_in3,list cs_in4,list cs_out1,list cs_out2,list cs_out3,list cs_out4,list cs_ses):
    
    #cdef double ses_points = SES_points[0]
    cdef list item
    #global cs_in1,cs_in2,cs_in3,cs_in4,cs_out1,cs_out2,cs_out3,cs_out4,cs_ses
    cdef int cont_pto_tipo_ses=0
    #for i in range(len(myList)):
    #t1=time.time()
    for item in SES_points:
            print("cont_pto_tipo_ses",cont_pto_tipo_ses)
            cont_pto_tipo_ses=cont_pto_tipo_ses+1
            #print("item",item)
            #print("item0",item[0])
            #print("item11111",item[1])
            #print("perry")
            type_cell=item[1]
            number_ray=item[len(item)-1]
            if type_cell=="ses":
                origen=[float(item[2][0]),float(item[2][1]),float(item[2][2])] # para ver si está dentro de la recta y no sale del límite
                #final=[float(item[3][0]),float(item[3][1]),float(item[3][2])]
                #origen=>centro de la esfera, final=>fin del rayo
                inicio=[float(item[4][0]),float(item[4][1]),float(item[4][2])]#para calcular las capas
                #fin=[float(item[5][0]),float(item[5][1]),float(item[5][2])]
                #inicio=>inicio del segmento
                #fin=>fin del segmento
                #ses
                # xmean=(inicio[0]+fin[0])/2
                # ymean=(inicio[1]+fin[1])/2
                # zmean=(inicio[2]+fin[2])/2
                # ptos_ses=[xmean,ymean,zmean]
                ptos_ses=[inicio[0],inicio[1],inicio[2]]
                #restandole 1 al punto para obtener el in1
                #start = time.time()
                s = time.perf_counter()  
                ptos_in1=suma_capa(ptos_ses,-0.2)
                #restandole 1 al punto para obtener el in2
                ptos_in2=suma_capa(ptos_ses,-0.4)
                ptos_in3=suma_capa(ptos_ses,-0.8)
                ptos_in4=suma_capa(ptos_ses,-1)
                #out1
                ptos_out1=suma_capa(ptos_ses,0.2)
                #out2
                ptos_out2=suma_capa(ptos_ses,0.4)
                #out3
                ptos_out3=suma_capa(ptos_ses,0.8)
            
                ptos_out4=suma_capa(ptos_ses,1)
                e = time.perf_counter()
                print(f'Duration function suma_capa: {e-s:.10f}s')    
                #end =  time.time()
                #cy_time = end - start
                #print("Cython time suma_capa = {}".format(cy_time))
                #Para determinar si el pto de la capa calculada cae en la esfera se debe calcular la distancia
                r=3
                s = time.perf_counter() 
                d_in1=pto_en_esfera(r,origen,ptos_in1)
                d_in2=pto_en_esfera(r,origen,ptos_in2)
                d_in3=pto_en_esfera(r,origen,ptos_in3)
                d_in4=pto_en_esfera(r,origen,ptos_in4)
                d_out1=pto_en_esfera(r,origen,ptos_out1)
                d_out2=pto_en_esfera(r,origen,ptos_out2)
                d_out3=pto_en_esfera(r,origen,ptos_out3)
                d_out4=pto_en_esfera(r,origen,ptos_out4)
                d_ses=pto_en_esfera(r,origen,ptos_ses)
                e = time.perf_counter()
                print(f'Duration function pto_en_esfera: {e-s:.10f}s') 
                #print("llegue aqui")
                if d_in1:
                    cs_in1.append([item[0],number_ray,ptos_in1])

                if d_in2:
                    cs_in2.append([item[0],number_ray,ptos_in2])

                if d_in3:
                    cs_in3.append([item[0],number_ray,ptos_in3])

                if d_in4:
                    cs_in4.append([item[0],number_ray,ptos_in4])

                if d_out1:
                    cs_out1.append([item[0],number_ray,ptos_out1])

                if d_out2:
                    cs_out2.append([item[0],number_ray,ptos_out2])

                if d_out3:
                    cs_out3.append([item[0],number_ray,ptos_out3])

                if d_out4<1:
                    cs_out4.append([item[0],number_ray,ptos_out4])

                if d_ses:
                    cs_ses.append([item[0],number_ray,ptos_ses])

    #t2=time.time()
    #t = t2-t1
    #print("%.20f" % t)    
        #print("se termino")

