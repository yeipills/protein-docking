import React, { useEffect, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { TabMenu } from "primereact/tabmenu";
import { classNames } from "primereact/utils";

const Navbar = ({ connected = false, setSelectedRoute }) => {
    const [activeIndex, setActiveIndex] = useState(0);
    const navigate = useNavigate();
    const location = useLocation();

    useEffect(() => {
        if (location.pathname === "/") {
            setActiveIndex(0);
        }
        if (location.pathname === "/parteUno") {
            setActiveIndex(1);
        }
        if (location.pathname === "/parteDos") {
            setActiveIndex(2);
        }
    }, [location]);

    const items = [
        {
            label: "Inicio",
            icon: "pi pi-fw pi-home",
            command: () => {
                navigate("/");
            },
        },
        {
            label: "Cálculo de Rayos de Contexto",
            icon: "pi pi-fw pi-bolt",
            command: () => {
                navigate("parteUno");
            },
        },
        {
            label: "Evaluación de capas",
            icon: "pi pi-fw pi-chart-bar",
            command: () => {
                navigate("parteDos");
            },
        },
        {
            label: "Parte 1",
            icon: "pi pi-fw pi-server",
            disabled: true,
            style: { marginLeft: "auto" },
            template: (item, options) => {
                return (
                    <>
                        <div className={options.className} target={item.target}>
                            <span className={options.labelClassName}>
                                <span
                                    className={classNames(
                                        options.iconClassName,
                                        "pi pi-server"
                                    )}
                                ></span>
                                <span
                                    style={{
                                        color: connected ? "green" : "red",
                                    }}
                                    className={options.labelClassName}
                                >
                                    {connected ? "Conectado" : "No conectado"}
                                </span>
                            </span>
                        </div>
                    </>
                );
            },
        },
    ];
    return (
        <>
            <TabMenu
                model={items}
                activeIndex={activeIndex}
                onTabChange={(e) => setActiveIndex(e.index)}
            />
        </>
    );
};

export default Navbar;
