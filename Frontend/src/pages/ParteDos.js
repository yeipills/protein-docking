import React, {useState} from "react";
import { Card } from "primereact/card";
import CustomDropzone from "../components/CustomDropzone";
import { Button } from "primereact/button";
import { BlockUI } from "primereact/blockui";
import { ProgressSpinner } from "primereact/progressspinner";
import { InputText } from "primereact/inputtext";
import toast from "react-hot-toast";
import styles from './ParteDos.module.css';
import {Divider} from "primereact/divider"; // Importa el archivo de estilos CSS

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
        if ((fileOne[0] && !fileOne[0].name.toLowerCase().endsWith('.txt')) ||
            (fileTwo[0] && !fileTwo[0].name.toLowerCase().endsWith('.txt'))) {
            toast.error("Archivo .txt inválido. Por favor, sube archivos .txt válidos.");
            return;
        }
        setFinalizado(false);
        setProcesando(true);
        uploadHandler();
    };

     const [isDownloadReady, setIsDownloadReady] = useState(false);
    const handleDownloadFiles = () => {
        window.location.href = `${urlBack}/descargaParteDos`;
        console.log("descargado");
    };

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

const footer = (
    <span>
        <Button
            label="Subir"
            icon="pi pi-check"
            style={buttonStyles}
            disabled={
                !fileOne.length || !fileTwo.length || !nombreProteina.length
            }
            onClick={onSubmitFiles}
        />
        <Button
            label="Cancelar"
            icon="pi pi-times"
            className="p-button-secondary"
            style= {buttonStyles}
            onClick={() => {
                setFileOne([]);
                setFileTwo([]);
                setNombreProteina("");
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
        console.log(nombreProteina);
        formData.append("nombreProteina", nombreProteina);
        formData.append("crTotales", fileOne[0]);
        formData.append("rayosContexto", fileTwo[0]);

        try {
            const response = await fetch(`${urlBack}/cargarParteDos`, {
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
        } catch (error) {
            toast.error(error.message, {
                duration: 4000,
                position: "top-right",
            });
        }
    };

    return (
        <>
            <div className={styles['centered-container']}>
                <BlockUI blocked={procesando} template={<ProgressSpinner />}>
                    <div style={{ display: "flex", justifyContent: "center" }}>
                    <Card title="Evaluación de capas" footer={footer} className={styles.card}>
                        <div className={styles['flex-column']}>
                            <InputText
                                className={styles.input}
                                placeholder="Nombre proteína"
                                value={nombreProteina}
                                onChange={(e) => setNombreProteina(e.target.value)}
                                pt={{root: { className: 'border-primary-400' }}}

                            />
                        </div>
                        <Divider />
                        <div className={styles['spaced-container']}>
                            <div className={styles.dropzone}>
                                <CustomDropzone
                                    chipLabel="CR Totales"
                                    file={fileOne}
                                    setFile={setFileOne}
                                    acceptedFile=".txt"
                                />
                            </div>

                            <div className={styles.dropzone}>
                                <CustomDropzone
                                    chipLabel="Rayos contexto"
                                    file={fileTwo}
                                    setFile={setFileTwo}
                                    acceptedFile=".txt"
                                    />
                                </div>
                            </div>
                        </Card>
                    </div>
                </BlockUI>
            </div>

        </>
    );
};

export default ParteDos;
