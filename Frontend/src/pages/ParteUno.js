import React from "react";
import { Card } from "primereact/card";
import CustomDropzone from "../components/CustomDropzone";
import StlPreview from "../components/StlPreview";
import { Button } from "primereact/button";
import { BlockUI } from "primereact/blockui";
import { ProgressSpinner } from "primereact/progressspinner";
import { Divider } from "primereact/divider";
import toast from "react-hot-toast";
import './ParteUno.css';

const ParteUno = ({
    socket,
    fileOne,
    setFileOne,
    fileTwo,
    setFileTwo,
    procesando,
    setProcesando,
    finalizado,
    setFinalizado,
    fileVertice,
    setFileVertice,
    fileFace,
    setFileFace,
}) => {
    const urlBack = process.env.REACT_APP_URL_BACKEND;

    const onSubmitFiles = () => {
        setFinalizado(false);
        setProcesando(true);
        uploadHandler();
    };

    const handleDownloadFiles = () => {
        window.location.href = `${urlBack}/descargaParteUno`;
        console.log("descargado");
    };

    const buttonStyles = {
    backgroundColor: 'black',  // Cambia el color de fondo a azul
    borderColor: 'black',      // Cambia el color del borde a rojo
    color: 'white',          // Cambia el color del texto a blanco
    marginLeft: ".5em", width: "27%"
    };

    const footer = (
        <span>
            <Button
                label="Subir"
                icon="pi pi-check"
                className="button"
                style={buttonStyles} // Agrega los estilos al botón
                disabled={
                    !fileOne.length || !fileVertice.length || !fileFace.length
                }
                onClick={onSubmitFiles}
            />
            <Button
                label="Cancelar"
                icon="pi pi-times"
                className="p-button-secondary button"
                style={buttonStyles} // Agrega los estilos al botón
                onClick={() => {
                    setFileOne([]);
                    setFileVertice([]);
                    setFileFace([]);
                }}
            />
        </span>
    );


    const uploadHandler = async () => {
        const formData = new FormData();

        formData.append("proteinaStl", fileOne[0]);
        formData.append("proteinaVertices", fileVertice[0]);
        formData.append("proteinaCaras", fileFace[0]);

        const response = await fetch(`${urlBack}/cargarParteUno`, {
            method: "POST",
            body: formData,
        });
        await response.json();
        toast.success("Archivos cargados correctamente", {
            duration: 4000,
            position: "top-right",
        });
    };

    return (
        <>
            <div className="upload-container">
                <BlockUI blocked={procesando} template={<ProgressSpinner />}>
                    <div style={{ display: "flex", justifyContent: "space-between" }}>
                        <Card
                            title="Lectura de caras y vértices"
                            footer={footer}
                            className="card"
                        >
                            <div className="upload-row">
                                <div style={{ width: "45%" }}>
                                    <CustomDropzone
                                        chipLabel="Vertices proteina"
                                        file={fileVertice}
                                        setFile={setFileVertice}
                                        acceptedFile=".vert"
                                    />
                                </div>
                                <div style={{ width: "45%" }}>
                                    <CustomDropzone
                                        chipLabel="Caras proteina"
                                        file={fileFace}
                                        setFile={setFileFace}
                                        acceptedFile=".face"
                                    />
                                </div>
                            </div>
                            <Divider />
                            <div className="upload-row">
                                <div style={{ width: "45%" }}>
                                    <CustomDropzone
                                        chipLabel="Archivo STL"
                                        file={fileOne}
                                        setFile={setFileOne}
                                        acceptedFile=".stl"
                                        blocked={
                                            !fileVertice.length || !fileFace.length
                                        }
                                    />
                                </div>
                            </div>
                        </Card>
                        {fileOne.length > 0 && (
                            <Card title="Vista previa del STL" style={{ marginLeft: "20px" }}>
                                <StlPreview file={fileOne[0]} />
                            </Card>
                        )}
                    </div>
                </BlockUI>
            </div>
            <div className="download-container">
                {finalizado && (
                    <Card>
                        <Button onClick={handleDownloadFiles}>
                            Descargar resultados
                        </Button>
                    </Card>
                )}
            </div>
        </>
    );
};
export default ParteUno;