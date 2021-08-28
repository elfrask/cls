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
                i: i - n.length,
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
        },
        cml:(c, i) => {
            return {
                data:c,
                i:i,
                tipo:"cml"
            }
        }
    },
    errores: {
        ValueError:"ValueError",
        SyntaxError:"SyntaxError",
        TypingError:"TypingError",
        ModuleError:"ModuleError",
    },
    compare:(c = [], k = []) => {
        if (c.length < k.length) {
            
            return false
        }

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
    },
    replace:(cadena, viejo, nuevo) => {
        while (cadena.includes(viejo)) {
            cadena.replace(viejo, nuevo)
        };
        return cadena
    },
    cml:{
        cmlobj:(data) => {
            function getobj(ikd, d) {
                let salida = null;
        
                for (let i = 0; i < d.length; i++) {
                    const e = d[i];
                    if (typeof(e) == "object") {
                        if (e.arg.id == ikd) {
                            return e;
                        } else {
                            let k= getobj(ikd, e.nodes);
                            if ((k+"") != "null") {
                                return k;
                            }
                        }
                    }
                }
        
                return salida
            };
            function replace_data(d, viejo, nuevo) {
                let salida = [];
                
                if (!Array.isArray(d)) {
                    d= [d]
                }
        
                for (let i = 0; i < d.length; i++) {
                    let e = d[i];
                    
                    if (typeof(e) == "object") {
                        let carg = Object.keys(e.arg);
                        for (let x = 0; x < carg.length; x++) {
                            const ele = e.arg[carg[x]];
                            if (nuevo.__classname__ == "CML") {
                                
                                e.arg[carg[x]] = tools.replace(ele, viejo, nuevo.getString())
                            } else {
                                
                                e.arg[carg[x]] = tools.replace(ele, viejo, nuevo.toString())
                            }
        
                        };
                        if (nuevo.__classname__ == "CML") {
                            e.inner = tools.replace(e.inner, viejo, nuevo.getString());
                        } else {
                            e.inner = tools.replace(e.inner, viejo, nuevo.toString());                   
                        }
                        let ik = replace_data(JSON.parse(JSON.stringify(e.nodes)), viejo, nuevo);
                        e.nodes = ik
                        salida.push(e)
                    } else if (typeof(e) == "string") {
                        if (typeof(nuevo) == "string") {
                            let m = tools.replace(e, viejo, nuevo);
                            salida.push(m)
                        } else if (typeof(nuevo) == "object") {
                            
                            if (nuevo.__classname__ == "CML") {
                                let cadena = "";
                                for (let y = 0; y < e.length; y++) {
                                    let elemento = e[y];
                                    
                                    if (cadena.substr(cadena.length-viejo.length) == viejo) {
                                        cadena =cadena.substr(0, cadena.length-viejo.length)
                                        
                                        if (cadena != "") {
                                            salida.push(cadena);
                                            cadena = "";    
                                        };
                                        
                                        
                                    
                                        let datossi= nuevo.node();
                                        
                                        for (let k = 0; k < datossi.length; k++) {
                                            const t = datossi[k];
                                            salida.push(t)
                                            
                                        }
                                    
                                        cadena = "";
                                    } else {
                                        cadena = cadena + elemento
                                        //console.log(cadena)
                                    }
                                };
            
                                //ultima verificacion
                                if (cadena.substr(cadena.length-viejo.length) == viejo) {
                                    cadena =cadena.substr(0, cadena.length-viejo.length)
                                    
                                    if (cadena != "") {
                                        salida.push(cadena);
                                        cadena = "";    
                                    }
                                    
                                    
                                    if (nuevo.__classname__ == "CML") {
                                        let datossi= nuevo.node();
                                        
                                        for (let k = 0; k < datossi.length; k++) {
                                            const t = datossi[k];
                                            salida.push(t)
                                            
                                        }
                                    }
                                    cadena = "";
                                };
        
                                if (cadena != "") {
                                    salida.push(cadena);
                                    cadena = "";    
                                };
                                
                            } else {
                                
                                let m = tools.replace(e, viejo, nuevo.toString());
                                
                                salida.push(m);
                                
                            }
                            
                            
                        } else {
                            let m = tools.replace(e, viejo, nuevo);
                            salida.push(m)
                        }
                    }
        
                };
        
        
                return salida
            };
            function tostr_data(d) {
                let salida = "";
        
                for (let i = 0; i < d.length; i++) {
                    const e = d[i];
                    
                    if (typeof(e) == "string") {
                        salida = salida + e
                    } else if (typeof(e) == "object") {
                        
                        let arg = "";
                        let carg = Object.entries(e.arg);
                        for (let x = 0; x < carg.length; x++) {
                            const y = carg[x];
                            arg= arg + y[0] + '="' + y[1] + '" '
                        }
                        let io=""
                        if (arg != "") {
                            io=" "
                        }
        
                        salida = salida + `<${e.tag}${io}${arg}>${tostr_data(e.nodes)}</${e.tag}>`
                    }
                }
        
                return salida;
            }
            
            let xml = data;
            let me = {
                __classname__:"CML",
                node:() => {return xml},
                toString:() => {return("[CML: Object]")},
                getobj:(id) => {
                    return cmlobj(getobj(id, xml))
                },
                format:(datos) => {
                    let ma = JSON.parse(JSON.stringify(xml))
                    let arreglo = Object.entries(datos);
        
                    for (let i = 0; i < arreglo.length; i++) {
                        const e = arreglo[i];
                        if (e[0] == "__classname__") {
                            continue;
                        }
                        ma = replace_data(ma, "${"+e[0]+"}", e[1])
                        //console.log(ma)
                        //console.log(e)
                    };
                    return cmlobj(ma)
                },
                getString:() =>{
                    return tostr_data(xml)
                },
                
            }
        
        
        
            return me;
        },
        recml:(data) => {
            let salida = [];
            //console.log(data)
            
            let modo = "normal";
            let cadena = "";
            let tag = "";
            let etiqueta = {
                tag:"tag",
                arg:{},
                inner:"",
                nodes:[]
            };
            let contar = 0;
            for (let i = 0; i < data.length; i++) {
                let e = data[i];
                if (modo == "normal") {
                    if (e=="<") {
                        if (cadena.trim() != "") {
                            
                            salida.push(cadena.trim());
                            cadena= "";
                        } else {cadena = ""};
                        
                        etiqueta = {
                            tag:"tag",
                            arg:{},
                            inner:"",
                            nodes:[]
                        };
                        modo = "tag"
                        
                    } else {
                        cadena = cadena +e;
                    }
                } else if (modo == "tag") {
                    if (e==" ") {
                        if (cadena != "") {
                            tag = cadena
                        } else{
                            tag="tag"
                        };
                        etiqueta.tag = tag;
                        modo = "arg";
                        cadena = "";
                    }else if (e==">") {
                        if (cadena.substr(cadena.length-1) == "/") {
                            //console.log("error")
                            cadena = cadena.substr(0, cadena.length-1)
                            if (cadena != "") {
                                tag = cadena
                            } else{
                                tag="tag"
                            };
                            etiqueta.tag = tag;
                            modo = "normal";
                            cadena = "";
                            salida.push(etiqueta)
                        } else {
                            
        
                            if (cadena != "") {
                                tag = cadena
                            } else{
                                tag="tag"
                            };
                            etiqueta.tag = tag;
                            modo = "inner";
                            cadena = "";
                            contar = 0;
                        }
                        
                    } else {
                        cadena = cadena +e;
                    }
                } else if (modo == "arg") {
                    if (e==">") {
                        let = arg={};
                        
                        
                        cadena = cadena.trim();
                        let subcadena="";
                        let submodo="normal";
                        let subarg = "";
                        for (let o = 0; o < cadena.length; o++) {
                            const ael = cadena[o];
                            if (submodo == "normal") {
                                
                                if (ael == "=") {
                                    
                                    if (cadena[o+1] == '"') {
                                        
                                        subarg = subcadena.trim();
                                        submodo = "str";
                                        subcadena = '"';
                                        o++;
                                    } else {
                                        subarg = subcadena.trim();
                                        submodo = "comi";
                                        subcadena = '';
                                        o++;
                                    }
                                                              
                                } else {
                                    subcadena = subcadena + ael;
                                }
                            } else if (submodo == "str") {
                                if (ael == '"') {
                                    arg[subarg] = subcadena.substr(1);
                                    submodo = "normal"
                                    subcadena ="";
                                } else {
                                    subcadena = subcadena + ael;
                                }
                            } else if (submodo == "comi") {
                                if (ael == '"') {
                                    //subarg = subcadena.trim();
                                    submodo = "str";
                                    subcadena = '"';
                                   
                                }
                            }
                            
                        };
        
                        etiqueta.arg = arg;
                        if (cadena[cadena.length-1] == "/") {
                            cadena = "";
                            salida.push(etiqueta);
                            modo = "normal";
                        } else {
                            cadena = "";
                            modo = "inner";
                            contar = 0
                        }
                       
                        
                    } else {
                        cadena = cadena +e;
                    }
                } else if (modo == "inner") {
                    if (e == '>') {
                        cadena = cadena + e;
                        if (cadena.substr(cadena.length-`</${tag}>`.length) == `</${tag}>`) {
                            contar = contar-1
                        };
                        
                        if (contar == -1) {
                            cadena = cadena.substr(0, cadena.length-`</${tag}>`.length).trim()
                            //console.log(cadena)
                            let lii = tools.cml.recml(cadena);
                            etiqueta.inner = cadena;
                            etiqueta.nodes = lii;
                            cadena = "";
                            modo = "normal";
                            salida.push(etiqueta)
                            etiqueta = {
                                tag:"tag",
                                arg:{},
                                inner:"",
                                nodes:[]
                            };
                            //console.log(lii)
                        }
                        
                        
                    } else {
                        cadena=cadena+e
                        if (cadena.substr(cadena.length-`<${tag}`.length) == `<${tag}`) {
                            contar = contar+1
                        };
                        
                    }
                }
            };
            if (cadena.trim() != "") {
                salida.push(cadena.trim())
            }
            //console.log(modo)
            return(salida)
        }
    }
}


let App = (name, pass, ApiJS, Consol) => {
    let asi = (e, t) => Object.assign(e, t);
    ApiJS = ApiJS||false;
    Consol=Consol||false;

    let Api = {
        Module:asi(() => {
            return {
                __class__:"Module",
                __clase__:Api.Module
            }
        }, {
            __class__:"Module",
        }),
        String:asi((a) => {
            
            if (typeof(a) == "object") {
                
                if ( (a||{})._str ) {
                    return a._str()
                }
                
            }
            
            return a+""
        }, 
        {
            add:(t, f) => {me.tstr[t+""] = f},
            __class__:"String"
        }),
        Integer:asi((a) => {

            if (typeof(a) == "object") {
                
                if ( (a||{})._int ) {
                    return a._int()
                }
                
            }

            return parseInt(a)
        }, {__class__:"Integer"}),
        Float:asi((a) => {
            
            if (typeof(a) == "object") {
                
                if ( (a||{})._float ) {
                    return a._float()
                }
                
            }

            return parseFloat(a)
        }, {__class__:"Float"}),
        Array:asi((a) => {
            let salida = [];

            if (typeof(a) == "string" | Array.isArray(a)) {
                
                for (let i = 0; i < a.length; i++) {
                    let e = a[i];
                    salida.push(e)
                }

            } else {
                if ( (a||{})._array ) {
                    salida = a._array()
                } else {
                    
                    salida = [a];
                }
            }


            return asi(salida, {__class__:"Array", __clase__:Api.Array, toString:() => JSON.stringify(salida)})
        }, {__class__:"Array"}),
        Boolean:asi((e) => {
            
            if (typeof(e) == "string") {
                return !(e==="")
            } else if (typeof(e) == "number") {
                return !(e===0)
            } else if (typeof(e) == "boolean") {
                return (e)
            } else if (typeof(e) == "object") {
                if ( (e||{})._bool ) {
                    return e._bool()
                } else {
                    return Boolean(e)
                }
            }

        }, {__class__:"Boolean"}),
        Function:asi((e) => {
            return asi (e, {
                __class__:"Function",
                __clase__:Api.Function
            })
        }, {__class__:"Function"}),
        Any:asi(e => (
            e
        ), {__class__:"Any"}),
        Namespace:asi((e)=> {
            return {
                __clase__:Apibase.Namespace,
                __class__:"Namespace",
                __names__:e
            }
        }, {__class__:"Namespace"}),
        Object:asi(
            (e) => asi(e, {
            __class__:"Object",
            __clase__:Api.Object,
            toString:() => JSON.stringify(e)
            }, e), 
        {__class__:"Object"}),
        print:(...e) => {
            console.log(...e);
            /*console.log(Consol);
            if (Consol) {
                Consol.innerText = Consol.innerText + tools.chars.N + e.join(" ")
            }*/
        },
        iter:(e) => {
            if ((typeof(e)==="string")|(Array.isArray(e))) {
                return e
            } else {
                return e._iter()
            }
        },
        len:(e) => {
            if ((typeof(e) === "string")|(Array.isArray(e))) {
                return e.length
            } else {
                return e._len()
            };
        },
        
    }
    let Apibase = Api;
    asi(Api, {
        str:Api.String,
        int:Api.Integer,
        float:Api.Float,
        list:Api.Array,
        bool:Api.Boolean,
        __app__:{
            name:name,
        },
        void:{__class__:"Void"},
        Void:{__class__:"Void"},
        obj:Api.Object,
        true:true,
        True:true,
        on:true,
        false:false,
        False:false,
        off:false,
    });

    asi(Api, pass||{})

    function typer (v) {
        if (typeof(v) !== "object") {
            if (typeof(v) === "number") {
                let sal = "Integer";
                if ((v+"").includes(".")) sal = "Float";
                return sal
            }else return (({
                boolean:"Boolean",
                string:"String",
                function:"Function",
                undefined:"Void"
            })[typeof(v)])
        } else {
            return v.__class__||"Object"
        }
    }

    let Script = (c, o) => {
        
        
        let Api = asi(Apibase, {
            _file:o,
            _name:name
        })
    
        function funca(e) {
            asi(e, {__class__:"Function", __clase__:Api.Function})
            me.func_list.push(e)
            return e
        }
    
    
        let me = {
            origin: o||"",
            code: c||"",
            stadef:"Any",
            errores: [],
            namespace:"std",
            tstr:{
                "":Api.String
            },
            func_list:[],
            func_count:0,
            conti:0,
            code_exit:0,
            onexit:(e) => {
                //console.log("code exit", e)
            },
            getlib:(src) => {

                if (myapp.libs[src]!==undefined) {
                    return myapp.libs[src]
                } else {
                    me.error(`lib '${src}' not found`, tools.errores.ModuleError, me.index)
                }
            },
            desline: (c) => {
                
                let cadena = "";
                let linea = []
                let salida = []
                let modo = "normal"
                let byte = "";
                let str_l = ""
                let tag = ""
                let t_count = 0;
                
    
    
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
    
                                    
                                    
                                    if ( ["<", ">", "=", ":", "-"].includes(w) ) {
                                        if ( ["<", ">", "=", ":"].includes(e) )  {
                                            linea.pop()
                                            linea.push(tools.tipos.ope(w + e, i))
    
                                        } else {
                                            linea.push(tools.tipos.ope(e, i))
                                        }
                                    } else if (e.length === 1) {
                                        if (["<", ">", "=", ":"].includes(e)) {
                                            
                                            linea.push(tools.tipos.ope(e, i))
                                            
                                        } else {
                                            linea.pop()
                                            linea.push(tools.tipos.ope(w+e, i))
                                            
                                        }
    
                                    } else {
                                        linea.push(tools.tipos.ope(e, i))
    
                                    }

                                    
    
                                } else {
                                    linea.push(tools.tipos.ope(e, i))
    
                                }
                                let med = linea.slice(-3)
                                let med2= [ 
                                    (med[0]||{}).tipo === "ope",
                                    (med[1]||{}).tipo === "name",
                                    (med[2]||{}).tipo === "ope",
                                    (med[0]||{}).char === "<",
                                    (med[2]||{}).char === ">",
                                ]
                                
                                if (!med2.includes(false)) {
                                    
                                    linea.pop()
                                    let eti = linea.pop().name
                                    linea.pop()

                                    

                                    cadena = `<${eti}>`;
                                    tag = eti;
                                    modo = "tag";
                                    t_count = 0
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
                        if (e == tools.chars.N) {
                            modo = "normal"
                        }
                    } else if (modo === "tag") {
                        if (e == '>') {
                            cadena = cadena + e;
                            if (cadena.substr(cadena.length-`</${tag}>`.length) == `</${tag}>`) {
                               t_count =t_count-1
                            };
                            
                            if (t_count == -1) {
                                linea.push(tools.paren.cml(cadena, i-cadena.length));
                                cadena = "";
                                mode = "normal";
                            }
                            
                            
                        } else if ([tools.chars.N,tools.chars.R,tools.chars.T].includes(e)) {
            
                        } else {
                            cadena=cadena+e
                            if (cadena.substr(cadena.length-`<${tag}`.length) == `<${tag}`) {
                               t_count =t_count+1
                            };

                            if (cadena.substr(-2) == "  ") {
                                cadena = cadena.substr(0, cadena.length-1) 
                            }
                            
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
                throw msg;
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
                                    def:me.estructuration_one(def)
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
                                def:me.estructuration_one(def)
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
                    if (sta.length===0) sta = [me.stadef]
                    
                    salida.push({
                        arg:arg,
                        sta:sta,
                        def:me.estructuration_one(def)
                    })
                }
    
                return salida
            },
            estructuration_one:(c) => {
                let salida = [];
                let cadena = [];
    
                c = [c]
    
                for (let i = 0; i < c.length; i++) {
                    let e = c[i];
                    
                    cadena = [];
    
                    for (let x = 0; x < e.length; x++) {
                        let ele = e[x];
    
                        //console.log(ele.tipo)
    
                        if (ele.tipo == "()") {
                            cadena.push(
                                tools.paren.tupla(
                                    ele.complet.map(e=>me.estructuration_one(e)), ele.i
                                )
                            )
                        }  else if (ele.tipo == "[]") {
                            cadena.push(
                                tools.paren.list(
                                    ele.complet.map(e=>me.estructuration_one(e)), ele.i
                                )
                            )
                        }  else if (ele.tipo == "code") {
                            cadena.push(
                                tools.paren.code(
                                    (ele.data).map(e=>me.estructuration_one(e)), ele.i
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
    
                return salida[0]
            },
            estructuration:(c) => {
    
                let salida = []
    
                let var_names = []
                
                function gen_eval(c, i) {
                    return {
                        i:i,
                        eval:me.estructuration_one(c),
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
                                            cond:me.estructuration_one(ceo[1+x].data),
                                            code:ex_names(me.estructuration(ceo[2+x].data)),
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
                                                    cond:me.estructuration_one(ceo[1+x].data),
                                                    code:ex_names(me.estructuration(ceo[2+x].data)),
                                                })
                                                
                                            } else {
                                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                            }
                                        } else if (ceo[x+1].tipo == "code") {
                                            if (ceo.slice(x).length < 2) {
                                                me.error("Syntax Error", tools.errores.SyntaxError, e[0].i)
                                            }
                                            //console.log(e[1+x])
                                            out.push({
                                                to:"else",
                                                code:ex_names(me.estructuration(ceo[1+x].data)),
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
                                    list:out,
                                    i:e[0].i
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
                                            data:e[1].complet.map(e => {
                                                return(me.estructuration_one(e))
                                            }),
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
                                            itera:e[2].name,
                                            data:me.estructuration_one(e[3].data),
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
                                            value:me.estructuration_one(e[2].data),
                                            name:e[1].name,
                                            code:ex_names(me.estructuration(e[3].data)),
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
                            } else if (e[0].name == "namespace") {
                                if (tools.compare(["name", "name", "code"], e)) {
                                    salida.push({
                                        i:e[0].i,
                                        tipo:"namespace-def",
                                        name:e[1].name,
                                        code:me.estructuration(e[2].data)
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
                            } else if (e[0].name == "return") {
                                salida.push({
                                    tipo:"rt-def",
                                    eval:me.estructuration_one(e.slice(1))
                                })
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
                                            
                                            var:me.estructuration_one(e.slice(0, dim_le)),
                                            name:e[0].name,
                                            eval:me.estructuration_one(e.slice(dim_le+1)),
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
                
                
                if (code.names !== undefined & !["class", "module"].includes(mode)) {
                    code.names.forEach(e=>{
                        //console.log(e)
                        salida.push(
                            `let var_${e}`
                        )
                    })
                }
    
                function error_p(c, i) {
                    if ((i||dexi) === undefined) {
                        console.log(c);
                        console.log(i, dexi);
                    }
                    return[
                        "app.index = " + (i||dexi),
                        "try {",
                            c,
                        "} catch (e) {",
                        `    app.error(e, 'ErrorExec', ${i||dexi})`,
                        "}"
                    ]
                }
                
                let dexi = 0;
                //console.log(code)

                for (let i = 0; i < code.length; i++) {
                    let e = code[i];

                    dexi = e.i||dexi
    
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
                            `${name_var} = Api.Function( ${asyncrono} (...arg) => {`,
                            //`    let este = ${name_var}`,
                            `    let rt = var_${e.sta}`,
    
                            ...e.arg.map((e)=>{
                                iter++;
                                let tto = me.generator_one(e.def, mode)
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
                            `    let var_me = ((...ar) => { if (me._call) return me._call(...ar) });`,
                            `    let var_private = {};`,
                            `    let exportar = [];`,
                            `    let out = {__clase__:"${name_var}"};`,
                            `    let me = var_me;`,
                            `    let me.toString = h=>me._str(h)`,
                            `    let me._str = k=>"<${name_var}-Instance:0x${
                                (me.conti++).toString(16)
                            }>"`,
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
                            `    Object.assign(me, {__class__:"${name_var}", __clase__:este})`,
                            "    return me",
                            `}; ((${name_var})());`
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
                            `${name_var} = () => {`,
                            `    let este = ${name_var};`,
                            `    let var_me = {};`,
                            `    let var_private = {};`,
                            `    if (!este.__class__)Object.assing(este, {__class__:"Module", __clase__:Api.Module});`,
                            `    let me = var_me;`,
                            `    let private = var_private;`,
                            
                            me.generator(e.code, "module"),
    
                            "    return me",
                            `};${name_var} = (${name_var})();`
                        )
                    } else if ((e.tipo == "namespace-def") & ["normal", "func", "module"].includes(mode)) {
                        let name_var = "var_" + e.name;
                        if ("module" === mode) {
                            name_var = `me.${e.name}`;
    
                            if (e.visible=="private") {
                                name_var = `private.${e.name}`;
    
                            }
                        }
                        
                        salida.push(
                            `${name_var} = (() => {`,
                            `    let este = ${name_var};`,
                            `    let var_me = {};`,
                            `    let var_private = {};`,
                            `    if (!este.__class__)Object.assign(este, {__class__:"Namespace", __clase__:Api.Namespace});`,
                            `    let me = var_me;`,
                            `    let private = var_private;`,
                            
                            me.generator(e.code, "module"),
    
                            "    return Api.Namespace(me)",
                            `});${name_var} = (${name_var})();`
                        )
                    } else if ((e.tipo == "exec") & ["normal", "func"].includes(mode)) {
                        salida.push(
                            ...error_p("    "+me.generator_one(e.eval, mode, "exec"), e.i)
                        )
                    } else if ((e.tipo == "if-def") & ["normal", "func"].includes(mode)) {
                        let out = [];
    
                        for (let x = 0; x < e.list.length; x++) {
                            let element = e.list[x];
                            if (element.to == "if") {
                                out.push(...[
                                    `if (${me.generator_one(element.cond, mode, "eval")}) {`,
                                        me.generator(element.code, mode),
                                    `}`
                                ])
                            } else if (element.to == "elif") {
                                out.push(...[
                                    `else if (${me.generator_one(element.cond, mode, "eval")}) {`,
                                        me.generator(element.code, mode),
                                    `}`
                                ])
                            } else if (element.to == "else") {
                                out.push(...[
                                    `else {`,
                                        me.generator(element.code, mode),
                                    `}`
                                ])
                            }
                        }
    
                        salida.push(...error_p(out, e.i))
                    } else if ((e.tipo == "while-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p([
                            `while (${me.generator_one(e.cond, mode, "eval")}) {`,
                                me.generator(e.code, mode),
                            `}`
                        ], e.i))
                    } else if ((e.tipo == "for-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p([
                            `for (let ${
                                me.generator_one(e.data[0], mode, "exec")
                            }; ${
                                me.generator_one(e.data[1], mode, "exec")
                            }; ${
                                me.generator_one(e.data[2], mode, "exec")
                            }) {`,
                                    me.generator(e.code, mode),
                            `}`
                        ], e.i))
                    } else if ((e.tipo == "for-each-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p([
                            `let i_mii = Api.iter(${me.generator_one(e.data, mode)});`,
                            `for (let itera = 0; itera < i_mii.length; itera++) {`,
                            `    let var_${e.itera} = i_mii[itera];`,
                                 me.generator(e.code, mode),
                            `}; delete i_mii;`
                        ], e.i))
                    } else if ((e.tipo == "with-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p([
                            `;((var_${e.name}) => {`,
                                me.generator(e.code, mode),
                            `})(${me.generator_one(e.value, mode)});`
                        ], e.i))
                    } else if ((e.tipo == "rt-def") & ["func"].includes(mode)) {
                        salida.push(...error_p([
                            `return app.dim((${me.generator_one(e.eval, mode)}), rt)`
                        ], e.i))
                    } else if ((e.tipo == "try-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p([
                            "try {",
                                me.generator(e.try_code, mode),
                            `} catch (var_${e.e_name}) {`,
                                me.generator(e.e_code, mode),
                            "}"
                        ], e.i))
                    } else if ((e.tipo == "import-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p(
                            [`var_${e.as} = app.getlib("${e.lib}")`], e.i
                        ))
                    } else if ((e.tipo == "from-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...error_p(
                            [`var_${e.as} = app.getlib("${e.lib}").${e.import}`], e.i
                        ))
                    } else if ((e.tipo == "include-def") & ["normal", "func"].includes(mode)) {
                        salida.push(...[
                            `let lib_in = app.getlib("${e.lib}")`,
                            `let lib_dex = Object.keys(lib_in)`,
                            `let var_li = ""`,
                            `for (let iom = 0; iom < lib_dex.length; iom++) {`,
                            `    let index = lib_dex[iom]`,
                            `    var_li = var_li + ("var_" + index + " = lib_dex['" + index + "']") + ";"`,
                            `};`,
                            //"console.log(var_li)",
                            "eval(var_li);",
                            `delete lib_dex, lib_in, var_li`
                        ])
                    } else if ((e.tipo == "var") & ["normal", "func", "class", "module"].includes(mode)) {
                        if (e.one) {
                            let l_out = [];
                            let qqq = "";
    
                            if (["normal", "func"].includes(mode)) {
                                l_out = [`${me.generator_one(e.var, mode)} = ${me.generator_one(e.eval, mode)}`];
                            } else {
    
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
                                };
    
                                l_out = [`${name_var} = ${me.generator_one(e.eval, mode)}`, qqq];
    
    
                            }
    
                            salida.push(...error_p(l_out, e.i))
    
                        } else {
                            salida.push(...error_p([
                                `${me.generator_one(e.var, mode)} = ${me.generator_one(e.eval, mode)}`
                            ], e.i))
                        }
                    } else if ((e.tipo == "var-def") & ["normal", "func", "module", "class"].includes(mode)) {
                        let visibilidad = e.visible
                        let out = (e.vals.map((e)=>{
                            
                            let tto = me.generator_one(e.def, mode);
                            if (tto === "") tto = undefined+"";
    
    
                            let qqq = "";
    
                            if (["normal", "func"].includes(mode)) {
                                name_var = `var_${e.arg}`;
                            } else {
    
                                if (["class", "module"].includes(mode)) {
                                    name_var = `me.${e.arg}`;
            
                                    if (visibilidad==="private") {
                                        name_var = `private.${e.arg}`;
            
                                    }
                                    
                                    if (mode == "class") if (visibilidad==="export") {
                                        name_var = `me.${e.arg}`;
                                        qqq = `export.push('${e.arg}')`
                                    } else if (visibilidad==="static") {
                                        name_var = `out.${e.arg}`;
            
                                    }
                                };
    
                                //l_out = [`${name_var} = ${me.generator_one(e.eval, mode)}`, qqq];
    
    
                            }
    
    
    
                            return([
                                `${name_var} = (${tto});`,
                                `${name_var} = app.dim(${name_var}, var_${e.sta[0]})`,
                                qqq
                            ])
                        }));
                        salida.push(...error_p(out, e.i))
                    } else if ((e.tipo == "with-def") & ["normal", "func"].includes(mode)) {
                    }
                    
                }
    
    
                return salida;
            },
            generator_one:(code, mode, modei, key)=>{
                let salida = "";
                mode= mode||"normal";
                modei= modei||"eval";
                key= key||false;
                let last = {tipo:"none", i:0}
                let error = (i, msg) => {
                    //console.log(i, "'"+me.code.substr(i-5, i+5)+"'")
                    me.error("Error of syntax" + (msg||""), tools.errores.SyntaxError, i)
                }
                //console.log(code)
                
                for (let i = 0; i < code.length; i++) {
                    let e = code[i];
                    //console.log(e.tipo)
                    if (e.tipo === last.tipo) {
                        if (e.tipo === "name") {
                            if (e.name[0] !== ".") error(e.i)
                        } else if ( ["value", "code", "ope", "sim", "func-a"].includes(e.tipo) ) {
                            error(e.i)
                        } else if (e.tipo === "[]") {
                            e.fist = false
                        } else if (e.tipo === "()") {
                            e.fist = false
                        }
                    } else {
                        if ( ["[]", "()"].includes(e.tipo) ) {
                            if (["[]", "()", "name", "value", "none", "func-a"].includes(last.tipo)) {
                                if (last.tipo === "value") {
                                    if (last.type === "str") e.fist = false
                                    else error(e.i)
                                } else if (last.tipo === "none") {
                                    e.fist = true
                                } else {
                                    e.fist = true

                                    if (["[]", "()", "name"].includes(last.tipo)) {
                                        if (e.tipo === "[]") {
                                            e.fist = false
                                        }
                                        
                                    }
                                }
                            } else {
                                //console.log("llego")
                                error(e.i)
                            }
                        } else if ( e.tipo === "name" ) {
                            if ( ["value", "code", "()", "[]", "func-a"].includes(last.tipo) ) {
                                if (e.name[0] !== ".") error(e.i)
                                
                            } else if (["none", "ope", "sim"].includes(last.tipo)) {
                                if (e.name[0] === ".") error(e.i)
                                else if (last.tipo === "ope") {
                                    if (last.char === "::") {
                                        e.name = ".__names__."+e.name;
                                        e.notmod = true;
                                    }
                                }
                            }
                        } else if ( e.tipo === "value" ) {
                            if ( !["ope", "sim", "none"].includes(last.tipo) ) {
                                error(e.i)
                            }
                        } else if ( e.tipo === "code" ) {
                            if ( !["ope", "none"].includes(last.tipo) ) {
                                error(e.i)
                            }
                        } else if ( e.tipo === "ope" ) {
                            if ( !["[]", "()", "name", "code", "value", "none"].includes(last.tipo) ) {
                                error(e.i)
                            }
                        } else if ( e.tipo === "func-a" ) {
                            if ( !(last.tipo === "none") ) error(e.i)
                        }
    
                        if (key & ( e.tipo === "name" )) {
                            if (["sim", "none"].includes(last.tipo)) {
                                e = tools.tipos.value(`'${e.name}'`, "str", e.i, "")
                                e.key = true
                            }
                        }
                    }
    
                    last = e
    
                    
    
    
    
                    if (e.tipo === "name") {
                        if (["or", "and"].includes(e.name)) {
                            salida = salida + ` ${
                                ({
                                    or:"|", 
                                    and:"&",
                                })[e.name]
                            } `;
                            
                        } else if (["break", "continue"].includes(e.name)) {
                            salida = salida + ` ${e.name} `;
                        } else {
                            if (!e.notmod) { 
                                if (e.name[0]===".") salida = salida + ` ${e.name} `;
                                else salida = salida + ` var_${e.name} `;
                            } else {
                                salida = salida + ` ${e.name} `;
                            }
                            
                        }
                    } else if (e.tipo === "value") {
    
                        if (e.type === "str") {
                            if (!e.key) salida = salida + ` app.tstr['${e.byte}'](${e.value}) `;
                            else salida = salida + `${e.value}`
                        } else if (e.type === "int") {
                            salida = salida + ` Api.int(${e.value}) `;
                        } else if (e.type === "float") {
                            salida = salida + ` Api.float(${e.value}) `;
                        }
                        
                    } else if (e.tipo === "()") {
                        salida = salida + ` (${me.generator_one(e.data, mode, "eval")}) `;
                    } else if (e.tipo === "[]") {
                        if (e.fist) salida = salida + ` Api.Array([${me.generator_one(e.data, mode, "eval")}]) `;
                        else salida = salida + ` [${me.generator_one(e.data, mode, "eval")}] `;
                    } else if (e.tipo === "code") {
                        salida = salida + ` Api.obj({${me.generator_one(e.one, mode, "eval", true)}}) `;
                    } else if (e.tipo === "ope") {
                        if (["+", "*", "-", "/", "%", "&", "!", "<", ">", "=", ":"].includes(e.char)) {
                            salida = salida + ` ${e.char} `;
                        } else if (["==", "!=", ">=", "<=", "++", "--", "**"].includes(e.char)) {
                            salida = salida + ` ${e.char} `;
                            
                        } else if (e.char === "^") {
                            salida = salida + ` ** `;
    
                        }
                    } else if (e.tipo === "sim") {
                        if (e.char === ",") {
                            salida = salida + ", "
                        }
                    } else if (e.tipo == "func-a") {
                        let iter = -1;
                        
                        salida = salida + me.jump([
                            `funca((...arg) => {`,
                            `    let este = app.func_list[${me.func_count}];`,
                            `    let rt = var_${e.sta};`,
    
                            ...e.arg.map((e)=>{
                                iter++;
                                let tto = me.generator_one(e.def)
                                if (tto === "") tto = undefined+""
    
                                return([
                                    `let var_${e.arg} = (arg[${iter}]||(${tto}));`,
                                    `var_${e.arg} = app.dim(var_${e.arg}, var_${e.sta[0]});`
                                ])
                            }),
    
                            me.generator(e.code, "func"),
    
                            `})`
                        ], 0);
    
                        me.func_count++; 
                        
    
                    }
                    
                }

                return salida.trim()
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
            },
            exec:(code_run) => {
                let app = me;
                let var_export = {};

                let a = Object.keys(Api)
                let listar = []

                for (let i = 0; i < a.length; i++) {
                    let e = a[i];
                    listar.push(`let var_${e} = Api['${e}']`)
                }



                eval(
                    listar.join(";") + ";" +
                    code_run
                )

                myapp.libs[o] = var_export;

                me.onexit(me.code_exit)
            },
            dim:(value, type) => {
                
                //console.log(value, type)

                if (type.__class__ === "Any") {
                    return value
                } else if (type.__class__ === "Void")  {
                    me.error("error of typing at write a value in a var of type void", tools.errores.TypingError, me.index)
                } else if (Array.isArray(value) & ("Array" === type.__class__))  {
                    return Api.Array(value)
                } else if (typer(value) === type.__class__)  {
                    return value
                } else {
                    me.error(`error of typing at write a value of type '${
                        typer(value)
                    }' in a var of type '${type.__class__}'`, tools.errores.TypingError, me.index)

                }
            }, 
            compile:(c, modo) => {
                modo = modo||"normal";

                let k = me.desline(c);
                let par = me.parselex(k);
                let estr = me.estructuration(par);
                let gen = me.generator(estr, modo);
                let salida = me.jump(gen, 0)

                delete k, par, estr, gen, modo

                return salida
            }
        };
    
        return me
    
    };

    let myapp = {
        libs:{
            document:this.document||{},
            window:this.window||{}
        },
        Script:Script,
        Api:Api
    };

    if (ApiJS) {
        myapp.libs["ApiJS"] = {
            getthis:x =>this[x],
            eval:v => eval(v),
        }
    }

    return myapp
}

let CLSWEBAPP

try {
    module.exports.App = App
} catch (e) {
    CLSWEBAPP = App(
        "WebAplication", 
        {
            input:e => prompt(e)
        }, 
        true
    );
    window.addEventListener("load", () => {

        let tags = document.getElementsByTagName("script");
        let scripts = []
        let enume = 0;
        for (let i = 0; i < tags.length; i++) {
            const e = tags[i];
            if (e.getAttribute("type") === "cls/script") scripts.push(e)
            
        }
        
        //console.log(scripts);
    
        scripts.reverse();
    
        function leer() {
            if (scripts.length === 0) return;
    
            let este =scripts.pop()
            
            if (este.getAttribute("src")) {
                fetch(este.getAttribute("src"), {}).then(e=>{
                    e.text().then(x => {
                        
                        let ap = CLSWEBAPP.Script(
                            x||"",
                            este.getAttribute("module")||este.getAttribute("src")
                        );
            
                        let k = ap.compile(x||"")
                        ap.onexit = leer;

                        if (este.getAttribute("debug")) console.log(k)
                        
                        ap.exec(k);
                        
                        delete k, ap
                    
                    })
                }).catch(x => {leer()})
            } else {
                let ap = CLSWEBAPP.Script(
                    este.innerHTML||"",
                    este.getAttribute("module")||"Script" + (enume++)
                );
    
                let k = ap.compile(este.innerHTML||"")
                ap.onexit = leer;
                if (este.getAttribute("debug")) console.log(k)
                ap.exec(k);
                
                delete k, ap
    
            }
    
        };
    
        leer()
    })
}

