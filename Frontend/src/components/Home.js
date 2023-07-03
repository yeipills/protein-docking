import React from "react";
import { Card } from "primereact/card";
import bg from "../resources/bg.png";
import ubb_logo from "../resources/ubb_logo.png";

// Define los estilos en un objeto fuera del componente de renderizado
const styles = {
    flexContainer: {
        display: "flex",
        flexDirection: "row"
    },
    leftPane: {
        width: "60%"
    },
    rightPane: {
        height: "91.8%",
        width: "38%" ,
        marginLeft:"1%"
    },
    background: {
        backgroundImage: `url(${bg})`,
        position: "absolute",
        width: "60%",
        height: "91.8%",
        backgroundRepeat: "no-repeat",
        backgroundPosition: "center",
        backgroundSize: "cover"
    },
    logo: {
        marginLeft:"25%"
    },
    card: {
        textAlign: "center",
        border: "1px solid",
        borderColor:"#8B5CF6",
        borderRadius: "12px",
        backgroundColor:"#cddcdc",
        color: "black"
    },
    cardText: {
        fontWeight: "light",
        fontSize:"20px",
        color:"#8B5CF6",
        textTransform: "uppercase"
    }
};

const Home = () => {
    return (
        <div style={styles.flexContainer}>
            <div style={styles.leftPane}>
                <div style={styles.background} />
            </div>
            <div style={styles.rightPane}>
                <img src={ubb_logo} alt="fireSpot" width="300" height="250" style={styles.logo} />
                <Card style={styles.card}>
                    <span style={styles.cardText}>
                        Análisis y comparación de los tiempos de ejecución en los algoritmos del cálculo de las formas de contexto para  mejorar la precisión de las mediciones utilizadas en el proceso de docking
                    </span>
                </Card>
            </div>
        </div>
    );
};

export default Home;
