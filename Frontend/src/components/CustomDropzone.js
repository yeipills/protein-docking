import { Card } from "primereact/card";
import React from "react";
import { useDropzone } from "react-dropzone";
import { Tag } from "primereact/tag";
import { Chip } from "primereact/chip";
import { BlockUI } from "primereact/blockui";

const CustomDropzone = ({
    file,
    setFile,
    chipLabel,
    blocked = false,
    acceptedFile = "*",
}) => {
    const { acceptedFiles, getRootProps, getInputProps } = useDropzone({
        getFilesFromEvent: (event) => myCustomFileGetter(event),
        accept: {
            acceptedFile: [acceptedFile],
        },
    });

    function nameLengthValidator(file) {
        if (file.name.length > maxLength) {
            return {
                code: "name-too-large",
                message: `Name is larger than ${maxLength} characters`,
            };
        }

        return null;
    }

    async function myCustomFileGetter(event) {
        const files = [];
        const fileList = event.dataTransfer
            ? event.dataTransfer.files
            : event.target.files;

        for (var i = 0; i < fileList.length; i++) {
            const file = fileList.item(i);

            Object.defineProperty(file, "myProp", {
                value: true,
            });

            files.push(file);
        }
        setFile(files);
        return files;
    }

    return (
        <>
            <Tag
                key={chipLabel}
                style={{ marginBottom: "5%" }}
                value={chipLabel}
                className="mr-2"
                rounded
            />
            <BlockUI
                blocked={file.length || blocked}
                template={
                    <i className="pi pi-lock" style={{ fontSize: "3rem" }} />
                }
            >
                <div {...getRootProps({ className: "dropzone" })}>
                    <Card
                        style={{
                            textAlign: "center",
                        }}
                        disabled
                    >
                        <input {...getInputProps()} />
                        <p>Arrastra o clickea para cargar</p>
                    </Card>
                </div>
            </BlockUI>
            {file.map((item, key) => (
                <Chip
                    key={key}
                    style={{ marginTop: "5%" }}
                    label={item.name}
                    removable
                    onRemove={() => setFile([])}
                />
            ))}
        </>
    );
};

export default CustomDropzone;
