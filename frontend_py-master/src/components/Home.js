import React from "react";
import { Card } from "primereact/card";
import bg from "../resources/bg.png";
import ubb_logo from "../resources/ubb_logo.png";

const Home = () => {
    return (
        <>
        <div style={{display:"flex", flexDirection:"row"}}>
        <div style={{ width: "60%", }}>
        <div
                style={{
                    backgroundImage: `url(${bg})`,
                    position: "absolute",
                    width: "60%",
                    height: "91.8%",
                    backgroundRepeat: "no-repeat",
                    backgroundPosition: "center",
                    backgroundSize: "cover",
                }}
            />


        </div>


        <div style={{    height: "91.8%", width: "38%" ,marginLeft:"1%" ,   }}>

            <div>  <img  src={ubb_logo} alt="fireSpot"  width="300" height="250" style={{marginLeft:"25%"}}/></div>
            <Card style={{ textAlign: "center", 
            border: "1px solid",
            borderColor:"#8B5CF6",
            borderRadius: "12px",
            backgroundColor:"#cddcdc",
            
//   position: "absolute",
//   marginTop: "50%",
//   left: "50%",
//   transform: "translate(-50%, -50%)",
  color: "black"
  }}>
   <span style={{
    // fontWeight: "500",
    fontWeight: "light",
    fontSize:"20px",  
    // fontFamily: "cursive", 
    color:"#8B5CF6",
    textTransform: "uppercase",
    // color:"white"
//    fontFamily: "oblique"
   }}>
    Análisis y comparación de los tiempos de ejecución en los algoritmos del cálculo de las formas de contexto para  mejorar la precisión de las mediciones utilizadas en el proceso de docking
                    {/* <br /> */}
                   
                  
                    {/* <br /> */}
                    </span>

            </Card>


        </div>
        </div>
              {/* <div
                style={{
                    backgroundImage: `url(${bg})`,
                    position: "absolute",
                    width: "100%",
                    height: "91.8%",
                    backgroundRepeat: "no-repeat",
                    backgroundPosition: "center",
                    backgroundSize: "cover",
                }}
            /> */}


          
        </>
    );
};

export default Home;
