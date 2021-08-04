let tools = {
    enter_value: (d) => {
        let salida = "name"
        if ((parseInt(d) + "") == (d + "")) {
            salida = "int"
        } else if ((parseFloat(d) + "") == (d + "")) {
            salida = "float"
        }



        return [d, salida]
    },
    set_v: (v, i) => {
        let salida

        let k = tools.enter_value(v);

        if (k[1] == "name") {
            salida = tools.tipos.name(v, i, false)
        } else {
            salida = tools.tipos.value(v, k[1], i, "")

        }

        return salida
    },
    chars: {
        N: "\n",
        T: "\t",
        B: "\b",
        BAR: "\\",
        COMILLAS: "\"",
        APOSTROFE: "\'",
        R: "\r",
    },
    tokens: {
        ope: ["+", "-", "<", ">", "/", "^", "*", "$", "@", "%", "|", "!", "=", ":"],
        sim: ["(", ")", "{", "}", "[", "]", ","]
    },
    tipos: {
        name: (n, i, s) => {
            return {
                name: n,
                i: i,
                notmod: s || false,
                tipo: "name"
            }
        },
        ope: (c, i, con) => {
            return {
                char: c,
                i: i,
                cond: con || false,
                tipo: "ope"
            }
        },
        sim: (c, i) => {
            return {
                char: c,
                i: i,
                tipo: "sim"

            }
        },
        value: (v, t, i, b) => {
            return {
                value: v,
                type: t,
                i: i,
                byte: b || "",
                tipo: "value"
            }
        }
    },
    paren: {
        tupla:(c, i) => {

            return {
                tipo:"()",
                i:i,
                data:c[0]||[],
                complet:c
            }
        },
        list:(c, i) => {

            return {
                tipo:"[]",
                i:i,
                data:c[0]||[],
                complet:c
            }
        },
        code:(c, i) => {

            return {
                tipo:"{}",
                i:i,
                data:c,
                one:c[0]||[]
            }
        }
    },
    errores:{
        ValueError:"ValueError",
        SyntaxError:"SyntaxError"
    }
}

let APP = () => {

    let me = {
        origin: "",
        code: "",
        errores: [],
        desline: (c, o) => {
            me.origin = o;
            me.code = c;
            let cadena = "";
            let linea = []
            let salida = []
            let modo = "normal"
            let byte = "";
            let str_l = ""


            for (let i = 0; i < c.length; i++) {
                const e = c[i] + "";

                if (modo === "normal") {
                    if (e != " ") {
                        if (tools.tokens.ope.includes(e)) {

                            if (cadena != "") {
                                linea.push(tools.set_v(cadena, i-cadena.length));
                                cadena = ""
                            }

                            if ((linea[linea.length - 1] || {}).tipo === "ope") {
                                let w = linea[linea.length - 1].char;

                                if (e.length != 1) {
                                    linea.pop()
                                    linea.push(tools.tipos.ope(e, i))

                                } else if ((["<", ">", "=", ":"].includes(w))) {
                                    if ((["<", ">", "=", ":"].includes(e))) {

                                        linea.push(tools.tipos.ope(w + e, i))

                                    } else {
                                        linea.push(tools.tipos.ope(e, i))
                                    }
                                } else {
                                    linea.push(tools.tipos.ope(e, i))

                                }

                            } else {
                                linea.push(tools.tipos.ope(e, i))

                            }



                        } else if (tools.tokens.sim.includes(e)) {
                            if (cadena != "") {
                                linea.push(tools.set_v(cadena, i-cadena.length));
                                cadena = ""
                            }

                            linea.push(tools.tipos.sim(e, i))

                        } else if (e == "#") {
                            if (cadena != "") {
                                linea.push(tools.set_v(cadena, i-cadena.length));
                                cadena = ""
                            }

                            modo = "com"

                        } else if (e == tools.chars.N | e == tools.chars.R) {
                            if (cadena != "") {
                                linea.push(tools.set_v(cadena, i-cadena.length));
                                cadena = ""
                            }
                            

                        } else if (e == ";") {
                            if (cadena != "") {
                                linea.push(tools.set_v(cadena, i-cadena.length));
                                cadena = ""
                            }

                            if (linea.length != 0) {
                                salida.push(linea)
                            }

                            linea = []
                            

                        } else if (e == "'" | e == '"') {
                            byte = cadena;
                            cadena = e


                            modo = "str"

                        } else {
                            cadena = cadena + e;
                        }
                    } else {
                        if (cadena != "") {
                            linea.push(tools.set_v(cadena, i));
                            cadena = ""
                        }
                    }

                } else if (modo === "str") {
                    if (e === cadena.substr(0, 1)) {
                        linea.push(
                            tools.tipos.value(
                                cadena + e, "str", i - cadena.length, byte
                            )
                        );
                        cadena = "";
                        modo = "normal"
                    } else if (e === tools.chars.N) {
                        cadena = cadena + ("" + tools.chars.BAR + "n")
                    } else {
                        cadena = cadena + e
                    }
                } else if (modo === "com") {
                    if (e == N) {
                        modo = "normal"
                    }
                }

            };

            if (cadena != "") {
                linea.push(tools.set_v(cadena, c.length-cadena.length));
                cadena = ""
            };

            if (linea.length != 0) {
                salida.push(linea)
            }


            
            return salida
        },
        error:(msg, type, i) => {

            me.errores.push(
                {msg:msg, type:type, i:i}
            );
            //throw {msg:msg, type:type, i:i}
            throw msg
        },
        parselex:(c) => {

            let modo = "normal";
            let iter = 0;
            let count = 0;
            let cadena = [];
            let salida = [];
            let linea = [];
            let subline = [];



            for (let i = 0; i < c.length; i++) {
                const e = c[i];

                for (let x = 0; x < e.length; x++) {
                    const q = e[x];
                    
                    if (modo == "normal") {

                        if (q.char == "(") {
                            iter = q.i;
                            cadena = [];
                            modo = "()";
                            count = 1;
                        } else if (q.char == "[") {
                            iter = q.i;
                            cadena = [];
                            modo = "[]";
                            count = 1;
                        } else if (q.char == "{") {
                            iter = q.i;
                            cadena = [];
                            modo = "{}";
                            count = 1;
                        } else if ([")", "]", "}"].includes(q.char)) {
                            //console.log(q)
                            me.error(`the token '${q.char}' is invalid`, tools.errores.SyntaxError, q.i)
                        } else {
                            linea.push(q)
                        }

                    } else if (modo == "()") {
                        if (q.char == ")") {

                            count--

                            if (count == 0) {
                                if (subline.length!=0) {
                                    cadena.push(subline);
                                    subline = [];
                                };

                                linea.push(tools.paren.tupla(me.parselex(cadena), iter))
                                cadena = [];
                                modo = "normal";


                            } else {
                                subline.push(q)
                                
                            }
                            
                        } else if (q.char == "(") {
                            count++;
                            subline.push(q)

                        } else {
                            subline.push(q)
                        }
                    } else if (modo == "[]") {
                        if (q.char == "]") {

                            count--

                            if (count == 0) {
                                if (subline.length!=0) {
                                    cadena.push(subline);
                                    subline = [];
                                };

                                linea.push(tools.paren.list(me.parselex(cadena), iter))
                                cadena = [];
                                modo = "normal";


                            } else {
                                subline.push(q)
                                
                            }
                            
                        } else if (q.char == "[") {
                            count++;
                            subline.push(q)

                        } else {
                            subline.push(q)
                        }
                    } else if (modo == "{}") {
                        if (q.char == "}") {

                            count--

                            if (count == 0) {
                                if (subline.length!=0) {
                                    cadena.push(subline);
                                    subline = [];
                                };

                                linea.push(tools.paren.code(me.parselex(cadena), iter))
                                cadena = [];
                                modo = "normal";


                            } else {
                                subline.push(q)
                                
                            }
                            
                        } else if (q.char == "{") {
                            count++;
                            subline.push(q)

                        } else {
                            subline.push(q)
                        }
                    }
                }

                if (modo == "normal") {
                    if (linea.length!=0) {
                        salida.push(linea)
                        linea = []
                    }
                } else {
                    if (subline.length!=0) {
                        cadena.push(subline)
                        subline = []
                    }
                }
                
            }

            if (linea.length!=0) {
                salida.push(linea)
                linea = []
            }

            if (modo != "normal") {
                me.error(`the token '${me.code[iter]}' is invalid`, tools.errores.SyntaxError, iter)
                
            }

            return salida

        },
        estructuration:(c, func) => {

        }
    };

    return me

};



try {
    module.exports.clsapp = APP
} catch (e) {
    let app = APP();
}