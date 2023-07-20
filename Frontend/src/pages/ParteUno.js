import React, { useState } from "react";
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

    // Create a new state variable to represent whether the file is ready to be downloaded
    const [isDownloadReady, setIsDownloadReady] = useState(false);

    const buttonStyles = {
        backgroundColor: 'black', // Cambia el color de fondo
        color: 'white',          // Cambia el color del texto
        borderColor: 'black',    // Cambia el color del borde
        marginLeft: ".5em",
        width: "27%",
        transition: 'all 0.3s ease', // Agrega transiciones suaves a los cambios de estilos
        ':hover': {
        backgroundColor: 'gray', // Cambia el color de fondo al pasar el mouse por encima
        boxShadow: '0 0 10px black, 0 0 40px black, 0 0 80px black', // Agrega un efecto de 'glow' al pasar el mouse por encima
    }
    };

    const onSubmitFiles = () => {
        if (!fileOne.length || !fileVertice.length || !fileFace.length) {
            toast.error('Error: Todos los campos son obligatorios');
            return;
        }
        const isFileTypeValid = (
            fileOne[0].type === 'application/vnd.ms-pki.stl' &&
            fileVertice[0].type === 'text/plain' &&
            fileFace[0].type === 'text/plain'
        );

        if (!isFileTypeValid) {
            toast.error('Error: Los tipos de archivo son incorrectos');
            return;
        }

        setFinalizado(false);
        setProcesando(true);
        uploadHandler();
    };

    const handleDownloadFiles = () => {
        window.location.href = `${urlBack}/descargaParteUno`;
        console.log("descargado");
    };

    const footer = (
        <span>
            <Button
                label="Subir"
                icon="pi pi-check"
                className="button"
                style={buttonStyles}
                disabled={
                    !fileOne.length || !fileVertice.length || !fileFace.length
                }
                onClick={onSubmitFiles}

            />
            <Button
                label="Cancelar"
                icon="pi pi-times"
                className="p-button-secondary button"
                style={buttonStyles}
                onClick={() => {
                    setFileOne([]);
                    setFileVertice([]);
                    setFileFace([]);
                }}
            />
            <Button
                label="Descargar"
                onClick={handleDownloadFiles}
                className="p-button-info button"
                style={buttonStyles}
                disabled={!isDownloadReady} // Button will be disabled until the file is ready
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

        if (!response.ok) {
            toast.error('Error al cargar los archivos');
            return;
        }

        await response.json();
        toast.success("Archivos cargados correctamente", {
            duration: 4000,
            position: "top-right",
        });

        // Update the download ready flag here
        setIsDownloadReady(true);
    };

    return (
        <>
            <div className="upload-container">
                <BlockUI blocked={procesando} template={<ProgressSpinner />}>
                    <div style={{ display: "flex", justifyContent: "center" }}>
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
        </>
    );
};


export default ParteUno;
