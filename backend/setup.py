"""
Setup file for Cython compilation
Compiles optimized Cython modules for protein docking algorithms
"""
from setuptools import setup, Extension
from Cython.Build import cythonize
import numpy

# Define Cython extensions
extensions = [
    Extension(
        "app.algorithms.cython_utils",
        ["app/algorithms/cython_utils.pyx"],
        include_dirs=[numpy.get_include()],
        extra_compile_args=['-O3'],  # Maximum optimization
    )
]

setup(
    name="protein-docking-algorithms",
    version="2.0.0",
    description="Optimized algorithms for protein docking platform",
    ext_modules=cythonize(
        extensions,
        compiler_directives={
            'language_level': "3",
            'boundscheck': False,
            'wraparound': False,
            'cdivision': True,
            'profile': True
        },
        annotate=True,  # Generate HTML annotation files for optimization analysis
    ),
    zip_safe=False,
)
