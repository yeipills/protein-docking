    import React from "react";
    import { Card } from "primereact/card";
    import ubb_logo from "../resources/ubb_logo.png";
    import { Button } from "primereact/button";
    import bg from "../resources/bg.jpeg";
    import './Home.css';


    const styles = {
      container: {
        position: "relative",
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
        right: "10px"
      },

      logo: {
        width: "200px",
        height: "150px"
      },

      card1: {
        textAlign: "center",
        position: "center",
        border: "1px solid",
        borderColor: "#727272",
        borderRadius: "12px",
        backgroundColor: "#F2F2F2",
        color: "black",
        maxWidth: "80%",
        margin: "10px"
      },

      cardText2: {
        position: "flex",
        fontWeight: "light",
        fontSize: "20px",
        color: "#111020",
        textTransform: "center"
      },

      cardText: {
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        fontWeight: "light",
        fontSize: "40px",
        color: "#111020",
        textTransform: "center",
        maxWidth: "120%",
        margin: "10px"
      },

      card2: {
        position: "absolute",
        top: "50%",
        left: "50%",
        transform: "translate(-50%, -50%)",
        // Other card styles...
      },

      card2ExpandLeft: {
        position: "absolute",
        top: "40%",
        left: "0",
        transform: "translateY(-50%)",
        textAlign: "center",
        border: "1px solid",
        borderColor: "#727272",
        borderRadius: "12px",
        backgroundColor: "#F2F2F2",
        color: "black",
        width: "80%",
        maxWidth: "100%",
        margin: "10px"
      },

      card3: {
        position: "absolute",
        right: "10px",
        top: "55%",
        transform: "translateY(-50%)",
        textAlign: "center",
        border: "1px solid",
        borderColor: "#727272",
        borderRadius: "12px",
        backgroundColor: "#F2F2F2",
        color: "black",
        maxWidth: "20%",
        margin: "10px"
      }
    };

    const Buttonstyle = {
      buttun1: {
        margin: "10px auto",
        width: "100%",
        backgroundColor: "#000000",
        color: "white",
        border: "none",
        padding: "15px",
        textAlign: "center",
        textDecoration: "none",
        display: "inline-block",
        fontSize: "16px",
        borderRadius: "4px",
        cursor: "pointer",
        transitionDuration: "0.4s",
        // Hover state
        ":hover": {
          backgroundColor: "#008CBA",
          color: "white "}
      }
    };

    const Home = () => {
      return (
        <div style={styles.container}>
          <div style={styles.logoContainer}>
            <img src={ubb_logo} alt="fireSpot" style={styles.logo} />
          </div>

          <Card style={styles.card1}>
            <span style={styles.cardText}>Protein Docking</span>
          </Card>

          <div style={{ width: "100%", overflowX: "hidden" }}>
            <Card style={styles.card2} className="expand-card">
              <span style={styles.cardText}>
                <text style={{ ...styles.cardText2 }}>
                  El acoplamiento molecular (conocido como docking) es una técnica para predecir cómo se unen compuestos y proteínas, útil para estudiar nuevos tratamientos. Aunque estos análisis pueden ser subjetivos debido a los criterios variados de selección de la "mejor pose" de los programas de docking, la aplicación del método semiempírico PM6 en la presente investigación mejoró la selección de poses, produciendo resultados con alta probabilidad de unión y menores energías de unión.
                </text>
              </span>
            </Card>
          </div>

               <Card style={styles.card3}>
                    <span style={{ fontSize: "20px", fontWeight: "bold" }}>Bancos de proteinas</span>

                    <div style={{ marginTop: "10px" }}>
                    <a href="https://www.rcsb.org/" target="_blank" style={Buttonstyle.buttun1}>
                  <Button style={Buttonstyle.buttun1} label="Protein Data Bank" />
                </a>

                <a href="https://zlab.umassmed.edu/benchmark/" target="_blank" style={Buttonstyle.buttun1}>
                  <Button style={Buttonstyle.buttun1} label="Zlab Bank" />
                </a>

                <a href="https://www.ncbi.nlm.nih.gov/" target="_blank" style={Buttonstyle.buttun1}>
                  <Button style={Buttonstyle.buttun1} label="National Library of Medicine" />
                </a>
              </div>
            </Card>
        </div>
      );
    };

    export default Home;
