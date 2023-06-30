import React, { useState, useEffect } from "react";
import { Route, Routes } from "react-router-dom";
import Home from "./components/Home";
import Navbar from "./components/Navbar";
import ParteUno from "./pages/ParteUno";
import ParteDos from "./pages/ParteDos";
import { io } from "socket.io-client";
import toast, { Toaster } from "react-hot-toast";
const socket = io(process.env.REACT_APP_URL_SOCKET);

const App = () => {
    const [isConnected, setIsConnected] = useState(socket.connected);

    //!! Hooks parte uno
    const [fileOnePartOne, setFileOnePartOne] = useState([]);
    const [fileTwoPartOne, setFileTwoPartOne] = useState([]);

    const [fileVerticesPartOne, setFileVerticesPartOne] = useState([]);
    const [fileFacePartOne, setFileFacePartOne] = useState([]);

    const [procesandoPartOne, setProcesandoPartOne] = useState(false);
    const [finalizadoPartOne, setFinalizadoPartOne] = useState(false);
    //!! Hooks parte uno

    //!! Hooks parte dos
    const [fileOnePartTwo, setFileOnePartTwo] = useState([]);
    const [fileTwoPartTwo, setFileTwoPartTwo] = useState([]);

    const [procesandoPartTwo, setProcesandoPartTwo] = useState(false);
    const [finalizadoPartTwo, setFinalizadoPartTwo] = useState(false);

    const [nombreProteina, setNombreProteina] = useState("");
    //!! Hooks parte dos

    useEffect(() => {
        socket.on("connected_user", (e) => {
            setIsConnected(true);
            console.log("Cliente conectado id: ", e);
        });

        socket.on("disconnect", () => {
            socket.off("disconnect");
            setIsConnected(false);
        });

        socket.on("termina_procesar_uno", () => {
            toast.success("Termina de procesar parte 1", {
                duration: 4000,
                position: "top-right",
            });
            setFinalizadoPartOne(true);
            setProcesandoPartOne(false);
            console.log("Procesado");
        });

        socket.on("termina_procesar_dos", () => {
            toast.success("Termina de procesar parte 2", {
                duration: 4000,
                position: "top-right",
            });
            setFinalizadoPartTwo(true);
            setProcesandoPartTwo(false);
            console.log("Procesado");
        });

        return () => {
            setIsConnected(false);
            socket.off("connected_user");
            socket.off("disconnect");
            socket.off("termina_procesar_uno");
            socket.off("termina_procesar_dos");
        };
    }, []);

    return (
        <>
            <Navbar connected={isConnected} />
            <Toaster />
            <Routes>
                <Route path="/" element={<Home />} />
                <Route
                    path="/parteUno"
                    element={
                        <ParteUno
                            socket={socket}
                            fileOne={fileOnePartOne}
                            setFileOne={setFileOnePartOne}
                            fileTwo={fileTwoPartOne}
                            setFileTwo={setFileTwoPartOne}
                            procesando={procesandoPartOne}
                            setProcesando={setProcesandoPartOne}
                            finalizado={finalizadoPartOne}
                            setFinalizado={setFinalizadoPartOne}
                            fileVertice={fileVerticesPartOne}
                            setFileVertice={setFileVerticesPartOne}
                            fileFace={fileFacePartOne}
                            setFileFace={setFileFacePartOne}
                        />
                    }
                />
                <Route
                    path="/parteDos"
                    element={
                        <ParteDos
                            socket={socket}
                            fileOne={fileOnePartTwo}
                            setFileOne={setFileOnePartTwo}
                            fileTwo={fileTwoPartTwo}
                            setFileTwo={setFileTwoPartTwo}
                            procesando={procesandoPartTwo}
                            setProcesando={setProcesandoPartTwo}
                            finalizado={finalizadoPartTwo}
                            setFinalizado={setFinalizadoPartTwo}
                            nombreProteina={nombreProteina}
                            setNombreProteina={setNombreProteina}
                        />
                    }
                />
            </Routes>
        </>
    );
};

export default App;
