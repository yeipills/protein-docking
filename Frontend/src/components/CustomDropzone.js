import React, { useCallback } from 'react';
import { useDropzone } from 'react-dropzone';
import { Tag } from "primereact/tag";
import { Chip } from "primereact/chip";
import { Card } from "primereact/card";
import { BlockUI } from "primereact/blockui";
import styled from 'styled-components';
import toast from 'react-hot-toast';

// Aquí creas componentes con estilos personalizados
const StyledCard = styled(Card)`
  background-color: #ffffff; // Cambia a tu color de preferencia
  color: black; // Cambia a tu color de preferencia
  text-align: center;
`;

const StyledTag = styled(Tag)`
  background-color: #727272; // Cambia a tu color de preferencia
  color: white; // Cambia a tu color de preferencia
`;

const StyledChip = styled(Chip)`
    background-color: green; // Cambia a tu color de preferencia
    color: white; // Cambia a tu color de preferencia
`;

const CustomDropzone = ({
    file,
    setFile,
    chipLabel,
    blocked = false,
    acceptedFile = '*',
    }) => {
    const onDrop = useCallback((acceptedFiles) => {
    if (acceptedFiles.length === 0) {
        return;
    }
    const file = acceptedFiles[0];
    const fileName = file.name.toLowerCase();
    if ((acceptedFile === '.stl' && !fileName.endsWith('.stl')) ||
        (acceptedFile === '.vert' && !fileName.endsWith('.vert')) ||
        (acceptedFile === '.face' && !fileName.endsWith('.face')) ||
        (acceptedFile === '.txt' && !fileName.endsWith('.txt'))) {
        toast.error(`Archivo ${acceptedFile} inválido. Por favor, sube un archivo ${acceptedFile} válido.`);
        return;
    }
    setFile(acceptedFiles);
    }, [setFile, acceptedFile]);

    const { getRootProps, getInputProps } = useDropzone({
        onDrop,
        accept: acceptedFile,
    });

    const removeFile = () => setFile([]);

    const dropZoneContent = (
        <StyledCard>
            <input {...getInputProps()} />
            <p>Haga clic o arrastre los archivos aquí</p>
        </StyledCard>
    );

    const blockUIContent = (
        <BlockUI
            blocked={file.length || blocked}
            template={<i className="pi pi-lock" style={{ fontSize: '3rem' }} />}
        >
            {dropZoneContent}
        </BlockUI>
    );

    const chips = file.map((item, key) => (
        <StyledChip
            key={key}
            style={{ marginTop: '5%' }}
            label={item.name}
            removable
            onRemove={removeFile}
        />
    ));

    return (
        <>
            <StyledTag
                key={chipLabel}
                style={{ marginBottom: '5%' }}
                value={chipLabel}
                rounded
            />
            <div {...getRootProps({ className: 'dropzone' })}>
                {blockUIContent}
            </div>
            {chips}
        </>
    );
};

export default CustomDropzone;
