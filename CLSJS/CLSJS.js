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
    nombres: {
        reservados:[
            "class", "function", "async", "def", "method", "func", "fub", "module", "namespace",
            "sync", "with", "import", "private", "public", "export", "static", "from", "include",
            "if", "while", "for"
        ],
        visible:[
            "private", "public", "export", "static"
        ],
        async:[
            "async", "sync"
        ],
        
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
                tipo:"code",
                i:i,
                data:c,
                one:c[0]||[]
            }
        }
    },
    errores:{
        ValueError:"ValueError",
        SyntaxError:"SyntaxError"
    },
    compare:(c = [], k = []) => {
        if (c.length < k.length) return false

        let salida = true;
        for (let i = 0; i < c.length; i++) {
            let e = c[i];
            let ee = k[i];
            
            if (typeof(e) == "string") {
                if (e !== ee.tipo) {
                    salida = false
                }
            } else {
                if (e.tipo === ee.tipo) {
                    let c = Object.keys(e);
                    for (let x = 0; x < c.length; x++) {
                        const xx = c[x];

                        if (e[xx] !== ee[xx]) {
                            salida = false
                        }
                        
                    }

                } else{
                    salida = false
                }
            }
            
        }
        return salida
    },
}

let APP = () => {

    let me = {
        origin: "",
        code: "",
        stadef:"Any",
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
                                    linea.push(tools.tipos.ope(w+e, i))

                                } else if ((["<", ">", "=", ":", "-"].includes(w))) {
                                    if ((["<", ">", "=", ":"].includes(e))) {
                                        linea.pop()
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
        parsearg:(c) => {


            let salida = []

            let arg = "";
            let sta = [me.stadef];
            let def = [];

            let modo = "normal";

            for (let i = 0; i < c.length; i++) {
                let e= c[i];

                if (modo == "normal") {
                    if (e.tipo == "name") {
                        arg = e.name;
                        modo = "coma";
                    } else {
                        me.error("Syntax error", tools.errores.SyntaxError, e.i)
                    }
                } else if (modo == "coma") {
                    if (["sim", "ope"].includes(e.tipo)) {
                        if ([":", "->"].includes(e.char) & sta.length == 0) {
                            modo = "sta";
                        } else if (e.char == "=" & def.length == 0) {
                            modo = "def";
                        } else if (e.char == ",") {
                            
                            salida.push({
                                arg:arg,
                                sta:sta,
                                def:def
                            })

                            let arg = "";
                            let sta = [me.stadef];
                            let def = [];
                            modo = "normal"
                        }
                        

                    } else {
                        me.error("Syntax error", tools.errores.SyntaxError, e.i)
                    }
                } else if (modo == "sta") {
                    if (e.tipo == "name") {
                        sta = [e.name];
                        modo = "coma";
                    } else {
                        me.error("Syntax error", tools.errores.SyntaxError, e.i)
                    }
                } else if (modo == "def") {
                    if (e.char == ",") {
                            
                        salida.push({
                            arg:arg,
                            sta:sta,
                            def:def
                        })

                        let arg = "";
                        let sta = [me.stadef];
                        let def = [];
                        modo = "normal"
                    } else {
                        def.push(e)
                    }
                } else {
                    
                }
                
            };

            if (arg !== "") {
                            
                salida.push({
                    arg:arg,
                    sta:sta,
                    def:def
                })
            }

            return salida
        },
        estructuration_one:(c) => {
            
        },
        estructuration:(c) => {

            let salida = []

            function gen_eval(c, i) {
                return{
                    tipo:"eval",
                    i:i,
                    code:c
                }
            }

            for (let i = 0; i < c.length; i++) {
                let e = c[i];
                let visible = "public";
                let asyncrono = false;

                if (e.length > 0) {
                    
                    if (e[0].tipo == "name") {

                        if (tools.nombres.visible.includes(e[0].name)) {
                            visible = e[0].name;
                            e = e.slice(1)
                        };
                        if (tools.nombres.async.includes(e[0].name)) {
                            asyncrono = e[0].name == "async";
                            e = e.slice(1)
                        };
                        if (!tools.nombres.reservados.includes(e[0].name)) {
                            asyncrono = e[0].name == "async";
                            if (tools.compare(["name", "name", "()", "code"], e)) {
                                e = [
                                    tools.tipos.name("function", e[0].i),
                                    e[1],
                                    e[2],
                                    tools.tipos.ope("->", e[3].i, false),
                                    e[0],
                                    e[3],
                                ]
                            }
                        };
                        
                        

                        if (["func", "function", "method", "fub", "def"].includes(e[0].name)) {
                            let nombre = "";
                            let sta = me.stadef;
                            let code = [];
                            let arg = [];
                            if (tools.compare(["name", "name", "()", "code"], e)) {
                                if (e.length !== 4) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[4].i)
                                };
                                nombre = e[1].name;
                                code = me.estructuration(e[3].data);
                                arg = me.parsearg(e[2].data);
                            } else if (tools.compare(["name", "name", "()", {char:"->", tipo:"ope"}, "name", "code"], e)) {
                                if (e.length !== 6) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[6].i)
                                };
                                nombre = e[1].name;
                                sta = e[4].name;
                                code = me.estructuration(e[5].data);
                                arg = me.parsearg(e[2].data);
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            salida.push({
                                name:nombre,
                                sta:sta,
                                code:code,
                                arg:arg,
                                visible:visible,
                                tipo:"func-def",
                                async:asyncrono
                            })
                        } else if (e[0] == "if") {

                            let ceo = e.slice(3);
                            if (e.slice(x).length < 2) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                            let out = [{
                                to:"if",
                                cond:e[1].data,
                                code:e[2].data,
                            }];

                            for (let x = 0; x < ceo.length; x=x+3) {
                                let xx = ceo[x];
                                
                                if (["elif", "elseif"].includes(xx.name)) {
                                    if (ceo.slice(x).length <3) {
                                        me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                    }

                                    out.push({
                                        to:"elif",
                                        cond:e[1+x].data,
                                        code:e[2+x].data,
                                    })
                                    
                                } else if (["else"].includes(xx.name)) {
                                    if (ceo.slice(x).length < 2) {
                                        me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                    }

                                    if (ceo[x+1].tipo == "name") {
                                        x++;

                                        if (ceo.slice(x).length < 3) {
                                            me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                        }
                                        
                                        if (["if"].includes(xx.name)) {
    
                                            out.push({
                                                to:"elif",
                                                cond:e[1+x].data,
                                                code:e[2+x].data,
                                            })
                                            
                                        } else {
                                            me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                        }
                                    } else if (ceo[x+1].tipo == "code") {
                                        if (ceo.slice(x).length < 2) {
                                            me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                        }

                                        out.push({
                                            to:"else",
                                            code:e[1+x].data,
                                        })
                                    } else {
                                        me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                    }
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                }
                            }

                            salida.push({
                                tipo:"if-def",
                                list:out
                            })

                        } else if (e[0] == "while") {
                            let ceo = e.slice(3);
                            if (e.length < 3) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                            salida.push({
                                tipo:"while-def",
                                cond:e[1].data,
                                code:me.estructuration(e[2].data),
                            });
                        } else if (e[0] == "for") {
                            let ceo = e.slice(3);
                            if (e.length === 3) {
                                //me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                salida.push({
                                    tipo:"for-def",
                                    data:e[1].complet,
                                    code:e[2].data,
                                });
                            };
                        }
                        
                    } else {
                        salida.push(gen_eval(e, e[0].i))
                    }

                } else {
                    salida.push(gen_eval(e, e[0].i))
                }
                
            }

            return salida;
        }
    };

    return me

};



try {
    module.exports.clsapp = APP
} catch (e) {
    let app = APP();
}