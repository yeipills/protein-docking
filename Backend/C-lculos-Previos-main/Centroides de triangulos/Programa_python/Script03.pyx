cimport cython
import numpy as np
cimport numpy as np
from libc.math cimport  sqrt, pi,sin,cos

cpdef computeCR(double radii, int delta, list centers):
    cdef double[:] phi = np.linspace(0, 2*pi, delta)
    cdef double[:] theta = np.linspace(0, 2*pi, delta)

    cdef double[:, :] ray_directions_item = np.zeros((len(centers) * delta**2, 3))
    cdef double[:, :] ray_origins_item = np.zeros((len(centers) * delta**2, 3))
    cdef double[:, :] item
    cdef int counter = 0
    cdef double x, y, z

    for item in centers:
        for i in phi:
            for j in theta:
                x = radii * sin(i) * cos(j) + item[0]
                y = radii * sin(i) * sin(j) + item[1]
                z = radii * cos(i) + item[2]
                ray_directions_item[counter] = [x, y, z]
                ray_origins_item[counter] = item
                counter += 1
    return ray_directions_item, ray_origins_item


cpdef float compute_function_final(list cr_segments, double[:, :] ray_directions_item, double[:, :] ray_origins_item, mesh_SES):
    cdef double[:, :] vector
    cdef double[:, :] origen, rayo
    cdef double[:] hits
    cdef int n = 17
    cdef int count = 0

    for count in range(ray_directions_item.shape[0]):
        origen = ray_origins_item[count]
        rayo = ray_directions_item[count]
        vector = np.linspace(origen, rayo, n)
        hits = mesh_SES.ray.intersects_any(ray_origins=vector[:-1], ray_directions=vector[1:])
        cr_segments.append([count, origen, vector, hits])
    return cr_segments
