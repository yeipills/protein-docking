import React, { useState } from "react";
import { Button } from "primereact/button";
import { Sidebar } from 'primereact/sidebar';
import { useNavigate } from 'react-router-dom';

const Navbar = ({ connected = false }) => {
    const [visible, setVisible] = useState(false);
    const navigate = useNavigate();

    const handleNavigation = (path) => {
        navigate(path);
        setVisible(false);
    };

    return (
        <>
            <Button icon="pi pi-bars" onClick={(e) => setVisible(true)} style={{position: 'fixed', left: '1em', top: '1em', zIndex: 1000}} />
            <Sidebar visible={visible} onHide={(e) => setVisible(false)} style={{width:'20em', backgroundColor: '#f8f9fa'}} position="left" baseZIndex={1000} fullScreen={false}>
                <Button label="Inicio" icon="pi pi-fw pi-home" onClick={() => handleNavigation('/')} style={{width: '100%', marginBottom: '1em'}} />
                <Button label="Cálculo de Rayos de Contexto" icon="pi pi-fw pi-bolt" onClick={() => handleNavigation('/parteUno')} style={{width: '100%', marginBottom: '1em'}} />
                <Button label="Evaluación de capas" icon="pi pi-fw pi-chart-bar" onClick={() => handleNavigation('/parteDos')} style={{width: '100%', marginBottom: '1em'}} />
                <Button label="Parte 1" icon="pi pi-fw pi-server" style={{width: '100%', marginBottom: '1em'}} disabled />
                <div style={{color: connected ? 'green' : 'red'}}>
                    {connected ? 'Conectado' : 'No conectado'}
                </div>
            </Sidebar>
        </>
    );
};

export default Navbar;
