import React from "react";
import { Card } from "primereact/card";
import CustomDropzone from "../components/CustomDropzone";
import { Button } from "primereact/button";
import { BlockUI } from "primereact/blockui";
import { ProgressSpinner } from "primereact/progressspinner";
import { InputText } from "primereact/inputtext";
import toast from "react-hot-toast";

const ParteDos = ({
    socket,
    fileOne,
    setFileOne,
    fileTwo,
    setFileTwo,
    procesando,
    setProcesando,
    finalizado,
    setFinalizado,
    nombreProteina,
    setNombreProteina,
}) => {
    const urlBack = process.env.REACT_APP_URL_BACKEND;

    const onSubmitFiles = () => {
        setFinalizado(false);
        setProcesando(true);
        uploadHandler();
    };

    const handleDownloadFiles = () => {
        window.location.href = `${urlBack}/descargaParteDos`;
        console.log("descargado");
    };

    const footer = (
        <span>
            <Button
                label="Subir"
                icon="pi pi-check"
                style={{ marginRight: ".25em", width: "27%" }}
                disabled={
                    !fileOne.length || !fileTwo.length || !nombreProteina.length
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
                    setFileTwo([]);
                    setNombreProteina("");
                }}
            />
        </span>
    );

    const uploadHandler = async () => {
        const formData = new FormData();
        console.log(nombreProteina);
        formData.append("nombreProteina", nombreProteina);
        formData.append("crTotales", fileOne[0]);
        formData.append("rayosContexto", fileTwo[0]);

        const response = await fetch(`${urlBack}/cargarParteDos`, {
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
                        title="Evaluación de capas"
                        footer={footer}
                        style={{
                            textAlign: "center",
                        }}
                    >
                        <div
                            style={{
                                display: "flex",
                                flexDirection: "column",
                            }}
                        >
                            <InputText
                                style={{
                                    width: "60%",
                                    margin: "auto",
                                    marginBottom: "5%",
                                }}
                                placeholder="Nombre proteína"
                                value={nombreProteina}
                                onChange={(e) =>
                                    setNombreProteina(e.target.value)
                                }
                            />
                        </div>
                        <div
                            style={{
                                display: "flex",
                                justifyContent: "space-around",
                            }}
                        >
                            <div style={{ width: "45%" }}>
                                <CustomDropzone
                                    chipLabel="CR Totales"
                                    file={fileOne}
                                    setFile={setFileOne}
                                    acceptedFile=".txt"
                                />
                            </div>
                            <div style={{ width: "45%" }}>
                                <CustomDropzone
                                    chipLabel="Rayos contexto"
                                    file={fileTwo}
                                    setFile={setFileTwo}
                                    acceptedFile=".txt"
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
                }}
            >
                {finalizado && (
                    <Card>
                        <Button onClick={handleDownloadFiles}>
                            Descargar resultados
                        </Button>
                    </Card>
                )}{" "}
            </div>
        </>
    );
};

export default ParteDos;
