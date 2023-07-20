import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from "primereact/button";
import { Sidebar } from 'primereact/sidebar';

const Navbar = ({ connected = false, buttonColor = '#000000', buttonBorderColor = '#000000', onToggleSidebar, isSidebarExpanded }) => {
    const navigate = useNavigate();

    const handleNavigation = (path) => {
        navigate(path);
        onToggleSidebar(false);
    };

    return (
        <>
            <Sidebar
                visible={true}
                style={{
                    width: isSidebarExpanded ? '18em' : '5em',
                    backgroundColor: '#1f253f',
                    position: 'fixed',
                    left: '1em',
                    top: '1em',
                    bottom: '1em',
                    borderRadius: '10px',
                    transition: 'width 0.5s',
                    height: 'calc(100% - 2em)'
                }}
                position="left"
                baseZIndex={1000}
                fullScreen={false}
                showCloseIcon={false}
                blockScroll={false}
                modal={isSidebarExpanded}
            >
                <Button
                    icon="pi pi-fw pi-home"
                    onClick={() => handleNavigation('/')}
                    style={{top:'1%', width: '100%', marginBottom: '1em', backgroundColor: buttonColor, borderColor: buttonBorderColor, borderRadius: '10px'}}
                >
                    {isSidebarExpanded && "Inicio"}
                </Button>
                <Button
                    icon="pi pi-fw pi-bolt"
                    onClick={() => handleNavigation('/parteUno')}
                    style={{top:'1%',width: '100%', marginBottom: '1em', backgroundColor: buttonColor, borderColor: buttonBorderColor, borderRadius: '10px'}}
                >
                    {isSidebarExpanded && "Cálculo de Rayos de Contexto"}
                </Button>
                <Button
                    icon="pi pi-fw pi-chart-bar"
                    onClick={() => handleNavigation('/parteDos')}
                    style={{top:'1%',width: '100%', marginBottom: '1em', backgroundColor: buttonColor, borderColor: buttonBorderColor, borderRadius: '10px'}}
                >
                    {isSidebarExpanded && "Evaluación de capas"}
                </Button>
                <Button
                    icon="pi pi-fw pi-server"
                    style={{top:'1%',width: '100%', marginBottom: '1em', backgroundColor: buttonColor, borderColor: buttonBorderColor, borderRadius: '10px'}}
                    disabled
                >
                    {isSidebarExpanded && "Parte 1"}
                </Button>
                <div style={{width: '100%', margin: '0.8em', color: connected ? 'green' : 'red', display: 'flex', alignItems: 'center'}}>
                    <i className={`pi ${connected ? 'pi-check' : 'pi-times'}`} style={{fontSize: '1em', marginRight: '.5em'}}></i>
                    {isSidebarExpanded && (connected ? '¡Estás conectado!' : '¡Ups! Parece que estás desconectado.')}
                </div>
                <Button
                    icon={isSidebarExpanded ? "pi pi-angle-left" : "pi pi-angle-right"}
                    onClick={() => onToggleSidebar(!isSidebarExpanded)}
                    style={{position: 'absolute', bottom: '1em', left: '50%', transform: 'translateX(-50%)', backgroundColor: buttonColor, borderColor: buttonBorderColor, borderRadius: '50%'}}
                />
            </Sidebar>
        </>
    );
};

export default Navbar;
