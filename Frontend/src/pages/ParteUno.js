import React, { useState, useEffect } from "react";
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
    fileOne,
    setFileOne,
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
    const [isDownloadReady, setIsDownloadReady] = useState(false);

    useEffect(() => {
        if (finalizado) {
            setIsDownloadReady(true);
        }
    }, [finalizado]);

    const buttonStyles = {
        backgroundColor: 'black',
        color: 'white',
        borderColor: 'black',
        marginLeft: ".5em",
        width: "27%",
        transition: 'all 0.3s ease',
        ':hover': {
            backgroundColor: 'gray',
            boxShadow: '0 0 10px black, 0 0 40px black, 0 0 80px black',
        }
    };

    const onSubmitFiles = () => {
    if (!fileOne.length || !fileVertice.length || !fileFace.length) {
        toast.error('Error: Todos los campos son obligatorios');
        return;
    }

    const isFileTypeValid = (
        fileOne[0].name.endsWith('.stl') &&
        fileVertice[0].name.endsWith('.vert') &&
        fileFace[0].name.endsWith('.face')
    );

    if (!isFileTypeValid) {
        toast.error('Error: Los tipos de archivo son incorrectos');
        return;
    }

    setProcesando(true);
    uploadHandler().catch(() => {
        setProcesando(false);
    });
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
                disabled={!isDownloadReady}
            />
        </span>
    );

    const uploadHandler = async () => {
        const formData = new FormData();
        formData.append("proteinaStl", fileOne[0]);
        formData.append("proteinaVertices", fileVertice[0]);
        formData.append("proteinaCaras", fileFace[0]);

        try {
            const response = await fetch(`${urlBack}/cargarParteUno`, {
                method: "POST",
                body: formData,
            });

            if (!response.ok) {
                throw new Error('Error al cargar los archivos');
            }

            await response.json();
            toast.success("Archivos cargados correctamente", {
                duration: 4000,
                position: "top-right",
            });

            setIsDownloadReady(true);
            setProcesando(false);
            setFinalizado(true);
        } catch (error) {
            setProcesando(false);
            toast.error(error.message);
            throw error;
        }
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
