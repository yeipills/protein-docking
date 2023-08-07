    import React, {useState} from "react";
    import { Card } from "primereact/card";
    import CustomDropzone from "../components/CustomDropzone";
    import { Button } from "primereact/button";
    import { BlockUI } from "primereact/blockui";
    import { ProgressSpinner } from "primereact/progressspinner";
    import { InputText } from "primereact/inputtext";
    import toast from "react-hot-toast";
    import styles from './ParteDos.module.css';
    import {Divider} from "primereact/divider";

    const validateFiles = (file1, file2) => {
        if ((file1 && !file1.name.toLowerCase().endsWith('.txt')) ||
            (file2 && !file2.name.toLowerCase().endsWith('.txt'))) {
            return false;
        }
        return true;
    };



    const uploadFiles = async (url, formData) => {
        const response = await fetch(url, { method: "POST", body: formData });
        if (!response.ok) {
            throw new Error(`Error: ${response.statusText}`);
        }
        return response.json();
    };

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

        const onSubmitFiles = async () => {
            if (!validateFiles(fileOne[0], fileTwo[0])) {
                toast.error("Archivo .txt inválido. Por favor, sube archivos .txt válidos.");
                return;
            }
            setFinalizado(false);
            setProcesando(true);

            const formData = new FormData();
            formData.append("nombreProteina", nombreProteina);
            formData.append("crTotales", fileOne[0]);
            formData.append("rayosContexto", fileTwo[0]);

            try {
                await uploadFiles(`${urlBack}/cargarParteDos`, formData);
                toast.success("Archivos cargados correctamente", {
                    duration: 4000,
                    position: "top-right",
                });
            } catch (error) {
                toast.error(error.message, {
                    duration: 4000,
                    position: "top-right",
                });
            } finally {
                setProcesando(false);
            }
        };

        const [isDownloadReady, setIsDownloadReady] = useState(false);
        const handleDownloadFiles = () => {
            window.location.href = `${urlBack}/descargaParteDos`;
            console.log("descargado");
        };

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
                    disabled={!isDownloadReady}
                />
            </span>
        );

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
