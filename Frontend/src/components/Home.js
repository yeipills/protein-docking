import React from "react";
import { Card } from "primereact/card";
import ubb_logo from "../resources/ubb_logo.png";

const styles = {
  container: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    height: "100vh",
    padding: "20px"
  },
  logoContainer: {
    position: "absolute",
    top: "10px",
    right: "10px",
  },
  logo: {
    width: "200px",
    height: "150px"
  },
  card: {
    textAlign: "center",
    border: "1px solid",
    borderColor: "#8B5CF6",
    borderRadius: "12px",
    backgroundColor: "#cddcdc",
    color: "black",
    maxWidth: "80%",
    margin: "20px"
  },
  cardText: {
    fontWeight: "light",
    fontSize: "20px",
    color: "#8B5CF6",
    textTransform: "uppercase"
  }
};

const Home = () => {
  return (
    <div style={styles.container}>
      <div style={styles.logoContainer}>
        <img src={ubb_logo} alt="fireSpot" style={styles.logo} />
      </div>
      <Card style={styles.card}>
        <span style={styles.cardText}>
          Análisis y comparación de los tiempos de ejecución en los algoritmos del cálculo de las formas de contexto para mejorar la precisión de las mediciones utilizadas en el proceso de docking
        </span>
      </Card>
      {/* Agrega aquí cualquier otro contenido adicional */}
    </div>
  );
};

export default Home;
