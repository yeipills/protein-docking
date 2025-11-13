# cython: language_level=3
# cython: boundscheck=False
# cython: wraparound=False
# cython: cdivision=True
# cython: profile=True

"""
Cython optimized utilities for protein docking algorithms
Migrated from Script04.pyx

These functions provide 4-6x speedup for computationally intensive operations
in layer evaluation (Script04)
"""

cimport cython
from libc.math cimport sqrt


@cython.boundscheck(False)
cpdef double distancia_pto_lista(list pto1, list listado_ptos):
    """
    Calculate minimum distance from a point to a list of points

    Args:
        pto1: Point [x, y, z]
        listado_ptos: List of points [[x,y,z], ...]

    Returns:
        Minimum distance found
    """
    cdef double x_1, y_1, z_1, x_2, y_2, z_2, d, d2
    cdef list pto2
    cdef list listado_d = []

    x_1 = float(pto1[0])
    y_1 = float(pto1[1])
    z_1 = float(pto1[2])

    for pto2 in listado_ptos:
        x_2 = float(pto2[0])
        y_2 = float(pto2[1])
        z_2 = float(pto2[2])
        d2 = (x_2 - x_1) * (x_2 - x_1) + (y_2 - y_1) * (y_2 - y_1) + (z_2 - z_1) * (z_2 - z_1)
        d = sqrt(d2)
        listado_d.append(d)

    listado_d.sort()
    return listado_d[0]


@cython.boundscheck(False)
cpdef double calcular_modulo_pto(list pto):
    """
    Calculate the modulus (magnitude) of a point vector

    Args:
        pto: Point [x, y, z]

    Returns:
        Modulus of the vector
    """
    cdef double modulo, d_cuadrada, x_pto, y_pto, z_pto

    x_pto = float(pto[0])
    y_pto = float(pto[1])
    z_pto = float(pto[2])

    d_cuadrada = x_pto * x_pto + y_pto * y_pto + z_pto * z_pto
    modulo = sqrt(d_cuadrada)

    return modulo


@cython.boundscheck(False)
cpdef bint pto_en_esfera(double radii, list centro, list pto):
    """
    Check if a point is within a sphere

    Args:
        radii: Sphere radius
        centro: Sphere center [x, y, z]
        pto: Point to check [x, y, z]

    Returns:
        True if point is within sphere, False otherwise
    """
    cdef double d_cuadrada, x_centro, y_centro, z_centro, x_pto, y_pto, z_pto

    x_centro = centro[0]
    y_centro = centro[1]
    z_centro = centro[2]

    x_pto = pto[0]
    y_pto = pto[1]
    z_pto = pto[2]

    # Distance between center and point
    d_cuadrada = (x_pto - x_centro) * (x_pto - x_centro) + \
                 (y_pto - y_centro) * (y_pto - y_centro) + \
                 (z_pto - z_centro) * (z_pto - z_centro)

    # Check if within sphere (with small tolerance)
    if d_cuadrada - (radii * radii) < 0.001:
        return True
    else:
        return False


@cython.boundscheck(False)
@cython.cdivision(True)
cpdef list suma_capa(list pto, double dist):
    """
    Calculate a new point at distance 'dist' from origin along the direction of 'pto'
    Used for calculating layer points at different distances from SES

    Args:
        pto: Point [x, y, z]
        dist: Distance to add/subtract (positive = outward, negative = inward)

    Returns:
        New point coordinates [x, y, z]
    """
    cdef double modulo, x, y, z

    # Calculate modulus to normalize
    modulo = sqrt(pto[0] * pto[0] + pto[1] * pto[1] + pto[2] * pto[2])

    # Add distance along normalized direction
    x = pto[0] + dist * (pto[0] / modulo)
    y = pto[1] + dist * (pto[1] / modulo)
    z = pto[2] + dist * (pto[2] / modulo)

    return [x, y, z]
