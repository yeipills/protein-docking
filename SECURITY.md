# Seguridad

`cshape` está pensado como herramienta de laboratorio: un solo servicio, sin cuentas de
usuario.

- **Acceso.** Sin `CSHAPE_TOKEN` la API queda abierta; úselo así solo en `localhost` o en
  una red de confianza. Con `CSHAPE_TOKEN`, toda la API salvo `/api/health` exige
  `Authorization: Bearer <token>` (o `?token=` en descargas y EventSource). La comparación
  es de tiempo constante. Fuera de una red de confianza, ponga el servicio detrás de un proxy
  con HTTPS, porque el token viaja en cada petición.
- **Archivos.** Las subidas se escriben con nombres fijos dentro del directorio del trabajo,
  y los nombres de proteína se limpian (`[A-Za-z0-9_.-]`). Las descargas solo sirven
  archivos del directorio de salida del trabajo y rechazan rutas con separadores. El tamaño
  máximo de subida es `CSHAPE_MAX_UPLOAD_MB` (1024 MiB por defecto).
- **Validación.** Los archivos MSMS/STL se analizan antes de encolar el trabajo; los índices
  fuera de rango y los parámetros inválidos se rechazan con 400.
- **Contenedor.** La imagen corre como usuario sin privilegios (`uid 10001`) y solo
  escribe en `/app/var`.

Para reportar una vulnerabilidad, abra un issue privado (Security advisory) en GitHub.
