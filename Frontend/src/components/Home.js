import React from "react";
import {Card} from "primereact/card";
import ubb_logo from "../resources/ubb_logo.png";
import {Button} from "primereact/button";
import  Image  from 'react';
import bg from "../resources/bg.jpeg";

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

  logoContainer2: {
  position: "absolute",
  left: "50%",  // Centra horizontalmente la imagen
  top:"310px",
  transform: "translateX(-1000px)",  // Ajusta la posición a la izquierda
},


  logo: {
    width: "200px",
    height: "150px"
  },

    logo2:{
    width: "450px",
    height: "450px",

  },

  card: {
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
  cardText: {
    position: "flex",
    fontWeight: "light",
    fontSize: "40px",
    color: "#111020",
    textTransform: "center"
  },
    cardText2: {
    display: "flex",
  flexDirection: "column",
  alignItems: "center",
  justifyContent: "center",
  fontWeight: "light",
  fontSize: "40px",
  color: "#111020",
  textTransform: "center",
  maxWidth: "100%",
  margin: "10px"
    },

  card2:{
   position: "absolute",
    right: "10px",
    top: "50%",
    transform: "translateY(-50%)",
    textAlign: "center",
    border: "1px solid",
    borderColor: "#727272",
    borderRadius: "12px",
    backgroundColor: "#F2F2F2",
    color: "black",
    maxWidth: "30%",
    margin: "10px"
  },

  card3:{
   position: "absolute",
    right: "10px",
    top: "50%",
    transform: "translateY(-50%)",
    textAlign: "center",
    border: "1px solid",
    borderColor: "#727272",
    borderRadius: "12px",
    backgroundColor: "#F2F2F2",
    color: "black",
    maxWidth: "30%",
    margin: "10px"
  },



    card1:{
   position: "absolute",
    center: "10px",
    top: "50%",
    transform: "translateY(-50%)",
    textAlign: "center",
    border: "1px solid",
    borderColor: "#727272",
    borderRadius: "12px",
    backgroundColor: "#F2F2F2",
    color: "black",
    maxWidth: "30%",
    margin: "10px"
  }

};
const Buttonstyle ={
  buttun1:{
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
    ':hover': {
      backgroundColor: "#008CBA",
      color: "white"
    }
  }
};

const Home = () => {
  return (
    <div style={styles.container}>
      <div style={styles.logoContainer}>
        <img src={ubb_logo} alt="fireSpot" style={styles.logo} />
      </div>


      <div style={styles.logoContainer2}>
        <img src={bg} alt="fireSpot" style={{...styles.logo2, border: '5px solid black'}} />
      </div>



      <Card style={styles.card}>
        <span style={styles.cardText}>
          Protein Docking
        </span>
      </Card>

<Card style={styles.card1}>
  <text style={{...styles.cardText2, textAlign: "left",fontSize: "25px"}}>
El acoplamiento molecular (conocido como docking) es una técnica para predecir cómo se unen compuestos y proteínas, útil para estudiar nuevos tratamientos. Aunque estos análisis pueden ser subjetivos debido a los criterios variados de selección de la "mejor pose" de los programas de docking, la aplicación del método semiempírico PM6 en la presente investigación mejoró la selección de poses, produciendo resultados con alta probabilidad de unión y menores energías de unión.
  </text>
</Card>



     <Card style={styles.card2}>
  <a href="https://www.rcsb.org/" target="_blank" style={Buttonstyle.buttun1}>
    <Button style={Buttonstyle.buttun1} label="Protein Data Bank"/>
  </a>

  <a href="https://zlab.umassmed.edu/benchmark/" target="_blank" style={Buttonstyle.buttun1}>
    <Button style={Buttonstyle.buttun1} label="zlab bank"/>
  </a>
 <a href="https://www.ncbi.nlm.nih.gov/" target="_blank" style={Buttonstyle.buttun1}>
   <Button style={Buttonstyle.buttun1} label="banco3"/>
  </a>
</Card>

    </div>
  );
};
export default Home;
