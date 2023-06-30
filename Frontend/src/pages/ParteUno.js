import React from "react";
import { Card } from "primereact/card";
import CustomDropzone from "../components/CustomDropzone";
import { Button } from "primereact/button";
import { BlockUI } from "primereact/blockui";
import { ProgressSpinner } from "primereact/progressspinner";
import { Divider } from "primereact/divider";
import toast from "react-hot-toast";

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

    const footer = (
        <span>
            <Button
                label="Subir"
                icon="pi pi-check"
                style={{ marginRight: ".25em", width: "27%" }}
                disabled={
                    !fileOne.length || !fileVertice.length || !fileFace.length
                }
                onClick={onSubmitFiles}
            />
            <Button
                label="Cancelar"
                icon="pi pi-times"
                className="p-button-secondary"
                style={{ marginLeft: ".25em", width: "27%" }}
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
            <div
                style={{
                    display: "flex",
                    justifyContent: "center",
                    marginTop: "4%",
                }}
            >
                <BlockUI blocked={procesando} template={<ProgressSpinner />}>
                    <Card
                        title="Lectura de caras y vértices"
                        footer={footer}
                        style={{
                            textAlign: "center",
                        }}
                    >
                        <div
                            style={{
                                display: "flex",
                                justifyContent: "space-around",
                            }}
                        >
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
                        <div
                            style={{
                                display: "flex",
                                justifyContent: "space-around",
                            }}
                        >
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
                </BlockUI>
            </div>
            <div
                style={{
                    display: "flex",
                    justifyContent: "center",
                    marginTop: "2%",
                    marginBottom: "2%",
                }}
            >
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
