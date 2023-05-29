from distutils.core import Extension, setup
from Cython.Build import cythonize
import numpy
# define an extension that will be cythonized and compiled
ext = Extension(name="Script03", sources=["Script03.pyx"])
setup(ext_modules=cythonize(ext, compiler_directives={'language_level' : "3"}),include_dirs=[numpy.get_include()])


#!python
#cython: language_level=3

# from setuptools import setup
# from Cython.Build import cythonize

# setup(
#     ext_modules = cythonize("Script04.pyx")
# )

