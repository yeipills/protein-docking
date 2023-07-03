import React, { useCallback } from 'react';
import { useDropzone } from 'react-dropzone';
import { Tag } from "primereact/tag";
import { Chip } from "primereact/chip";
import { Card } from "primereact/card";
import { BlockUI } from "primereact/blockui";

const CustomDropzone = ({
    file,
    setFile,
    chipLabel,
    blocked = false,
    acceptedFile = '*',
}) => {
    const onDrop = useCallback((acceptedFiles) => {
        setFile(acceptedFiles);
    }, [setFile]);

    const { getRootProps, getInputProps } = useDropzone({
        onDrop,
        accept: acceptedFile,
    });

    const removeFile = () => setFile([]);

    const dropZoneContent = (
        <Card style={{ textAlign: 'center' }}>
            <input {...getInputProps()} />
            <p>Arrastra o clickea para cargar</p>
        </Card>
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
        <Chip
            key={key}
            style={{ marginTop: '5%' }}
            label={item.name}
            removable
            onRemove={removeFile}
        />
    ));

    return (
        <>
            <Tag
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
