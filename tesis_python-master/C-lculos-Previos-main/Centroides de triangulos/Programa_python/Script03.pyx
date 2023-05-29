



cimport cython
import numpy as np
cimport numpy as np
from libc.math cimport  sqrt, pi,sin,cos
#import numpy as np
import time




cpdef computeCR(radii,delta,centers):
        #cdef np.ndarray[DTYPE_INT_t, ndim=3] acc = np.zeros([m.size, size_max_song + size_trame + 1], dtype=DTYPE_INT)
        #cdef int i_range = m.size
        #cdef int j_range = active_points.shape[0]
        #start = time.time() 
        #cdef double[:] ray_directions_item,rayos_contexto,cr,ray_origins_item
        #cdef double[:, :, :] ray_directions_item,rayos_contexto,cr,ray_origins_item
        cdef list item,cr=[],rayos_contexto=[]
        cdef int counter=0,i,j
        cdef double x,y,z
        phi=[]
        theta=[]
        ray_directions_item=[]
        ray_origins_item=[]
        phi=np.linspace(0, 2*pi, delta) 
        theta=np.linspace(0, 2*pi, delta)

        for item in centers:
            for i in phi:
                for j in theta:               
                    x=radii*sin(i)*cos(j)+item[0]
                    y=radii*sin(i)*sin(j)+item[1]
                    z=radii*cos(i)+item[2]
                    counter=counter+1
                    ray_directions_item.append([x,y,z])
                    ray_origins_item.append(item)
                    final=str(x)+" "+str(y)+" "+str(z)
                    origen=str(item[0])+" "+str(item[1])+" "+str(item[2])
                    rayos_contexto.append(str(counter)+" "+origen+" "+final)
            cr.append([counter,item,ray_directions_item])
            counter=counter+1
        return cr,rayos_contexto    



       
cpdef float compute_function_final(list cr_segments,list cr,mesh_SES):
  
    cdef list item
    cdef list elem
    cdef double[:, :] origen
    cdef double[:, :] finales
    cdef double[:, :] vector
    cdef int count=0 
    cdef int n=17

    for item in cr:    
        origen=item[1]
        finales=item[2]
        count=count+1
        for rayo in finales:
            vector=np.linspace(origen,rayo,n) # Vector para dividir el rayo en segmentos
          
            hits=mesh_SES.ray.intersects_any(ray_origins=vector[0:len(vector)-1],
                                    ray_directions=vector[1:len(vector)]) #212,95 segundos 4 minutos
         
            string1=str(item[0])
           
            string2=" ".join([str(elem) for elem in item[1]])
           
            string3=" ".join([str(elem[0])+" "+str(elem[1])+" "+str(elem[2]) for elem in vector])
      
            string4=" ".join([str(elem) for elem in hits])
            
            cr_segments.append(string1+" "+string2+" "+string3+" "+string4)                   
    return cr_segments
