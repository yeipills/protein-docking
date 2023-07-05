import React from "react";
import { Card } from "primereact/card";
import ubb_logo from "../resources/ubb_logo.png";

const styles = {
  container: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "flex-start",
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
    borderColor: "#727272",
    borderRadius: "12px",
    backgroundColor: "#F2F2F2",
    color: "black",
    maxWidth: "80%",
    margin: "10px"
  },
  cardText: {
    position: "flex",
    fontWeight: "light",
    fontSize: "40px",
    color: "#111020",
    textTransform: "center"
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
          Protein Docking
        </span>
      </Card>
      {/* Agrega aquí cualquier otro contenido adicional */}
    </div>
  );
};

export default Home;
