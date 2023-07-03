// ./hooks/useUploadHandler.js

import { useState } from 'react';
import toast from 'react-hot-toast';

export const useUploadHandler = (setProcesando, setFinalizado, successMessage, errorMessage) => {
  const urlBack = process.env.REACT_APP_URL_BACKEND;

  const uploadHandler = async (fileOne, fileTwo, nombreProteina) => {
    const formData = new FormData();
    formData.append('nombreProteina', nombreProteina);
    formData.append('crTotales', fileOne[0]);
    formData.append('rayosContexto', fileTwo[0]);

    try {
      const response = await fetch(`${urlBack}/cargarParteDos`, {
        method: 'POST',
        body: formData,
      });
      if (!response.ok) {
        throw new Error('Error al cargar los archivos');
      }
      await response.json();
      toast.success(successMessage, {
        duration: 4000,
        position: 'top-right',
      });
      setFinalizado(true);
    } catch (error) {
      toast.error(errorMessage || error.message, {
        duration: 4000,
        position: 'top-right',
      });
    } finally {
      setProcesando(false);
    }
  };

  return uploadHandler;
};
