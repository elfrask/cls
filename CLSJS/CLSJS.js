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
            "if", "while", "for", "using", "var", "as", "try"
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

            if (ee === undefined) return false
            
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
    mulstr:(e, i) => {
        let sal = ""
        for (let x = 0; x < i; x++) {
            sal = sal + e
            
        };
        return sal
    }
}

let APP = () => {
    let asi = (e, t) => Object.assign(e, t)
    
    let Api = {
        Module:asi(() => {
            return {
                __class__:"Module",
                __clase__:Api.Module
            }
        }, {
            __class__:"Module",
        })
    }


    let me = {
        origin: "",
        code: "",
        stadef:"Any",
        errores: [],
        namespace:"std",
        desline: (c, o) => {
            me.origin = o||"<CLS:Stdin>";
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
            let sta = [];
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
                            if (sta.length===0) sta = [me.stadef]
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
                        if (sta.length===0) sta = [me.stadef]

                            
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
            let salida = [];
            let cadena = [];


            for (let i = 0; i < c.length; i++) {
                let e = c[i];

                cadena = [];

                for (let x = 0; x < e.length; x++) {
                    let ele = e[x];

                    if (ele.tipo == "()") {
                        cadena.push(
                            tools.paren.tupla(
                                me.estructuration_one(ele.complet), ele.i
                            )
                        )
                    }  else if (ele.tipo == "[]") {
                        cadena.push(
                            tools.paren.list(
                                me.estructuration_one(ele.complet), ele.i
                            )
                        )
                    }  else if (ele.tipo == "code") {
                        cadena.push(
                            tools.paren.code(
                                me.estructuration_one(ele.data), ele.i
                            )
                        );
                        let func = cadena.slice(-4);
                        let func3 = cadena.slice(-3);
                        let isfunc = false;
                        let code = [];
                        let sta = me.stadef;
                        let arg = [];


                        if (tools.compare(["()", {tipo:"ope", char:"->"}, "code"], func3)) {
                            
                            code = func3[2];
                            arg = func3[0];
                            isfunc = true;

                            cadena.pop();
                            cadena.pop();
                            cadena.pop();


                        } else if (tools.compare(["()", {tipo:"ope", char:"->"}, "name", "code"], func)) {
                            
                            code = func[3];
                            sta = func[2].name;
                            arg = func[0];
                            isfunc = true;

                            
                            cadena.pop();
                            cadena.pop();
                            cadena.pop();
                            cadena.pop();
                            
                        };

                        if (isfunc) {
                            cadena.push({
                                tipo:"func-a",
                                arg:me.parsearg(arg.data),
                                i:arg.i,
                                sta:sta,
                                code:me.estructuration(code.data)
                            })
                        };

                        delete func, func3;
                        

                    }  else {
                        cadena.push(ele)
                    }
                    
                };

                salida.push(cadena)

            }

            return salida
        },
        estructuration:(c) => {

            let salida = []

            let var_names = []
            
            function gen_eval(c, i) {
                return {
                    i:i,
                    eval:c,
                    tipo:"exec"
                }
            }
            function isdim(c) {

                for (let i = 0; i < c.length; i++) {
                    const e = c[i];

                    if (e.tipo == "ope") if (e.char == "=") return[true, i]
                    
                }
                return [false, 0]
            }
            function ex_names(v=[]) {
                for (let i = 0; i < v.names.length; i++) {
                    const ele = v.names[i];
                    if (!var_names.includes(ele)) {
                        var_names.push(ele);
                    }
                    
                }
                v.names=[];
                return v
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
                            } else if (tools.compare(["name", "name"], e)) {
                                let mit = e;
                                e = [
                                    tools.tipos.name("var", e[0].i),
                                    e[1],
                                    tools.tipos.ope(":", e[1].i),
                                    e[1]
                                ];

                                if ( (mit.length === 4) & ( (mit[2]||{}).char === "=" )) {
                                    mit.slice(2).forEach(g=>e.push(g))
                                    
                                    
                                } else if ( (mit[2]||{}).char === "=" ) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                    
                                }
                            }
                        };

                        
                        let [is_dim, dim_le] = isdim(e);

                        //console.log(e)
                        //console.log(e[0] == "if")

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

                            if (!var_names.includes(nombre)) {
                                var_names.push(nombre)
                            }

                            salida.push({
                                name:nombre,
                                sta:sta,
                                code:code,
                                arg:arg,
                                visible:visible,
                                tipo:"func-def",
                                async:asyncrono,
                                i:e[0].i
                            })
                        } else if (e[0].name == "class") {
                            let nombre = "";
                            let code = [];
                            let arg = [];

                            if (tools.compare(["name", "name", "()", "code"], e)) {

                                if (e.length !== 4) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[4].i);
                                };

                                nombre = e[1].name;
                                arg = me.parsearg(e[2].data).map(e => e.arg);
                                code = me.estructuration(e[3].data);

                            } else if (tools.compare(["name", "name", "code"], e)) {

                                if (e.length !== 3) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[3].i);
                                };

                                nombre = e[1].name;
                                code = me.estructuration(e[2].data);

                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i);
                            };
                            if (!var_names.includes(nombre)) {
                                var_names.push(nombre)
                            }

                            salida.push({
                                tipo:"class-def",
                                name:nombre,
                                code:code,
                                i:e[0].i,
                                arg:arg,
                                visible:visible
                            });


                        } else if (e[0].name == "module") {
                            let nombre = "";
                            let code = [];
                            

                            if (tools.compare(["name", "name", "code"], e)) {

                                if (e.length !== 3) {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[3].i);
                                };

                                nombre = e[1].name;
                                code = me.estructuration(e[2].data);

                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i);
                            };

                            if (!var_names.includes(nombre)) {
                                var_names.push(nombre)
                            }

                            salida.push({
                                tipo:"module-def",
                                code:code,
                                name:nombre,
                                i:e[0].i,
                                visible:visible
                            });


                        } else if (e[0].name == "if") {
                            //console.log("llego el if")
                            let ceo = e.slice(3);
                            if (e.length < 2) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                            let out = [{
                                to:"if",
                                cond:me.estructuration_one(e[1].data),
                                code:ex_names(me.estructuration(e[2].data)),
                            }];

                            for (let x = 0; x < ceo.length; x=x+3) {
                                let xx = ceo[x];
                                
                                if (["elif", "elseif"].includes(xx.name)) {
                                    if (ceo.slice(x).length <3) {
                                        me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                    }

                                    out.push({
                                        to:"elif",
                                        cond:me.estructuration_one(e[1+x].data),
                                        code:ex_names(me.estructuration(e[2+x].data)),
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
                                                cond:me.estructuration_one(e[1+x].data),
                                                code:ex_names(me.estructuration(e[2+x].data)),
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
                                            code:ex_names(me.estructuration(e[1+x].data)),
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

                        } else if (e[0].name == "while") {
                            
                            if (e.length < 3) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (e.length === 3) {
                                //me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                if (tools.compare(["name", "()", "code"], e)) {
                                    salida.push({
                                        tipo:"while-def",
                                        cond:e[1].data,
                                        code:ex_names(me.estructuration(e[2].data)),
                                        i:e[0].i
                                    });
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            }
                            
                        } else if (e[0].name == "for") {
                            
                            if (e.length < 3) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                            if (e.length === 3) {
                                //me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                if (tools.compare(["name", "()", "code"], e)) {
                                    salida.push({
                                        tipo:"for-def",
                                        data:e[1].complet,
                                        code:ex_names(me.estructuration(e[2].data)),
                                        i:e[0].i
                                    })
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else if (e.length === 5) {
                                //me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                if (tools.compare(["name", {name:"each", tipo:"name"}, "name", "()", "code"], e)) {
                                    salida.push({
                                        tipo:"for-each-def",
                                        iterator:e[2].name,
                                        data:e[3].data,
                                        code:ex_names(me.estructuration(e[4].data)),
                                        i:e[0].i
                                    })
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "with") {
                            if (e.length < 4) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (e.length === 4) {
                                if (tools.compare(["name", "name", "()", "code"], e)) {
                                    salida.push({
                                        tipo:"with-def",
                                        value:e[2].data,
                                        name:e[1].name,
                                        code:ex_names(me.estructuration(e[2].data)),
                                        i:e[0].i
                                    });
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "import") {
                            if (e.length < 4) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (e.length === 4) {
                                if (tools.compare(
                                    ["name", {tipo:"value", type:"str"}, {tipo:"name", name:"as"}, "name"], 
                                    e)) {
                                    
                                    if (!var_names.includes(e[3].name)) {
                                        var_names.push(e[3].name)
                                    }
                                    salida.push({
                                        tipo:"import-def",
                                        as:e[3].name,
                                        lib:eval(e[1].value),
                                        i:e[0].i
                                    });
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "from") {
                            if (e.length < 6) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (e.length === 6) {
                                if (tools.compare(
                                    ["name", {tipo:"value", type:"str"}, {tipo:"name", name:"import"}, "name", {tipo:"name", name:"as"}, "name"], 
                                    e)) {
                                    if (!var_names.includes(e[5].name)) {
                                        var_names.push(e[5].name)
                                    }
                                    salida.push({
                                        tipo:"from-def",
                                        as:e[5].name,
                                        import:e[3].name,
                                        lib:eval(e[1].value),
                                        i:e[0].i
                                    });
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "include") {
                            if (e.length < 2) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (e.length === 2) {
                                if (tools.compare(
                                    ["name", {tipo:"value", type:"str"}], 
                                    e)) {
                                    salida.push({
                                        tipo:"include-def",
                                        lib:eval(e[1].value),
                                        i:e[0].i
                                    });
                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "try") {
                            if (e.length < 2) {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }

                            if (tools.compare(["name", "code"], e)) {
                                e.push(
                                    tools.tipos.name("catch", e[0].i),
                                    tools.tipos.name("e", e[0].i),
                                    tools.paren.code([], e[0].i)
                                )
                            }

                            if (e.length === 5) {
                                if (tools.compare(["name", "code", "name", "name", "code"], e)) {
                                    
                                    if (["catch", "error", "except", "cratch", "fatal"].includes(e[2].name)) {
                                        salida.push({
                                            tipo:"try-def",
                                            i:e[0].i,
                                            e_name:e[3].name,
                                            try_code:ex_names(me.estructuration(e[1].data)),
                                            e_code:ex_names(me.estructuration(e[4].data))
                                        });
                                    } else {
                                        me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                        
                                    }
                                    

                                } else {
                                    me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                };
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            };
                        } else if (e[0].name == "using") {
                            if (tools.compare(["name", {tipo:"name", name:"namespace"}, "name"], e)) {
                                salida.push({
                                    i:e[0].i,
                                    tipo:"namespace-comp",
                                    name:e[2].name
                                })
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                        } else if (e[0].name == "var") {
                            if (e.length > 1) {
                                let te
                                if (e[1].tipo == "()") {
                                    te = me.parsearg(e[1].data)
                                } else {
                                    te = me.parsearg(e.slice(1))
                                }

                                te.forEach(e => {
                                    if (!var_names.includes(e.arg)) {
                                        var_names.push(e.arg)
                                    }
                                })

                                salida.push({
                                    i:e[0].i,
                                    tipo:"var-def",
                                    vals:te,
                                    visible:visible
                                })
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                            }
                        } else if (is_dim) {

                            if (dim_le!==0) {
                                if (dim_le === 1 & (!e[0].name.includes("."))) {
                                    if (!var_names.includes(e[0].name)) {
                                        //console.log("llego", e[0])
                                        var_names.push(e[0].name)
                                    }
                                };
                                salida.push(
                                    {
                                        tipo:"var",
                                        i:e[0].i,
                                        
                                        var:e.slice(0, dim_le),
                                        eval:e.slice(dim_le+1),
                                        one:(dim_le === 1) & (!e[0].name.includes(".")),

                                    }
                                )
                            } else {
                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                
                            }
                        } else {
                            salida.push(gen_eval(e, e[0].i))
                        }
                        
                    } else {
                        salida.push(gen_eval(e, e[0].i))
                    }

                } else {
                    //salida.push(gen_eval(e, e[0].i))
                }
                
            };

            salida.names = var_names;

            delete var_names
            
            
            return salida;
        },
        generator:(code, mode) => {
            
            let salida = [];
            
            
            if (code.names !== undefined) {
                code.names.forEach(e=>{
                    //console.log(e)
                    salida.push(
                        `let var_${e}`
                    )
                })
            }

            function error_p(c, i) {
                return[
                    "try {",
                        c,
                    "} catch (e) {",
                    `    app.error(e, 'ErrorExec', ${i})`,
                    "}"
                ]
            }

            for (let i = 0; i < code.length; i++) {
                let e = code[i];

                if ((e.tipo == "func-def") & ["normal", "func", "class", "module"].includes(mode)) {
                    let iter = -1;
                    let asyncrono = "";
                    if (e.async) asyncrono = "async"
                    
                    let qqq = "";

                    let name_var = `var_${e.name}`;

                    if (["class", "module"].includes(mode)) {
                        name_var = `me.${e.name}`;

                        if (e.visible=="private") {
                            name_var = `private.${e.name}`;

                        }
                        
                        if (mode == "class") if (e.visible=="export") {
                            name_var = `me.${e.name}`;
                            qqq = `export.push('${e.name}')`
                        } else if (e.visible=="static") {
                            name_var = `out.${e.name}`;

                        }
                    }


                    salida.push(
                        `${name_var} = ( ${asyncrono} (...arg) => {`,
                        `    let este = ${name_var}`,
                        `    let rt = var_${e.sta}`,

                        ...e.arg.map((e)=>{
                            iter++;
                            let tto = me.generator_one(e.def)
                            if (tto === "") tto = undefined+""
                            return([
                                `let var_${e.arg} = (arg[${iter}]||(${tto}))`,
                                `var_${e.arg} = app.dim(var_${e.arg}, var_${e.sta[0]})`
                            ])
                        }),

                        me.generator(e.code, "func"),

                        `});` + qqq
                    )
                    

                } else if ((e.tipo == "class-def") & ["normal", "func", "module"].includes(mode)) {
                    let name_var = "var_" + e.name;
                    if ("module" === mode) {
                        name_var = `me.${e.name}`;

                        if (e.visible=="private") {
                            name_var = `private.${e.name}`;

                        }
                    }
                    
                    salida.push(
                        `${name_var} = (...arg_c) => {`,
                        `    let este = ${name_var};`,
                        `    let var_me = {};`,
                        `    let var_private = {};`,
                        `    let exportar = [];`,
                        `    let out = {__clase__:"${name_var}"};`,
                        `    let me = var_me;`,
                        `    let private = var_private;`,
                        `    app.clasi(me, out, private, exportar, [${e.arg.map(e=>("var_"+e+", "))}]);`,
                        
                        me.generator(e.code, "class"),

                        "    me.__export__ = exportar;",
                        "    out.__export__ = exportar;",
                        "    out.default = (me.default||(()=>{}));",
                        
                        "    let n = undefined",
                        "    me.default = undefined",
                        "    if (este.__pre__==='done') (me.main||(()=>{}))(...arg_c);",
                        "    else {este.__pre__='done'; Object.assign(este, me); {var_me, var_private} = {var_me:1, var_private:1} }",
                        
                        "    return me",
                        `}`
                    )
                } else if ((e.tipo == "module-def") & ["normal", "func", "module"].includes(mode)) {
                    let name_var = "var_" + e.name;
                    if ("module" === mode) {
                        name_var = `me.${e.name}`;

                        if (e.visible=="private") {
                            name_var = `private.${e.name}`;

                        }
                    }
                    
                    salida.push(
                        `${name_var} = (...arg_c) => {`,
                        `    let este = ${name_var};`,
                        `    let var_me = {};`,
                        `    let var_private = {};`,
                        `    if (!este.__class__)Object.assing(este, {__class__:"Module", __clase__:Api.Module});`,
                        `    let me = var_me;`,
                        `    let private = var_private;`,
                        
                        me.generator(e.code, "module"),

                        "    return me",
                        `}`
                    )
                } else if ((e.tipo == "exec") & ["normal", "func"].includes(mode)) {
                    salida.push(
                        ...error_p(me.generator_one(e.eval, e.i, "exec"))
                    )
                }
                
            }


            return salida;
        },
        generator_one:(code, mode, modei)=>{
            let salida = "";
            mode= mode||"normal";
            modei= modei||"eval";
            let last = {tipo:"none", i:0}

            for (let i = 0; i < code.length; i++) {
                let e = code[i];

                if (e.tipo === "name") {
                    if (["or", "and"].includes(e.name)) {
                        salida = salida + ` ${
                            ({
                                or:"|", 
                                and:"&",
                            })[e.name]
                        } `;
                        
                    } else {
                        salida = salida + ` var_${e.name} `;
                        
                    }
                } else if (e.tipo === "value") {

                    if (e.type === "str") {
                        salida = salida + ` app.tstr['${e.byte}'](${e.value}) `;
                    } else if (e.type === "int") {
                        salida = salida + ` Api.int(${e.value}) `;
                    } else if (e.type === "float") {
                        salida = salida + ` Api.float(${e.value}) `;
                    }
                    
                } else if (e.tipo === "()") {
                    salida = salida + ` (${me.generator_one(e.data, mode)}) `;
                }
                
            }

            return salida
        },
        jump:(c=[], i) => {
            i = i||0
            let salida = "";

            c.forEach(e=>{
                if (typeof(e)=="string") {
                    salida = salida + tools.chars.N + tools.mulstr("    ", i) + e;

                } else {
                    salida = salida + tools.chars.N + me.jump(e, i+1) + tools.chars.N;
                    
                }
            })
            return salida
        }
    };

    return me

};



try {
    module.exports.clsapp = APP
} catch (e) {
    let app = APP();
}