from distutils.core import Extension, setup
from Cython.Build import cythonize
# define an extension that will be cythonized and compiled
ext = Extension(name="Script04", sources=["Script04.pyx"])
setup(ext_modules=cythonize(ext, compiler_directives={'language_level' : "3"}))


#!python
#cython: language_level=3

# from setuptools import setup
# from Cython.Build import cythonize

# setup(
#     ext_modules = cythonize("Script04.pyx")
# )

