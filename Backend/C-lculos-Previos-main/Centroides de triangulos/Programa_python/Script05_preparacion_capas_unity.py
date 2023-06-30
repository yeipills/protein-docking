################## PROGRAMA 5 ###################
import numpy as np
import csv
import math
import os
import trimesh
import math
import time
#import Script04_evaluacion_capas as S4
######################## DATOS INICIALES ###########################
s = time.perf_counter()
nombreProteina='1AHW_l_u'

####################################
#S4.getCs(nombreProteina)

dirname = os.path.dirname(__file__)
carpeta_cr='./proteinas_cr/'
name_rayos_contexto=carpeta_cr+str(nombreProteina)+str('_rayos_contexto.txt')
fname_rayos_contexto=os.path.join(dirname,name_rayos_contexto)
#file_rayos_contexto=open(fname_rayos_contexto,'r')



cantidad_segmentos=15
cantidad_rayos=100
file_rayos_contexto=open(fname_rayos_contexto,'r')
lines_rayos_contexto=file_rayos_contexto.readlines()
n=len(lines_rayos_contexto)

centro_raw=list()
inicio_fin=list()
index_cs=list()
for item in lines_rayos_contexto:
    a=item.split(" ")
    centro=[float(a[1]),float(a[2]),float(a[3])]
    fin=[float(a[4]),float(a[5]),float(a[6])]
    centro_raw.append(centro)
    inicio_fin.append([centro,fin])
    index_cs.append(a[0])


cantidad_cs=int(len(index_cs))

centro_unique=np.unique(centro_raw,axis=0)
number_cs=np.arange(0,cantidad_cs)
centro_final=list(zip(number_cs,centro_unique))

########### RESUMEN CS PARA UNITY ################
carpeta_cs_unity='./proteinas_cs_unity/'
name_resumen_cs_unity=carpeta_cs_unity+str(nombreProteina)+str('_resumen_cs_unity.txt')
fname_resumen_cs_unity=os.path.join(dirname,name_resumen_cs_unity)

with open(fname_resumen_cs_unity, 'w') as f:
    for item in centro_final:
        string1=str(item[0])
        string2=" ".join([str(elem) for elem in item[1]])
        string=string1+" "+string2
        f.write("%s\n" % string)
############ FUNCION ESCRIBIR ARCHIVO ###############################
def escribir_archivo(nombre,lista):
    with open(nombre, 'w') as f:
        for item in lista:
            string=str(item[0])
            string1=str(item[1])
            string2=" ".join([str(elem) for elem in item[2]])
            string3=" ".join([str(elem[0])+" "+str(elem[1])+" "+str(elem[2]) for elem in item[3]])            
            string_final=string+" "+string1+" "+string2+" "+string3
            f.write("%s\n" % string_final)
####################################################################

def reshape_context_shapes(nombre_archivo,lista_final):
    file_ses=open(nombre_archivo)
    lines=file_ses.readlines()
    cs_number_raw=list()
    for item in range(0,cantidad_cs):
        [cs_number_raw.append(item) for i in range(0,cantidad_rayos)]
    cs_ses_raw=list()
    segmentos_raw=list()
    for item in lines:
        a=item.split(" ")
        cs=a[0]
        cr=a[1]
        value=a[2]
        cs_ses_raw.append([int(cs),int(cr),int(value)])
        segmentos_raw.append(int(value))
    
    
    #vector con valores de ses
    ls=len(segmentos_raw)
    array_segmentos=np.reshape(segmentos_raw,(int(ls/cantidad_segmentos),cantidad_segmentos))
    indice_ray=np.arange(len(array_segmentos))
    [lista_final.append([cs_number_raw[i],indice_ray[i],array_segmentos[i],inicio_fin[i]]) for i in range(0,len(array_segmentos))]

cs_ses_final=list()
cs_in1_final=list()
cs_in2_final=list()
cs_in3_final=list()
cs_in4_final=list()
cs_out1_final=list()
cs_out2_final=list()
cs_out3_final=list()
cs_out4_final=list()

cs_vol_final=list()

carpeta_cs=str('./proteinas_cs/')
name_cs=carpeta_cs+str(nombreProteina)
fname_ses=os.path.join(dirname,name_cs+'_ses.txt')
fname_in1=os.path.join(dirname,name_cs+'_in1.txt')
fname_in2=os.path.join(dirname,name_cs+'_in2.txt')
fname_in3=os.path.join(dirname,name_cs+'_in3.txt')
fname_in4=os.path.join(dirname,name_cs+'_in4.txt')
fname_out1=os.path.join(dirname,name_cs+'_out1.txt')
fname_out2=os.path.join(dirname,name_cs+'_out2.txt')
fname_out3=os.path.join(dirname,name_cs+'_out3.txt')
fname_out4=os.path.join(dirname,name_cs+'_out4.txt')

fname_vol=os.path.join(dirname,name_cs+'_vol.txt')

reshape_context_shapes(fname_ses,cs_ses_final)
reshape_context_shapes(fname_in1,cs_in1_final)
reshape_context_shapes(fname_in2,cs_in2_final)
reshape_context_shapes(fname_in3,cs_in3_final)
reshape_context_shapes(fname_in4,cs_in4_final)
reshape_context_shapes(fname_out1,cs_out1_final)
reshape_context_shapes(fname_out2,cs_out2_final)
reshape_context_shapes(fname_out3,cs_out3_final)
reshape_context_shapes(fname_out4,cs_out4_final)

reshape_context_shapes(fname_vol,cs_vol_final)
# print(cs_ses_final[0])
carpeta_cs_unity='./proteinas_cs_unity/'
name_cs_unity=carpeta_cs_unity+str(nombreProteina)
fname_cs_unity=os.path.join(dirname,name_cs_unity)

escribir_archivo(fname_cs_unity+'_cs_ses_unity.txt',cs_ses_final)
escribir_archivo(fname_cs_unity+'_cs_in1_unity.txt',cs_in1_final)
escribir_archivo(fname_cs_unity+'_cs_in2_unity.txt',cs_in2_final)
escribir_archivo(fname_cs_unity+'_cs_in3_unity.txt',cs_in3_final)
escribir_archivo(fname_cs_unity+'_cs_in4_unity.txt',cs_in4_final)
escribir_archivo(fname_cs_unity+'_cs_out1_unity.txt',cs_out1_final)
escribir_archivo(fname_cs_unity+'_cs_out2_unity.txt',cs_out2_final)
escribir_archivo(fname_cs_unity+'_cs_out3_unity.txt',cs_out3_final)
escribir_archivo(fname_cs_unity+'_cs_out4_unity.txt',cs_out4_final)

escribir_archivo(fname_cs_unity+'_cs_vol_unity.txt',cs_vol_final)
print("Termino escritura context shapes para unity")
e = time.perf_counter()
print(f'Duration script 5: {e-s:.4f}s')
