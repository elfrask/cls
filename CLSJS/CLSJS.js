function jse() {
    return({
        js:eval
    })
}

let CLSJS = (function(js_active, all) {

    /**/

all= all || true;
js_active= js_active || true;
let modulo_cls = {};
let Ver = {
    ver:"CLS 0.0.1 alpha",
    pkg:"CLS RE_PKG 0.0.1 Alpha NJ_19e",
    compiler:"Plix-compiler v3 0.1 Alpha NJ_19e",
    web:"CLSJS v1 Web_frame 0.1 Alpha JS_20"
}


require = (function(){ 
    try {
        require;
        return require
    } catch (error) {
        try {
            
        } catch (error) {
            
        }
        return(
            ((pkg) => {
                try {
                    return (eval((pkg+"").split("/").pop()))
                } catch (error) {
                    return{}
                }
                
            })
        )
    }

})();


let os;
let fs = require('fs');
let Crypto = require('./js/CryptoJS');
//let { func } = require('./python_api');
try {
 
os = require('os');
fs = require('fs');
   
} catch (error) {
    
}

let libs = [];







//const tools = require('./python_api.js');
//const expretions = require('./expretion')
let N="\n"
let R="\r"
let T="\t"
let B="\b"
let BAR_INVERTD = "\\"
let COMILLAS = "\""
let APOSTROFE = "\'"
let tools = (function () {
    let me = {}
    me.convert = {
        toArray: function (code) {
            var chars = []
            for (let i = 0; i < code.length; i++) {
    
                chars[i] = code[i]
        
            }
            return chars
        },
        join: function (lista, u) {
    
            var salida = ""
    
    
            for (let x = 0; x < lista.length; x++) {
                salida = salida + lista[x] + u;
                
            }
    
            salida = salida.substr(0,salida.length -u.length);
    
            return salida
            
        },
        
    };
    
    me.get = {
        argv: function (mai) {
            let rtfile= mai || process.argv;
    
            try {
                document.childNodes;
                rtfile = JSON.parse(fs.readFileSync("./cache/_argv"))[0]
                
            }catch (e) {
                
            }
    
    
            oa=rtfile[0].length
    
            if (os.platform() == "win32") {
                oa1 = rtfile[0].substr(oa-8);
                if (oa1 == "node.exe") {
                    rtfile = rtfile.slice(1, rtfile.length);
                    //rtfile[0] = "./"+rtfile[0]
                    return rtfile
                }
                oa1 = rtfile[0].substr(oa-12);
                if (oa1 == "electron.exe") {
                    rtfile = rtfile.slice(1, rtfile.length);
                    //rtfile[0] = "./"+rtfile[0]
                    return rtfile
    
                } else{
                    rtfile = rtfile;
                }
    
            } else if (os.platform() == "linux") {
                oa1 = rtfile[0].substr(oa-4);
                if (oa1 == "node") {
                    rtfile = rtfile.slice(1, rtfile.length);
                    //rtfile[0] = "./"+rtfile[0]
                    return rtfile
    
                } 
                oa1 = rtfile[0].substr(oa-8);
                if (oa1 == "electron") {
                    rtfile = rtfile.slice(1, rtfile.length);
                    //rtfile[0] = "./"+rtfile[0]
                    return rtfile
    
                } else{
                    rtfile = rtfile;
                }
            } else {
                
                //TypeError("La plataforma actual '" +os.platform()+ "' no tiene actualmente soporte para esta aplicacion Error code 0x0001")
                rtfile = console.error("La plataforma actual '" +os.platform()+ "' no tiene actualmente soporte para esta aplicacion Error code 0x0001");
                process.exit(0x0001)
            }
            return rtfile
        },
        lib: (nodo, app, pkg) => {
    
            let salida = [];
            //librarys
            for (let i = 0; i < fs.readdirSync(nodo+"/lib/" + pkg).length; i++) {
                const e = fs.readdirSync(nodo+"/lib/" + pkg)[i];
                salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), nodo+ "/lib/"+pkg +"/" +e, "lib"]
                
            }
            try {
                for (let i = 0; i < fs.readdirSync(app+"/lib/" + pkg).length; i++) {
                    const e = fs.readdirSync(app+"/lib/" + pkg)[i];
                    salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), app+ "/lib/"+pkg +"/" +e, "lib"]
                    
                }
            } catch (error) {
                
            }
    
            for (let i = 0; i < fs.readdirSync(nodo+"/lib/global").length; i++) {
                const e = fs.readdirSync(nodo+"/lib/global")[i];
                salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), nodo+ "/lib/global/" +e, "lib"]
                
            }
            try {
                for (let i = 0; i < fs.readdirSync(app+"/lib/global").length; i++) {
                    const e = fs.readdirSync(app+"/lib/global")[i];
                    salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), app+ "/lib/global/" +e, "lib"]
                    
                }
            } catch (error) {
                
            }
            
            //includes
            for (let i = 0; i < fs.readdirSync(nodo+ "/includes/" +pkg).length; i++) {
                const e = fs.readdirSync(nodo+ "/includes/" +pkg)[i];
                salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), nodo+ "/includes/" +pkg +"/"+e, "include"]
                
            }
            try {
                for (let i = 0; i < fs.readdirSync(app+ "/includes/" +pkg).length; i++) {
                    const e = fs.readdirSync(app+ "/includes/" +pkg)[i];
                    salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), app+ "/includes/"+pkg +"/"  +e, "include"]
                    
                }
            } catch (error) {
                
            }
    
            for (let i = 0; i < fs.readdirSync(nodo+ "/includes/global").length; i++) {
                const e = fs.readdirSync(nodo+ "/includes/global")[i];
                salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), nodo+ "/includes/global/"+e, "include"]
                
            }
            try {
                for (let i = 0; i < fs.readdirSync(app+ "/includes/global").length; i++) {
                    const e = fs.readdirSync(app+ "/includes/global")[i];
                    salida[salida.length] = [(e.split(".").slice(0, e.split(".").length-1)).join("."), app+ "/includes/global/"  +e, "include"]
                    
                }
            } catch (error) {
                
            }
    
    
    
    
            return salida;
            
    
        }
    };
    
    me.func = {
        print_list: function (l) {
            for (let i = 0; i < l.length; i++) {
               
                console.log(module.exports.func.replace(l[i]))
            }
        },
        replace: function (s) {
            s=s+""
    
            try {
                if (s.search(R)) {
    
                    s = s.replace(R, "")
                    s = this.replace(s)
                }
            } catch (error) {
                s = s
            }
            
    
            return s
        },
        replace_chars: function (s, a, b) {
            s=s+""
    
            try {
                if (s.search(a)) {
    
                    s = s.replace(a, b)
                    s = this.replace_chars(s, a, b)
                }
            } catch (error) {
                s = s
            }
            
    
            return s
        }
    };

    return (me);
})()
let clv = {}
let eli = false;






try {
    var remote = require("electron").remote
    argumenta = remote.getGlobal("sharedObject");
    if(argumenta.ele) {
        __dirname = argumenta.path;
        argumentos = argumenta.argv;
        eli = true;
        console.log = argumenta.func.print;
    }
} catch (error) {
    
}


/*

var ins = expretions.ins
var name = expretions.name
var val = expretions.val
var ope = expretions.ope
var sim = expretions.sim
*/
//instruction
class ins {
    constructor(valy) {
        this.valy = valy
    };
    rute(rt) {
        //declarar la ruta del archivo donde estara nuestra instruccion
        this.rt = rt
    };
    get() {
        return this.valy
    };
    tipo(){
        return "instruction"
    };
    use_lines(start, end) {

        this.log = [start, end]
        //declarar la linea o lineas donde se encuentra la instruccion

    };
    __eye__() {
        var a = this.valy
        var s = ""
        for (let i = 0; i < a.length; i++) {
            
            s = ", " + a[i].__eye__()
            
        }

        s = s.substr(2)
        return "ins(" + s + ")" 
    }
};
//val

function val(valy, tip, der, i)  {

    
    if (undefined == der) {
        b=""
        
    }else {
        b = der
    }

    

    return (
        {
            valy:valy,
            tip:tip,
            der:der,
            tipo:"val",
            i:i
        }
    )
};
//name


function name (nae, i){
    return({
        tipo:"name",
        nm:nae,
        i:i
    })
}
//operator

function ope(char, i){
    
    return(
        {
            c:char,
            tipo:"operator",
            conditional:false,
            i:i
        }
    )
}
//simbols

function sim(char, i)  {
    return({
        tipo:"simbols",
        c:char,
        i:i
    })
}

//json 
let json_val = (j)=> {
    j = tools.func.replace_chars(j, BAR_INVERTD+"n", "")
    j = tools.func.replace_chars(j, N, "")
    //j = JSON.parse(j)

    j = `{${j}}`

    try {
        j = JSON.parse(j)
    } catch (err) {
        j = {"error":err.toString()}
    }

    return({
        tipo:"json",
        data:JSON.stringify(j)
    })
}

class node {
    constructor(a1, argvs, inner, id) {
        this.type = a1;
        this.argumentos = argvs;
        this.nodes = [];
        
    };
    addnode(a_node) {
        this.nodes[this.nodes.length] = a_node
    }
}

// decodificador de llaves a cls
function decode_cls(cod) {
    let codigo= "";
    let f = cod[1];
    let code = cod[0];
    
    for (let i = 0; i < code.length; i++) {
        const e = code[i];
        

        if (e.tipo == "name") {
                    
                    
            codigo = codigo + ` ${e.nm} `


        } else if (e.tipo == "val") {


            codigo = codigo + ` ${e.valy} `

        } else if (e.tipo == "operator") {

            codigo = codigo + e.c

        } else if (e.tipo == "simbols") {
            if (e.c == ",") {
                codigo= codigo +`, `
            }
            
        } else if (e.tipo == "func-def") {

            codigo = `func ${e.name}(${e.argv.join(", ")}) { ... }`

           
        } else if (e.tipo == "class-def") {
            codigo = `class ${e.name} (...) { ... }`
            
            

        } else if (e.tipo == "module-def") {
            
            codigo = `module ${e.name} { ... }`
            
            
            
            

        } else if (e.tipo == "if-def") {
            codigo = `if (${decode_cls([e.onif.cond.data, f][0])[0]}) { ... }` + N;
            for (let hj = 0; hj < e.elif.length; hj++) {
                const element = e.elif[hj];
                codigo = codigo +`elseif (${decode_cls([element.cond.data, f])[0]}) { ... }` + N;
            }
            codigo = `else { ... }` + N;


           
            
        } else if (e.tipo == "while-def") {
            codigo = `while (${decode_cls([e.cond.data,f])[0]}) { ... }`
        } else if (e.tipo == "for-def") {
            codigo = `for ${e.iterador.nm} (${decode_cls([e.valor.data, f])[0]}) { ... }`
        } else if (e.tipo == "import") {
            codigo = `import ${e.src} as ${e.name}`
        } else if (e.tipo == "include") {

            codigo = `include ${e.src}`
            
        } else if (e.tipo == "try") {
            codigo = `try { ... } error ${e.vare} { ... }`
        } else if (e.tipo == "var-def") {
            
            codigo = `${e.name} = ${decode_cls([e.eval, f])[0]}`
            
            
        } else if (e.tipo == "let-def") {
            
            codigo = `${e.name} as ${decode_cls(e.eval)[0]}`
        } else if (e.tipo == "()") {
            codigo = codigo + ` (${decode_cls([e.data, f])[0]}) `; 
        } else if (e.tipo == "[]") {
            codigo = codigo + ` [${decode_cls([e.data, f])[0]}] `; 
            
            //codigo = codigo + ` lista([${generator_one(ele.data, variable, "normal")}]) `; 

        } else if (e.tipo == "json") {
            codigo = codigo + "/{" + e.data + "}/)"; 
            
        } else if (e.tipo == "asy") {

            let lee = "";
            for (let ouou = 0; ouou < e.data.length; ouou++) {
                const milee = e.data[ouou];

                
                lee= lee +`${N}	 << (${milee.argv.join(", ")}) => { ... }`
                    
            }
            



            codigo =  `${e.name}(${decode_cls([e.pass.data, f])})${N}${lee}`; 
            
        } else if (e.tipo == "code") {
            codigo = codigo + " { ... } "
        } else if (e.tipo == "return") {
            codigo = `return (${decode_cls([e.eval, f])[0]})`
        } else if (e.tipo == "global-def") {
            
            codigo = `global ${e.name} = ${decode_cls([e.eval.data, f])[0]}`
        }



    }
    
    return [codigo, f]
}
let cle_play = false
let cle_error = `
CLS ERROR REPORT
=======================================================================
`
let crath_report =[];
let msgerror = (msg, code_cls) => {
    cle_error = cle_error +"+- " + msg+ N;
    if (trys == 0) {
        crath_report = code_cls
        console.log(`
Error!!
========================================================================
detalles: ${msg}
========================================================================
codigo: ${code_cls[0]};
========================================================================
archivo: '${code_cls[1]}'
========================================================================`)    
    } else {
        trys = trys-1
    }
    
    if (cls_execute == "native") {
        throw `${msg}`
    } else {
        //console.error(msg)
        throw `${msg}`
    }
    

}

function islib(librerias, libreria) {
    salida = false
    for (let i = 0; i < librerias.length; i++) {
        const e = librerias[i];
        if (e[0] == libreria) {
            emi = false;
            if (e[2] == "lib") {
                emi = true
            }
            return [emi, e[1]]
        }
    }

    return [salida, '']
    
};
function isinclude(librerias, libreria) {
    salida = false
    for (let i = 0; i < librerias.length; i++) {
        const e = librerias[i];
        if (e[0] == libreria) {
            emi = false;
            if (e[2] == "include") {
                emi = true
            }
            return [emi, e[1]]
        }
    }

    return [salida, '']
    
}

function importarold(file, newwork, work, name) {

    try {
        if (cls_execute == "native") {
            if (fs.existsSync(file)) {
                let data = fs.readFileSync(file, "utf8");
                let codigocrudo=desline(data, file)
                let inter = parselex(codigocrudo)
                let ibu = estructuration(inter, false, newwork)
                let exe = generator(ibu, newwork, "normal")
                addworkspace(newwork, Object.assign(plt[work]));
                //addworkspace(newwork, {});
                fs.writeFileSync("./cache/log_"+ newwork+ ".json", JSON.stringify (ibu, N, 2));
                fs.writeFileSync("./cache/log_"+ newwork+ ".js", exe);
                execute_imp(exe)
                filenames.pop();
                return workspace[newwork] 
            } else {
                if (islib(libs, file)[0]) {
                    let vol = islib(libs, file)[1]

                    let data = fs.readFileSync(vol, "utf8");
                    let codigocrudo=desline(data, vol)
                    let inter = parselex(codigocrudo)
                    let ibu = estructuration(inter, false, newwork)
                    let exe = generator(ibu, newwork, "normal")
                    addworkspace(newwork, Object.assign(plt[work]));
                    //addworkspace(newwork, {});
                    fs.writeFileSync("./cache/log_"+ newwork+ ".json", JSON.stringify (ibu, N, 2));
                    fs.writeFileSync("./cache/log_"+ newwork+ ".js", exe);
                    execute_imp(exe)
                    filenames.pop();
                    return workspace[newwork] 
                    
                } else if (isinclude(libs, file)[0]) {
                    let vol = isinclude(libs, file)[1]

                    let data = fs.readFileSync(vol, "utf8");
                    
                    return eval(data)
                    
                } else {
                    msgerror(`no se a podido importar la libreria '${file}' porque no existe.`, [`import "${file}" as ` + name, filenames[filenames.length-1]])
                    
                }
                
            }
        } else {
            msgerror(`import no esta soportado para la version web de cls`, [`import "${file}" as ` + name, filenames[filenames.length-1]])

        }
        
    } catch (error) {
        msgerror(`${error}`, [`import "${file}" as ` + name, filenames[filenames.length-1]])

    }
    
    
}

function importar(file, name) {
    if (is_compile) {
        console.time("import");
        console.log("import: empaquetando '" + file + "'");
    };
    
    try {
        if (cls_execute == "native") {
            if (fs.existsSync(file)) {
                let data = fs.readFileSync(file, "utf8");
                let codigocrudo=desline(data, file)
                let inter = parselex(codigocrudo)
                let ibu = estructuration(inter, false, name)
                let exe = generator(ibu, name, "func-imp")
                if (is_compile) {
                    console.timeEnd("import");
                    console.log("import: '" + file + "' ha sido empaquetado")
                };
                filenames.pop();
                return [exe, true] 
            } else {
                if (islib(libs, file)[0]) {
                    let vol = islib(libs, file)[1]

                    let data = fs.readFileSync(vol, "utf8");
                    let codigocrudo=desline(data, vol)
                    let inter = parselex(codigocrudo)
                    let ibu = estructuration(inter, false, name)
                    let exe = generator(ibu, name, "func-imp")
                    if (is_compile) {
                        console.timeEnd("import");
                        console.log("import: '" + file + "' ha sido empaquetado")
                    };
                    filenames.pop();
                    return [exe, true]
                    
                } else if (isinclude(libs, file)[0]) {
                    let vol = isinclude(libs, file)[1]

                    let data = fs.readFileSync(vol, "utf8");
                    if (is_compile) {
                        console.timeEnd("import");
                        console.log("import: '" + file + "' ha sido empaquetado")
                    };
                    return [data, false]
                    
                } else {

                    if (file == "jse") {
                        return[`(jse.toString)()`, false]
                    }

                    msgerror(`no se a podido importar la libreria '${file}' porque no existe.`, [`import "${file}" as ` + name, filenames[filenames.length-1]])
                    
                }
                
            }
        } else {
            if (is_compile) {
                console.timeEnd("import");
                console.log("import: '" + file + "' ha sido empaquetado")
            };
            //msgerror(`import no esta soportado para la version web de cls`, [`import "${file}" as ` + name, filenames[filenames.length-1]])
            return [impweb[file], false];
        }
        
    } catch (error) {
        msgerror(`${error}`, [`import "${file}" as ` + name, filenames[filenames.length-1]])

    };

    
    
};

function cml(code, i) {
    
    return({
        "tipo":"cml",
        "data":code,
        "i":i
    })
}

//\
let toke ={
    operator:["+", "-", "/", "*", "%", "?", "^", "!", "<", ">", "~", "@", "&", "|", ":", "="],
    sim:["(", ")", "[", "]", "{", "}", ","],
    conditional:["=", "==", "<", ">", "!", "!=", ">=", "<="]
}
//fragmenta el codigo para que se convierta en codigo legible para el compilador
var files = {}
var filenames = []
let iterador = 0


function desline(code, file) {

    var chars = []

    if (file != false) {

        files[file] = code 
        filenames[filenames.length] = file;
                
    }

    chars = tools.convert.toArray(code) 
    let cadena = "";
    let nl = 1;//descontinuado
    let st = 1;//descontinuado
    let linea = [];
    let salida = [];
    let mode = "normal";
    let Byte = "";
    let tag = "";
    let count = 0
    //let iter = 0
    for (let x = 0; x < chars.length; x++) {
        //nlinea = nlinea + 1
        let o = chars[x];
        iterador = x;
        if (mode == "normal") {
            if (o != " ") {
                if (o == ";") {//determinar donde termina la instruccion


                    var a= enterval(cadena)
                    
                    if (a == "var") {
                        if (cadena !="") {
                            linea[linea.length] = name(cadena, x-cadena.length);
                        }
                    } else {
                        linea[linea.length] = val(cadena, a, "", x-cadena.length)

                    };
                    cadena = "";


                    salida[salida.length] = new ins(linea);
                    salida[salida.length-1].use_lines(st, nl)
                    st = nl;
                    
                    linea = [];
                }else if (o == "\n") {

                    var a= enterval(cadena)
                    
                    if (a == "var") {
                        if (cadena !="") {
                            linea[linea.length] = name(cadena, x-cadena.length);
                        }
                    } else {
                        linea[linea.length] = val(cadena, a, "", x-cadena.length)

                    };
                    cadena = "";
                    nl = nl+1
                    
                }else if (o == R) {

                    var a= enterval(cadena)
                    
                    if (a == "var") {
                        if (cadena !="") {
                            linea[linea.length] = name(cadena, x-cadena.length);
                        }
                    } else {
                        linea[linea.length] = val(cadena, a, "", x-cadena.length)

                    };
                    cadena = "";
                    nl = nl+1
                    
                }else if (toke.operator.includes(o)) {
                    let a= enterval(cadena)
                    if (cadena == "") {
                        let q = linea[linea.length-1];
                        if (q==undefined) {
                            linea[linea.length] = ope(o);
                            continue;
                        }
                        

                        let w = q.c;
                        if (toke.operator.includes(w) & (o != "=") & (w != "=") & (w != ":")) {
                            linea.pop()
                            linea[linea.length] = ope(w+o, x-1)
                            if (toke.conditional.includes(linea[linea.length-1].c)) {
                                linea[linea.length-1].conditional = true;
                            }
                            //c-/**/
                            if (linea[linea.length-1].c == "//"){
                                linea.pop()
                                mode = "c-#";
                            }else if (linea[linea.length-1].c == "/*"){
                                linea.pop()
                                mode = "c-/**/";
                            }
                            
                        }else if (o == "=") {
                            if (["!", "=", "<", ">"].includes(w)) {
                                linea.pop()
                                linea[linea.length] = ope(w+o, x-1)
                                linea[linea.length-1].conditional = true;
                            
                            }else {
                                linea[linea.length] = ope(o, x);

                            }
                             
                        } else {
                            linea[linea.length] = ope(o, x);
                        }
                    } else {
                        if (a == "var") {
                            if (cadena !="") {
                                linea[linea.length] = name(cadena, x-cadena.length);
                            }
                        } else {
                            linea[linea.length] = val(cadena, a, "", x-cadena.length);
                        };
                        linea[linea.length] = ope(o, x);
                        cadena = "";
                    };
                    let ab = linea[linea.length-1] || {};
                    
                    if (ab.c == ">") {
                        let q = linea[linea.length-2] || {};
                        if (q.tipo == "name") {
                            let qq = linea[linea.length-3] || {};
                            if (qq.c == "<") {
                                cadena = "<" + q.nm + ">";
                                tag = q.nm;
                                linea.pop();
                                linea.pop();
                                linea.pop();
                                mode = "tag";
                                count = 0;
                            }   
                        }
                    }
                    
                    
                }else if (toke.sim.includes(o)) {
                    var a= enterval(cadena)
                    if (a == "var") {
                        if (cadena !="") {
                            linea[linea.length] = name(cadena, x-cadena.length)
                        }
                    } else {
                        linea[linea.length] = val(cadena, a, "", x-cadena.length)
                    };
                    linea[linea.length] = sim(o, x);
                    if ((linea[linea.length-1]||{}).c == "{"){
                        if ((linea[linea.length-2]||{}).c == "{"){
                            linea.pop();
                            linea.pop();
                            mode = "str-{}";
                        }
                    }
                    cadena = "";
                }else if (o == "$") {

                    var a= enterval(cadena)
                    
                    if (a == "var") {
                        if (cadena !="") {
                            linea[linea.length] = name(cadena, x-cadena.length)
                        }
                    } else {
                        linea[linea.length] = val(cadena, a, "", x-cadena.length)

                    };

                    mode = "c-$"

                    cadena = "";
                    
                }else if (o == "'") {

                    Byte = cadena;

                    mode = "str-a"

                    cadena = "'";
                    
                }else if (o == '"') {

                    Byte = cadena;

                    mode = "str-c"

                    cadena = '"';
                    
                }else if (o == "#") {

                    

                    if (cadena != "") {

                        var a= enterval(cadena)
                        
                        if (a == "var") {
                            if (cadena !="") {
                                linea[linea.length] = name(cadena, x-cadena.length);
                            }
                        } else {
                            linea[linea.length] = val(cadena, a, "", x-cadena.length)
                
                        };
                        cadena = "";
                        
                    }
                    
                    /*if (linea +"" != "[]") {
                        salida[salida.length] = new ins(linea);
                        salida[salida.length-1].use_lines(st, nl);
                        st = nl;
                        
                        linea = [];
                    } */

                    mode = "c-#"

                    cadena = "";
                    
                }else {
                    //si no es ningun simbolo reservado entonces que lo meta a la cadena ("tokens")
                    cadena = cadena + o;
                }
            } else {
                var a= enterval(cadena)
                
                if (a == "var") {
                    if (cadena !="") {
                        linea[linea.length] = name(cadena)
                    }
                    
                } else {
                    linea[linea.length] = val(cadena, a)

                };
                cadena = "";
            }
        }else if (mode == "c-#") {
            if (o == R) {
                mode = "normal"
                nl = 1+nl;
            }
        }else if (mode == "c-$") {
            if (o == "$") {
                mode = "normal";
            } else if (o == R) {
                nl = 1+nl;
            } 


        }else if (mode == "c-/**/") {
            if (o == "/") {
                if (chars[x-1] == "*") {
                    
                    mode = "normal";
                }
            } else if (o == R) {
                nl = 1+nl;
            } 


        }else if (mode == "str-a") {
            if (o == "'") {
                cadena = cadena + "'";
                linea[linea.length] = val(cadena, "str", Byte, x-cadena.length)
                linea[linea.length-1].setString = "'"
                cadena = "";
                mode = "normal";
            } else if ([N,R].includes(o)) {
                if (o== N) {
                    cadena=cadena+BAR_INVERTD+"n"
                }
            } else {
                cadena=cadena+o
            }
        }else if (mode == "str-c") {
            if (o == '"') {
                cadena = cadena + '"';
                linea[linea.length] = val(cadena, "str", Byte, x-cadena.length)
                linea[linea.length-1].setString = '"'
                cadena = "";
                mode = "normal";
            } else if ([N,R].includes(o)) {
                if (o== N) {
                    cadena=cadena+BAR_INVERTD+"n"
                }
            } else {
                cadena=cadena+o
            }
        }else if (mode == "str-{}") {
            if (o == '}') {
                if (chars[x-1] == "}") {
                    //cadena = cadena.substr(0, cadena.length);
                    linea[linea.length] = val('"'+cadena+ '"', "str", x-cadena.length)
                    linea[linea.length-1].setString = '"'
                    cadena = "";
                    mode = "normal";
                }
            } else if ([N,R].includes(o)) {
                if (o== N) {
                    cadena=cadena+BAR_INVERTD+"n"
                }
            }else if (o=='"') {
                cadena=cadena+BAR_INVERTD+'"'
            } else {
                cadena=cadena+o
            }
        }else if (mode == "tag") {
            
            if (o == '>') {
                cadena = cadena + o;
                if (cadena.substr(cadena.length-`</${tag}>`.length) == `</${tag}>`) {
                    count = count-1
                };

                if (count == -1) {
                    linea[linea.length] = cml(cadena, x-cadena.length);
                    cadena = "";
                    mode = "normal";
                }
                
                
            } else if ([N,R,T].includes(o)) {

            } else {
                cadena=cadena+o
                if (cadena.substr(cadena.length-`<${tag}`.length) == `<${tag}`) {
                    count = count+1
                };
                
            }
        }

    }

  
    //en caso de que haya valores restantes que sean integrados al codigo

    if (cadena != "") {

        var a= enterval(cadena)
        
        if (a == "var") {
            linea[linea.length] = name(cadena)
        } else {
            linea[linea.length] = val(cadena, a)

        };
        cadena = "";
        
    }
    
    if (linea +"" != "[]") {
        salida[salida.length] = new ins(linea);
        salida[salida.length-1].use_lines(st, nl);
        st = nl;
        
        linea = [];
    } 

    //console.timeLog("run")
    //[0])
    ////////////////////////////limpiar el codigo y eliminar la fragmentacion y espacios en blancos
    //var linea = [];
    //var dsalida = [];

   

    //console.timeLog("run")

    //salida = dsalida



    return salida

};

let miato = []


var codigo_saliente = ""
let univars = {
    default:"any"
}

function parselex(code) {

    let salida = [];
    let modo_lex = "normal"; 
    let key = [];
    let fcode= filenames[filenames.length-1]
    let iter = 0;
    

    for (let c = 0; c < code.length; c++) {
        var ele
        try {
            
            ele = code[c].get();
            //const st = code[c].log[0];
            //const nl = code[c].log[1];
   
        } catch (e) {
            ele = code[c]
        }

        
        if (!(["{}", "[]", "()"].includes(modo_lex))) {
            var linea = [];
            var cadena = [];
            var cont=0    
        }
        
        
        miato = ele;
        for (let k = 0; k < ele.length; k++) {
            const kill = ele[k];


            if (modo_lex == "normal") {

                if (kill.tipo == 'simbols') {

                    if (kill.c == "(") {
                        cont = 1;
                        key = [];
                        modo_lex = "()";
                        iter = ele.i;

                    } else if (kill.c == "[") {
                        cont = 1;
                        key = [];
                        modo_lex = "[]";
                        iter = ele.i;
                    } else if (kill.c == "{") {
                        cont = 1;
                        key = [];
                        modo_lex = "{}";
                        iter = ele.i;

                    } else if (kill.c == ")") {
                        msgerror("sobre exposicion de tokens '" +kill.c+ "'", decode_cls([ele, fcode]))
                    } else if (kill.c == "]") {
                        msgerror("sobre exposicion de tokens '" +kill.c+ "'", decode_cls([ele, fcode]))
                        
                    } else if (kill.c == "}") {
                        msgerror("sobre exposicion de tokens '" +kill.c+ "'", decode_cls([ele, fcode]))
                        
                    } else {
                        linea[linea.length] = kill
                    }
                    
                } else {
                    linea[linea.length] = kill
                }

            } else if (modo_lex == "()") {

                if(kill.c == ")"){
    
                    cont = cont -1
    
                    if(cont == 0){
                        if (cadena.length>0) key[key.length] =cadena;
                        let lit = parselex(key)
                        let slida = {data:lit[0]||[], tipo:"()", complet:lit, i:iter}
                        linea[linea.length] = slida
                        cadena = []
                        modo_lex = "normal"
                        
                    }else {
                        cadena[cadena.length] = kill
                    }
    
                } else if(kill.c == "("){
    
                    cont = cont +1
                    cadena[cadena.length] = kill
    
                }else {
    
                    cadena[cadena.length] = kill
    
                }
    
            } else if (modo_lex == "[]") {

                if(kill.c == "]"){
    
                    cont = cont -1
    
                    if(cont == 0){
                        if (cadena.length>0) key[key.length] =cadena;
                        let lit = parselex(key);
                        let slida = {data:lit[0]||[], tipo:"[]", complet:lit, i:iter};
                        linea[linea.length] = slida
                        cadena = []
                        modo_lex = "normal"
                        
                    }else {
                        cadena[cadena.length] = kill
                    }
    
                } else if(kill.c == "["){
    
                    cont = cont +1
                    cadena[cadena.length] = kill
    
                }else {
    
                    cadena[cadena.length] = kill
    
                }
    
            } else if (modo_lex == "{}") {

                if(kill.c == "}"){
    
                    cont = cont -1
    
                    if(cont == 0){
                        if (cadena.length>0) key[key.length] =cadena;
                        let ili = parselex(key);
                        if (ili.length == 0) ili=[[]]
                        let slida = {data:ili, tipo:"code", i:iter}
                        linea[linea.length] = slida
                        cadena = []
                        modo_lex = "normal"
                        
                    }else {
                        cadena[cadena.length] = kill
                    }
    
                } else if(kill.c == "{"){
    
                    cont = cont +1
                    cadena[cadena.length] = kill
    
                }else {
    
                    cadena[cadena.length] = kill
    
                }
    
            }

            
        }

        if (cadena.length > 0 | linea.length > 0) {
            if (modo_lex=="{}") {
                key[key.length] = cadena;
                cadena = [];
                
            } else if (modo_lex=="()") {
                key[key.length] = cadena;
                cadena = [];
                
            } else if (modo_lex=="[]") {
                key[key.length] = cadena;
                cadena = [];
                
            } else if (modo_lex=="normal") {
                salida[salida.length] = linea;
                linea = [];
                
            }
        }
        //codigo_saliente = codigo_saliente +";"+ N + linea 
        //dsalida[dsalida.length] = new ins(linea)
        //dsalida[dsalida.length-1].use_lines(st, nl);

    }
    if (modo_lex=="()") {
        msgerror("sobre exposicion de tokens '('", decode_cls([ele, fcode]))
        
    }
    if (modo_lex=="[]") {
        msgerror("sobre exposicion de tokens '['", decode_cls([ele, fcode]))
        
    }
    if (modo_lex=="{}") {
        msgerror("sobre exposicion de tokens '{'", decode_cls([ele, fcode]))

        
    }else{
        return(salida)

    }

    //return (codigo_saliente)
}


function enterval(code  ="") {
    salida = "";

    
    
    if (code.replace(".", " ") == code) {
        
        salida = "int"
        a = parseInt(code)
        if (a +"" != code) {

            salida = "var"
            
        }


    } else {
        salida = "float"
        a = parseFloat(code)
        if (a +"" != code) {

            salida = "var"
            
        }
    }

    

    return salida
};


var eventos = {
    load:"",
    exit:"",
    input:{
    },
    output:{
    },
}

let trys = 0;
let valido = "";

function parse_args(data, conf) {
    let mode_fun = "normal";
    let argumentos = [];
    let estaticos = [];
    let vals = [];
    

    let dim_arg = {nm:univars.default};
    let def_val = defval_void();
    let nm_arg = undefined;

    for (let y = 0; y < data.length; y++) {
        const o = data[y];
        if (mode_fun == "normal") {
            if (o.tipo == "name") {
                nm_arg = o
                
                mode_fun = "com";
            }
            
        }else if (mode_fun == "com") {//una salida por coma
            if (o.c == ",") {
                argumentos[argumentos.length] = nm_arg;
                vals[vals.length] = def_val;
                estaticos[estaticos.length] = dim_arg;
                dim_arg = {nm:univars.default};
                nm_arg = undefined;
                def_val = defval_void();
                mode_fun = "normal";
            } else if (o.c == ":") {
                mode_fun = "sta"
            } else if (o.c == "=") {
                mode_fun = "deval"
            }
            
        }else if (mode_fun == "sta-post") {//salida por coma
            if (o.c == ",") {
                argumentos[argumentos.length] = nm_arg;
                vals[vals.length] = def_val;
                estaticos[estaticos.length] = dim_arg;
                dim_arg = {nm:univars.default};
                nm_arg = undefined;
                def_val = defval_void();
                mode_fun = "normal";
            } else if (o.c == "=") {
                mode_fun = "deval"
            }
            
        }else if (mode_fun == "sta") {
            if (o.tipo == "name") {
                dim_arg = o
                
                mode_fun = "sta-post";
            }
            
        }else if (mode_fun == "deval") {
            
            def_val = [o];
            
            mode_fun = "coma-only";
                                                    
        }else if (mode_fun == "coma-only") {//salida por coma
            if (o.c == ",") {
                argumentos[argumentos.length] = nm_arg;
                estaticos[estaticos.length] = dim_arg;
                vals[vals.length] = def_val;
                dim_arg = {nm:univars.default};
                nm_arg = undefined;
                def_val = defval_void();
                mode_fun = "normal";
            }
            
        }
    };
    if (nm_arg != undefined) {
        argumentos[argumentos.length] = nm_arg;
        estaticos[estaticos.length] = dim_arg;
        vals[vals.length] = def_val;
    };
    return(
        {
            arg:argumentos,
            sta:estaticos,
            val:vals
        }
    )
};
let defval_void = () => {
    return([name("null")])
};

function one_estructuration(code, fal, valio) {
    let salida = [];
    function redata(e, t) {
        let out = []
        for (let i = 0; i < e.complet.length; i++) {
            const x = e.complet[i];
            out.push(
                one_estructuration(x, fal, valio)
            )
        }
        let final= ({
            data:out[0]||[],//one_estructuration(e.data),
            tipo:t,
            i:e.i,
            complet:out
        });
        return final;
    };
    for (let i = 0; i < code.length; i++) {
        let e = code[i];
        if (e.tipo == "()") {
            
            if ((code[i+1]||{}).c == "->") {
                if ((code[i+2]||{}).tipo == "name") {
                    if ((code[i+3]||{}).tipo == "code") {
                        let mat = parse_args(e.data);
                        let cod = {
                            tipo:"code",
                            data:estructuration(code[i+3].data, fal, valio)
                        };
                        let asyncrono = false;
                        if ((code[i-1]||{}).tipo == "name") {
                            if (["sync", "async"].includes(code[i-1].nm)) {
                                asyncrono = (code[i-1].nm==="async");
                                salida.pop()
                            }
                        }
                        let stat = code[i+2].nm;
                        let tmp_prt = {
                            tipo:"func-a",
                            argv:mat.arg,
                            code:cod,
                            rt:stat,
                            sta:mat.sta,
                            defval:mat.val,
                            async:asyncrono
                        }
                        salida.push(tmp_prt)
                        i = i+3

                    } else {
                        salida.push(redata(e, "()"))

                    }
                }else if ((code[i+2]||{}).tipo == "code") {
                    let mat = parse_args(e.data);
                    let cod = {
                        tipo:"code",
                        data:estructuration(code[i+2].data, fal, valio)
                    };
                    let stat = univars.default;
                    let asyncrono = false;
                    if ((code[i-1]||{}).tipo == "name") {
                        if (["sync", "async"].includes(code[i-1].nm)) {
                            asyncrono = (code[i-1].nm==="async");
                            salida.pop()
                        }
                    }
                    let tmp_prt = {
                        tipo:"func-a",
                        argv:mat.arg,
                        code:cod,
                        rt:stat,
                        sta:mat.sta,
                        defval:mat.val,
                        async:asyncrono
                    }
                    salida.push(tmp_prt)
                    //console.log(tmp_prt)
                    i = i+2
                } else {
                    salida.push(redata(e, "()"))
                }
            } else {
                salida.push(redata(e, "()"))

            }
        } else if (e.tipo == "[]") {
            salida.push(redata(e, "[]"))
        } else if (e.tipo == "code") {
            let tmp = Object.assign({}, e);
            tmp.data[0] = one_estructuration(tmp.data[0], fal, valio)
            salida.push(tmp)
        } else {
            salida.push(e)
        }
    }
    return salida
};

function estructuration(code, fal, valio) {
    //debes de hacer uso de "func-a"
    let salida = [];
    var mode_structure = "normal"
    let fcode= filenames[filenames.length-1]
    function redata(e, t) {
        let out = []
        for (let i = 0; i < e.complet.length; i++) {
            const x = e.complet[i];
            out.push(
                one_estructuration(x, fal, valio)
            )
        }
        let final= ({
            data:out[0]||[],//one_estructuration(e.data),
            tipo:t,
            i:e.i,
            complet:out
        });
        return final;
    };
    if (valido == "") {
        valido = valio
    }
    if (valio == undefined) {
        valio = valido
    }
    if (fal == undefined) {
        fal = false
    }
    let prefijos = ["public", "static", "export", "private"]
    let nombres_reservados = [
        "func", "function", "class", "for", "try", "import", "module", "if", "from", "include", "return", 
        "global", "with", "export", "define", "public", "static", "export", "private", "protected"
    ];
    let not_tokens_old = [
        "class", "try", "import", "module", "from", "global", "with", "public", "static",
        "extend", "static", "private", "protected"
    ]
    for (let i = 0; i < code.length; i++) {
        let e = code[i];
        let extend = true;
        let prefijo= "public";
        let asyncrone = false;
        if (mode_structure == "normal") {
            if (e.length > 1) {
                (async () =>{})
                
                //prefijos
                if (e[0].tipo == "name") {
                    //console.log("emu: " + e[0].nm)
                    if (prefijos.includes(e[0].nm)) {
                        prefijo = e[0].nm
                        e=e.slice(1)                    
                    }
                }

                if (e[0].tipo == "name") {
                    //console.log("emu: " + e[0].nm)
                    if (["sync", "async"].includes(e[0].nm)) {
                        asyncrone = "async" === e[0].nm
                        prefijo = e[0].nm
                        e=e.slice(1)
                    }
                }

                
                // funcion prefija
                if (e.length == 4) { 
                    if (e[0].tipo == "name") {


                        if (!(nombres_reservados.includes(e[0].nm))){
                            if (e[1].tipo == "name") {
                                if (e[2].tipo == "()") {
                                    if (e[3].tipo == "code") {
                                        e= [name("func"), e[1], e[2], ope("->"), e[0], e[3]]
                                    }
                                }
                            }
                        }
                    }
                };

                if (mode_cls === "cls-beta") if (not_tokens_old.includes(e[0])) {
                    msgerror(`"${e[0].nm}" no esta soporta en la vieja version de cls`, decode_cls([e, fcode]))
                };
                //instruccion
                if (e[0].tipo == "name") {
                    if (e[0].nm == "func" | e[0].nm == "function") {
                        if (e[1].tipo == "name") {

                            if (e[2].tipo == "()") {

                                if (e[3].tipo == "code") {
                                    if (e[4]) {
                                        msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                                    }
                                    
                                    let stat = univars.default;
                                    
                                    let mat = parse_args(e[2].data);

                                    let tmp_pio = {
                                        tipo:"code",
                                        data:estructuration(e[3].data)
                                    }

                                    tmp_prt = {
                                        tipo:"func-def",
                                        name:e[1].nm,
                                        argv:mat.arg,
                                        code:tmp_pio,
                                        rt:stat,
                                        sta:mat.sta,
                                        defval:mat.val,
                                        visible:prefijo,
                                        async:asyncrone
                                    }
                                    salida[salida.length] = [tmp_prt];
                                } else if (e[3].c == "->") {
                                    if (e[4].tipo == "name") {
                                        
                                        if (e[5].tipo == "code") {
                                            if (e[6]) {
                                                msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                                            }
                                            
                                            let stat = e[4].nm;
                                            
        
                                            let mat = parse_args(e[2].data)
        
                                            let tmp_pio = {
                                                tipo:"code",
                                                data:estructuration(e[5].data)
                                            }
        
                                            tmp_prt = {
                                                tipo:"func-def",
                                                name:e[1].nm,
                                                argv:mat.arg,
                                                code:tmp_pio,
                                                rt:stat,
                                                sta:mat.sta,
                                                defval:mat.val,
                                                visible:prefijo,
                                                async:asyncrone
                                            }
                                            salida[salida.length] = [tmp_prt];
                                        } else {
                                            msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                                        }
                                    } else {
                                        msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                                    }
                                } else {
                                    msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                                }

                            }else {
                                msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                            }

                        }else {
                            msgerror("error al momento de estructurar una funcion", decode_cls([e, fcode]))
                        }
                        
                    } else if (e[0].nm == "class") {
                        
                        if (e[1].tipo == "name") {

                            if (e[2].tipo == "()") {

                                if (e[3].tipo == "code") {
                                    if (e[4]) {
                                        msgerror("error al momento de estructurar una clase", decode_cls([e, fcode]))
                                    }
                                    let mode_fun = "normal";
                                    let argumentos = [];

                                    for (let y = 0; y < e[2].data.length; y++) {
                                        const o = e[2].data[y];
                                        if (mode_fun == "normal") {
                                            if (o.tipo == "name") {
                                                argumentos[argumentos.length] = o
                                                
                                                mode_fun = "coma";
                                            }
                                            
                                        }else if (mode_fun == "coma") {
                                            if (o.c == ",") {
                                                
                                                
                                                mode_fun = "normal";
                                            }
                                            
                                        }
                                    }
                                    

                                    let tmp_pio = {
                                        tipo:"code",
                                        data:estructuration(e[3].data)
                                    }

                                    tmp_prt = {
                                        tipo:"class-def",
                                        name:e[1].nm,
                                        code:tmp_pio,
                                        ext:argumentos,
                                        visible:prefijo
                                    }
                                    salida[salida.length] = [tmp_prt];
                                }else {
                                    msgerror("error al momento de estructurar una clase", decode_cls([e, fcode]))
                                }

                            }else {
                                msgerror("error al momento de estructurar una clase", decode_cls([e, fcode]))
                            }

                        }else {
                            msgerror("error al momento de estructurar una clase", decode_cls([e, fcode]))
                        }
                        
                    } else if (e[0].nm == "module") {
                        if (e[1].tipo == "name") {

                            if (e[2].tipo == "code") {
                                if (e[3]) {
                                    msgerror("error al momento de estructurar un modulo", decode_cls([e, fcode]))
                                }
                                
                                

                                let tmp_pio = {
                                    tipo:"code",
                                    data:estructuration(e[2].data)
                                }
                                
                                tmp_prt = {
                                    tipo:"module-def",
                                    name:e[1].nm,
                                    code:tmp_pio,
                                    visible:prefijo
                                }
                                salida[salida.length] = [tmp_prt];
                            }else {
                                msgerror("error al momento de estructurar un modulo", decode_cls([e, fcode]))
                            }

                        }else {
                            msgerror("error al momento de estructurar un modulo", decode_cls([e, fcode]))
                        }
                        
                    } else if (e[0].nm == "if") {
                        let condiciones =[];
                        let startif = {}
                        
                        if (e[1].tipo == "()") {
                            if (e[2].tipo == "code") {
                                let tmp_pio = {
                                    tipo:"code",
                                    data:estructuration(e[2].data),
                                }
                                
                                startif = {
                                    cond:redata(e[1], "()"),
                                    code:tmp_pio
                                };
                            } else {
                                msgerror("error al intentar estructurar un if", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error al intentar estructurar un if", decode_cls([e, fcode]))
                        }
                        
                        let endif = {
                            code:{data:[[]],tipo:"code"}
                        }
                        //let mode_condition
                        for (let p = 3; p < e.length; p=p+3) {
                            //const o = e[p];

                            if (e[p].nm == "elseif" | e[p].nm == "elif") {
                                if (e[p+1].tipo == "()") {
                                    if (e[p+2].tipo == "code") {
                                        let tmp_pio = {
                                            tipo:"code",
                                            data:estructuration(e[2+p].data)
                                        }
                                        condiciones[condiciones.length] = {
                                            cond:redata(e[p+1], "()"),
                                            code:tmp_pio
                                        };
                                    } else {
                                        msgerror("error al intentar estructurar un if", decode_cls([e, fcode]))
                                    }
                                } else {
                                    msgerror("error al intentar estructurar un if", decode_cls([e, fcode]))
                                }
                            } else if (e[p].nm == "else") {
                                if (e[p+1].tipo == "code") {
                                    if (e[p+2]) {
                                        msgerror("error al momento de estructurar un if", decode_cls([e, fcode]))
                                    }
                                    let tmp_pio = {
                                        tipo:"code",
                                        data:estructuration(e[1+p].data)
                                    }
                                    
                                    endif = {
                                        
                                        code:tmp_pio
                                    };
                                    p=e.length;
                                    
                                } else {
                                    msgerror("error al intentar estructurar un if", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error al intentar estructurar un if (operador no valido)", decode_cls([e, fcode]))

                            }

                        }

                        salida[salida.length] = [{
                            tipo:"if-def",
                            onif:startif,
                            elif:condiciones,
                            outif:endif,
                            i:e[0].i
                        }]
                    } else if (e[0].nm == "while") {
                        if (e[1].tipo == "()") {
                            if (e[2].tipo == "code") {
                                if (e[3]) {
                                    msgerror("error al intentar estructurar un while", decode_cls([e, fcode]))
                                }
                                let tmp_pio = {
                                    tipo:"code",
                                    data:estructuration(e[2].data)
                                }



                                salida[salida.length] = [{
                                    tipo:"while-def",
                                    cond:redata(e[1], "()"),
                                    code:tmp_pio
                                }]
                            } else {
                                msgerror("error al intentar estructurar un while", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error al intentar estructurar un while", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "for") {
                        if (e[1].nm == "each") {
                            if (e[2].tipo == "name") {
                                
                                if (e[3].tipo == "()") {
                                    if (e[4].tipo == "code") {
                                        if (e[5]) {
                                            msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                                        }
                                        let tmp_pio = {
                                            tipo:"code",
                                            data:estructuration(e[4].data)
                                        }
                                        salida[salida.length] = [{
                                            tipo:"for-e-def",
                                            iterador:e[2],
                                            valor:redata(e[3], "()"),//e[3],
                                            code:tmp_pio
                                        }]
                                    } else {
                                        msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                                    }
                                } else {
                                    msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                            }
                            
                        } else if (e[1].tipo == "name") {
                                
                            if (e[2].tipo == "()") {
                                if (e[3].tipo == "code") {
                                    if (e[4]) {
                                        msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                                    }
                                    let tmp_pio = {
                                        tipo:"code",
                                        data:estructuration(e[3].data)
                                    }
                                    salida[salida.length] = [{
                                        tipo:"for-def",
                                        iterador:e[1],
                                        valor:redata(e[2], "()"),//e[2],
                                        code:tmp_pio
                                    }]
                                } else {
                                    msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error al intentar estructurar un for", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "with") {
                        if (e[1].tipo == "name") {
                                
                            if (e[2].tipo == "()") {
                                if (e[3].tipo == "code") {
                                    if (e[4]) {
                                        msgerror("error al intentar estructurar un with", decode_cls([e, fcode]))
                                    }
                                    let tmp_pio = {
                                        tipo:"code",
                                        data:estructuration(e[3].data)
                                    }
                                    salida[salida.length] = [{
                                        tipo:"with-def",
                                        name:e[1],
                                        valor:redata(e[2], "()"),//e[2],
                                        code:tmp_pio
                                    }]
                                } else {
                                    msgerror("error al intentar estructurar un with", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error al intentar estructurar un with", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error al intentar estructurar un with", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "import") {
                        if (e[1].tipo == "val" & e[1].tip == "str") {
                            if (e[2].nm == "as") {
                                if (e[3].tipo == "name") {
                                    if (e[4]) {
                                        msgerror("error de sintaxis, verificar la estructura de un import", decode_cls([e, fcode]))
                                    }
                                    salida[salida.length] = [
                                        {
                                            tipo:"import",
                                            src:e[1].valy,
                                            name:e[3].nm
                                        }
                                    ]
                                } else {
                                    msgerror("error de sintaxis, verificar la estructura de un import", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error de sintaxis, verificar la estructura de un import", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error de sintaxis, verificar la estructura de un import", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "from") {
                        if (e[1].tipo == "val" & e[1].tip == "str") {
                            if (e[2].nm == "import") {
                                if (e[3].tipo == "name") {
                                    if (e[4].nm == "as") {
                                        if (e[5].tipo == "name") {
                                            if (e[6]) {
                                                msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                                            }
                                            salida[salida.length] = [
                                                {
                                                    tipo:"f-import",
                                                    src:e[1].valy,
                                                    imp:e[3].nm,
                                                    name:e[5].nm,
                                                }
                                            ]
                                        } else {
                                            msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                                        }
                                    } else {
                                        msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                                    }
                                } else {
                                    msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("error de sintaxis, verificar la estructura de un from-import", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "$define") {
                        if (e[1].nm == "event") {

                            try {
                                if (e[2].tipo == "name") {
                                    if (e[3].tipo == "name") {
                                        
                                        eval(`eventos.${e[2].nm} = '${valio}.${e[3].nm}'`)
                                    
                                    } else {
                                        msgerror(`hubo un error al intentar definir el evento '${e[2].nm}' (error de sintaxis)`, decode_cls([e, fcode]))

                                    }
                                    
                                } else {
                                    msgerror(`hubo un error al intentar definir el evento '${e[2].nm}'`, decode_cls([e, fcode]))

                                }
                            } catch (error) {
                                msgerror(`hubo un error al intentar definir el evento '${e[2].nm}'`, decode_cls([e, fcode]))
                            }

                        }
                    } else if (e[0].nm == "include") {
                        if (e[1].tipo == "val" & e[1].tip == "str") {
                            if (e[2]) {
                                msgerror("error de sintaxis al intentar incluir un pedazo de codigo por include", decode_cls([e, fcode]))
                            }
                            salida[salida.length] = [
                                {
                                    tipo:"include",
                                    src:e[1].valy
                                }
                            ]
                        } else {
                            msgerror("error al incluir un pedazo de codigo (sintaxis invalida)", decode_cls([e, fcode]))
                        }
                    } else if (e[0].nm == "return"){
                        salida[salida.length] = [
                            {
                                tipo:"return",
                                eval:one_estructuration(e.slice(1), fal, valio)
                            }
                        ]
                    } else if (e[0].nm == "var"){
                        let out = {};

                        if ((e[1]||{}).tipo==="()") {out = parse_args(e[1].data)}
                        else {out = parse_args(e.slice(1))}
                        
                        salida[salida.length] = [
                            {
                                tipo:"all-var-def",
                                argv:out.arg,
                                sta:out.sta,
                                defval:out.val
                            }
                        ]
                    } else if (e[0].nm == "try") {
                        if (e[1].tipo == "code") {
                            if (e[2].tipo == "name") {
                                if (e[2].nm == "error" | e[2].nm == "catch" | e[2].nm == "except") {
                                    if (e[3].tipo == "name") {
                                        if (e[4].tipo == "code") {
                                            if (e[5]) {
                                                msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                                            }
                                            let tmp_try = {
                                                tipo:"code",
                                                data:estructuration(e[1].data)
                                            }
                                            let tmp_error = {
                                                tipo:"code",
                                                data:estructuration(e[4].data)
                                            }

                                            salida[salida.length] = [{
                                                code_try:tmp_try,
                                                code_error:tmp_error,
                                                tipo:"try",
                                                vare:e[3].nm
                                            }]
                                            
                                        } else {
                                            msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                                        }
                                    } else {
                                        msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                                    }
                                } else {
                                    msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                                }
                            } else {
                                msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                            }
                        } else {
                            msgerror("mala estructuracion de un try", decode_cls([e, fcode]))
                        }
                    } else if (e[1].c == "=") {
                        extend = false
                        salida[salida.length] = [{
                            tipo:"var-def",
                            name:e[0].nm,
                            eval:one_estructuration(e.slice(2), fal, valio),
                            visible:prefijo
                        }]
                    } else if (e[0].nm == "global") {
                        if (e[2].c == "=") {
                            salida[salida.length] = [{
                                tipo:"global-def",
                                name:e[1].nm,
                                eval:one_estructuration(e.slice(3), fal, valio)
                            }]
                        } else {
                        
                            let kito = Array_nodes(e, ope("="));
                            if (kito[0]) {
                            
                                salida.pop()
                                salida[salida.length] = [{
                                    tipo:"global_extend-def",
                                    subname:one_estructuration(e.slice(1, kito[1]), fal, valio),
                                    eval:one_estructuration(e.slice(kito[1]+1), fal, valio)
                                }];
                            
                                
                            } else {
                                msgerror("error al intentar declarar una variable global 'error de sintaxis'", decode_cls([e, fcode]))

                            }
                        
                        }
                        
                    } else if (e[0].nm == "export") {
                    
                        salida[salida.length] = [{
                            tipo:"export-def",
                            name:e[1].nm,
                            eval:one_estructuration(e.slice(3), fal, valio)
                        }]
                    
                        
                    } else if (e[1].c == ":") {
                        let def = false;
                        if (e.length > 3) {
                            if (e[3].c == "=") {
                                def = e.slice(4);
                                
                            }else {
                                msgerror("Error de sintaxis", decode_cls([e, fcode]))
                            }
                        };
                        salida[salida.length] = [{
                            tipo:"dim-def",
                            name:e[0].nm,
                            dim:e[2].nm,
                            eval:def,
                            visible:prefijo
                        }]
                    } else if (e[1].nm == "as") {
                        
                        salida[salida.length] = [{
                            tipo:"dim-def",
                            name:e[0].nm,
                            dim:e[2].nm,
                            visible:prefijo

                        }]
                    } else if (e[1].c == "->") {
                        
                        salida[salida.length] = [{
                            tipo:"flecha-def",
                            name:parse_args(e.slice(2)).arg,
                            dim:e[0].nm,
                            visible:prefijo

                        
                        }]
                    } else {
                        salida[salida.length] = one_estructuration(e, fal, valio);

                        
                        //var_extend-def

                    
                        
                        if (extend) {
                            let kito = Array_nodes(e, ope("="));
                            if (kito[0]) {
                                if (e[0].nm == "global") {
                                    salida.pop()
                                    salida[salida.length] = [{
                                        tipo:"global_extend-def",
                                        subname:one_estructuration(e.slice(1, kito[1]), fal, valio),
                                        eval:one_estructuration(e.slice(kito[1]+1), fal, valio)
                                    }];
                                } else {
                                    salida.pop()
                                    salida[salida.length] = [{
                                        tipo:"var_extend-def",
                                        subname:one_estructuration(e.slice(0, kito[1]), fal, valio),
                                        eval:one_estructuration(e.slice(kito[1]+1), fal, valio)
                                    }];
                                    
                                }
                                
                            }
                        }
                        

                        
                        
                    }
                } else if ((e[0]||{}).tipo == "[]") {
                    salida[salida.length] = one_estructuration(e, fal, valio);
                    
                } else if ((e[0]||{}).tipo == "()") {
                    salida[salida.length] = one_estructuration(e, fal, valio);
                    
                }
            } else if ((e[0]||{}).tipo == "val") {
                salida[salida.length] = one_estructuration(e, fal, valio);
                
            } else if ((e[0]||{}).tipo == "name") {
                salida[salida.length] = one_estructuration(e, fal, valio);
                
            } else if ((e[0]||{}).tipo == "[]") {
                salida[salida.length] = one_estructuration(e, fal, valio);
                
            } else if ((e[0]||{}).tipo == "()") {
                salida[salida.length] = one_estructuration(e, fal, valio);
                
            }
        }

        
        
    }

    return(salida)
    
}
var workspace = {}
var plt = {}
function addworkspace(obj, name) {
    name = name||"clstd";
    obj = eval(obj)
    let obje =workspace[name] || {};
    for (let i = 0; i < Object.keys(obj).length; i++) {
        const e = Object.keys(obj)[i];

        obje[e] = obj[e]
        
    }

    workspace[name] = Object.assign(obje, defstd());
    let objei = {}

    for (let i = 0; i < Object.keys(obje).length; i++) {
        const e = Object.keys(obje)[i];

        objei[e] = obje[e]
        
    }

    
    plt[name] = Object.assign(objei, defstd());
}
let renderizados = []
let glb={};

class cadena {
    constructor(val){
        this.val = val + ""

    };
    get len() {
        return this.val.length
    };
    toString() {
        return this.val
    };
    __type__() {
        return "string"
    }
}


class entero {
    constructor(val){
        this.val = val

    };
    toString() {
        return this.val
    };
    __type__() {
        return "int"
    }
}

class decimal {
    constructor(val){
        this.val = val

    };
    toString() {
        return this.val
    };
    __type__() {
        return "float"
    }
}

function Array_nodes(array, node) {
    for (let i = 0; i < array.length; i++) {
        const e = array[i];
        if (e.tipo == node.tipo) {
            if (node.tipo== "simbols" | node.tipo== "operator") {
                if (e.c == node.c) {
                    return([true, i])
                }
            }
        }
    };
    return([false, 0])
}

let vfs = {};
function cmlobj(data) {
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
                        
                        e.arg[carg[x]] = tools.func.replace_chars(ele, viejo, nuevo.getString())
                    } else {
                        
                        e.arg[carg[x]] = tools.func.replace_chars(ele, viejo, nuevo.toString())
                    }

                };
                if (nuevo.__classname__ == "CML") {
                    e.inner = tools.func.replace_chars(e.inner, viejo, nuevo.getString());
                } else {
                    e.inner = tools.func.replace_chars(e.inner, viejo, nuevo.toString());                   
                }
                let ik = replace_data(JSON.parse(JSON.stringify(e.nodes)), viejo, nuevo);
                e.nodes = ik
                salida.push(e)
            } else if (typeof(e) == "string") {
                if (typeof(nuevo) == "string") {
                    let m = tools.func.replace_chars(e, viejo, nuevo);
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
                        
                        let m = tools.func.replace_chars(e, viejo, nuevo.toString());
                        
                        salida.push(m);
                        //console.log("valor:");
                        //);
                    }
                    
                    
                } else {
                    let m = tools.func.replace_chars(e, viejo, nuevo);
                    salida.push(m)
                }
            }

        };


        
        //)
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
};
function recml(data) {
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
                    let lii = recml(cadena);
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
};

let _str = {}

function uvaly(val) {
    
    if (val.tip=="str") {
        if (val.der== "") {
            if (val.iskey) {
                return val.valy    
            } else {
                return `${val.valy}`
            }
            
        } else {
            return ` (_str["${val.der}"](${val.valy})) `
        }
    } else {
        return `${val.valy}`
    }

    if (val.tip=="str") {
        if (val.der== "") {
            if (val.iskey) {
                return val.valy    
            } else {
                return `(std.str(${val.valy}))`
            }
            
        } else {
            return ` (_str["${val.der}"](${val.valy})) `
        }
    } else if (val.tip=="int") {
        return `(std.int(${val.valy}))`

    } else if (val.tip=="float") {
        return `(std.float(${val.valy}))`

    }


    return val.valy
};
function glparse(index, name_var, nick) {
    let salida = "";
    for (let i = 2; i <= index; i++) {
        salida = salida + `localvalue${i}.${nick} = ${name_var};${N}`
    };
    return(salida)
}

function funcanom(func, rt, asy) {
    rt=rt||{__classname__:"Any"};
    func.__classname__ ="Function";
    func.__type__="Function";
    func.__rt__=rt;
    func.__async__ = asy;
    anon_func.push(func)
    return(func)
}
let anon_func = [];
let af = 0;
function generator_one(code, variable, modo, iskey) {
    let codigo = "";
    iskey = iskey || false;
    if (modo == "func") {
        variable = variable.split("_")[0]
    }
    let va = (v, s) => {
        return(
            "workspace." + v + "." + s
        )
    }
    let tipao = ""
    for (let x = 0; x < code.length; x++) {
        let ele = code[x];
        if (tipao == ele.tipo & tipao != "()" & tipao != "[]") {
            if (code[x-1].nm == "new") {
                        
            } else if (code[x-1].nm == "or") {
                        
            } else if (code[x-1].nm == "and") {
                    
            } else if (ele.nm == "and") {
                    
            } else if (ele.nm == "or") {
                    
            } else {
                msgerror("Error de sintaxis, uso proporcionados de un tipo de token", decode_cls(linearmo[linearmo.length-1]))

            }
            tipao = ele.tipo;

        } else if (ele.tipo == "[]") {
            if (tipao == "name") {
                
                ele.is_index = true;
                
            };
            if (tipao == "[]") {
                
                ele.is_index = true;
                
            };
            
            tipao = ele.tipo;
        } else {
            if (tipao == "()" | tipao == "[]" | tipao == "val") {
                if (ele.tipo == "name") {
                    if (ele.nm.substr(0,1) == ".") {
                        
                    } else if (ele.nm == "or") {

                    } else if (ele.nm == "and") {

                    } else {
                        msgerror("Error de sintaxis, uso proporcionados de un tipo de token", decode_cls(linearmo[linearmo.length-1]))
                        
                    }
                } else {
                    //msgerror("Error de sintaxis, uso proporcionados de un tipo de token")
                    
                }
            } else {
                if (ele.tipo == "[]") {
                    ele["is_index"] = true;
                    
                }
            }
            tipao = ele.tipo;
        }

        if (iskey) {
            if (ele.tipo == "name") {
                let sip = (code[x+1] || {})
                let nombre = ele.nm
                if (sip.c == ":") {
                    ele = val("'" + nombre + "'", "str", "")
                    ele.iskey= true
                    
                }
            }
        }
    
        if (modo == "normal") {

            if (ele.tipo == "name") {
                
                
                if ((ele.nm+"").substr(0, 1) == ".") {

                    codigo = codigo + ` ${ele.nm} ` 
                    
                } else if ((ele.nm+"").substr(0, 3) == "me.") {

                    codigo = codigo + ` workspace.${variable}.${(ele.nm+"").substr(3)} ` 
                    
                } else if (ele.nm == "me") {
                    codigo = codigo + ` workspace.${variable} ` 
                    

                } else if ((ele.nm+"").substr(0, 8) == "private.") {

                    codigo = codigo + ` workspace.${variable}.${(ele.nm+"").substr(8)} ` 
                    
                } else if (ele.nm == "private") {
                    codigo = codigo + ` workspace.${variable} ` 
                    

                } else if ((ele.nm+"").substr(0, 7) == "global.") {

                    codigo = codigo + ` glb.${(ele.nm+"").substr(7)} ` 
                    
                } else if (ele.nm == "global") {
                    codigo = codigo + ` glb ` 
                    

                } else if (ele.nm == "new") {
                    codigo = codigo + ` new `

                } else if (ele.nm == "await") {
                    codigo = codigo + ` await `
                    
                } else if (ele.nm == "yield") {
                    codigo = codigo + ` yield `
                    
                } else if (ele.nm == "or") {
                    codigo = codigo + ` | `

                } else if (ele.nm == "and") {
                    codigo = codigo + ` & `

                } else {

                    codigo = codigo + ` workspace.${variable}.${ele.nm} `
                }


            } else if (ele.tipo == "val") {
                
                codigo = codigo + ` ${uvaly(ele)} `
                



            } else if (ele.tipo == "operator") {

                if (ele.c == "+" | ele.c == ":" | ele.c == "-" | ele.c == "/" | ele.c == "*" | ele.c == "%" | ele.c == "**"  | ele.c == "++" | ele.c == "--" | ele.c == "|") {
                    if (ele.c == "--") {
                        codigo = codigo + ` -1 `
                        
                    } else if (ele.c == "++") {
                        codigo = codigo + ` +1 `
                        
                    } else if (ele.c == "|") {
                        codigo = codigo + ` || `
                        
                    } else {
                        codigo =  codigo + ` ${ele.c} `
                    }
                } else if (ele.c == "==" | ele.c == "<=" | ele.c == ">=" | ele.c == "!=" | ele.c == "<" | ele.c == ">") {
                    codigo = codigo + ` ${ele.c} `
                } else if (ele.c == "!") {
                    codigo = codigo + `${ele.c} `
                }

            } else if (ele.tipo == "simbols") {
                if (ele.c == ",") {
                    codigo= codigo +`, `
                }
                
            } else if (ele.tipo == "()") {
                

                codigo = codigo + ` (${generator_one(ele.data, variable, "normal")}) `; 
                    
                
            } else if (ele.tipo == "[]") {
                //codigo = codigo + ` lista([${generator_one(ele.data, variable, "normal")}]) `; 
                

                if (!ele.is_index) {
                    codigo = codigo + ` std.array([${generator_one(ele.data, variable, "normal")}]) `; 
                    
                } else {
                    
                    codigo = codigo + ` [${generator_one(ele.data, variable, "normal")}] `; 
                }
            } else if (ele.tipo == "code") {
                codigo = codigo + `{'__classname__': 'Object', ${generator_one(ele.data[0], variable, "normal", true)}}`; 
                
            } else if (ele.tipo == "cml") {
                codigo = codigo + `cmlobj(${JSON.stringify(recml(ele.data), N, 4)})`; 
                
            } else if (ele.tipo == "func-a") {
                    
                let io_p = "";
                
                let varioble = variable.split("_")[0]
                
                for (let l = 0; l < ele.argv.length; l++) {
                    const elemento = ele.argv[l].nm;
                    ele.argv[l] = elemento
                    const tida = ele.sta[l].nm;
                    let valua = ele.defval[l];
                    if ((valua[0]).nm == "null") {valua = "nulo"};
                    
                    io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                    io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                     (
                         (${elemento}!=undefined?${elemento}:${
                            valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                        })
                     ), 
                     sta${multimodos.length+1}); ${N}`;
                }



                codigo= codigo +` funcanom( ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
let este = anon_func[${af++}];
varint.push(localvalue${multimodos.length+1});


${io_p}

${
    generator(ele.code.data, variable, "func")
}
}, workspace.${variable}.${ele.name}, ${ele.async?"true":"false"})`




            }

        } else if ( (modo == "func") | (modo == "func-imp")) {

            if (ele.tipo == "name") {
                
                
                if ((ele.nm+"").substr(0, 1) == ".") {

                    codigo = codigo + ` ${ele.nm} ` 
                    
                } else if ((ele.nm+"").substr(0, 3) == "me.") {

                    codigo = codigo + ` me.${(ele.nm+"").substr(3)} ` 
                    
                } else if (ele.nm == "me") {
                    codigo = codigo + ` me ` 
                    

                } else if ((ele.nm+"").substr(0, 8) == "private.") {

                    codigo = codigo + ` private.${(ele.nm+"").substr(8)} ` 
                    
                } else if (ele.nm == "private") {
                    codigo = codigo + ` private ` 
                    

                } else if ((ele.nm+"").substr(0, 7) == "global.") {

                    codigo = codigo + ` glb.${(ele.nm+"").substr(7)} ` 
                    
                } else if (ele.nm == "global") {
                    codigo = codigo + ` glb ` 
                    

                } else if (ele.nm == "await") {
                    codigo = codigo + ` await `
                    
                } else if (ele.nm == "yield") {
                    codigo = codigo + ` yield `
                    
                } else if (ele.nm == "new") {
                    codigo = codigo + ` new `

                } else if (ele.nm == "or") {
                    codigo = codigo + ` | `

                } else if (ele.nm == "and") {
                    codigo = codigo + ` & `

                } else {

                    codigo = codigo + ` localvalue${multimodos.length}.${ele.nm}`

                }


            } else if (ele.tipo == "val") {

                
                codigo = codigo + ` ${uvaly(ele)} `
                


            } else if (ele.tipo == "operator") {

                if (ele.c == "+" | ele.c == ":" | ele.c == "-" | ele.c == "/" | ele.c == "*" | ele.c == "%" | ele.c == "**"  | ele.c == "++" | ele.c == "--" | ele.c == "|") {
                    if (ele.c == "--") {
                        codigo = codigo + ` -1 `
                        
                    } else if (ele.c == "++") {
                        codigo = codigo + ` +1 `
                        
                    } else if (ele.c == "|") {
                        codigo = codigo + ` || `
                        
                    } else {
                        codigo =  codigo + ` ${ele.c} `
                    }
                } else if (ele.c == "==" | ele.c == "<=" | ele.c == ">=" | ele.c == "!=" | ele.c == "<" | ele.c == ">") {
                    codigo = codigo + ` ${ele.c} `
                } else if (ele.c == "!") {
                    codigo = codigo + `${ele.c} `
                }

            } else if (ele.tipo == "simbols") {
                if (ele.c == ",") {
                    codigo= codigo +`, `
                }
                
            } else if (ele.tipo == "()") {
                

                codigo = codigo + ` (${generator_one(ele.data, variable, "func")}) `; 
            } else if (ele.tipo == "[]") {
                //codigo = codigo + ` lista([${generator_one(ele.data, variable, "func")}]) `; 
                
                if (!ele.is_index) {
                    codigo = codigo + ` std.array([${generator_one(ele.data, variable, "func")}]) `; 
                    
                } else {
                    codigo = codigo + ` [${generator_one(ele.data, variable, "func")}] `; 
                    
                }

            } else if (ele.tipo == "code") {
                codigo = codigo + `{'__classname__': 'Object', ${generator_one(ele.data[0], variable, "func", true)}}`; 
                
            } else if (ele.tipo == "cml") {
                codigo = codigo + `cmlobj(${JSON.stringify(recml(ele.data))})`; 
                
            } else if (ele.tipo == "func-a") {
                    
                let io_p = "";
                
                let varioble = variable.split("_")[0]
                
                for (let l = 0; l < ele.argv.length; l++) {
                    const elemento = ele.argv[l].nm;
                    ele.argv[l] = elemento
                    const tida = ele.sta[l].nm;
                    let valua = ele.defval[l];
                    if ((valua[0]).nm == "null") {valua = "nulo"};
                    
                    io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                    io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                     (
                         (${elemento}!=undefined?${elemento}:${
                            valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                        })
                     ), 
                     sta${multimodos.length+1}); ${N}`;
                }



                codigo= codigo +` funcanom( ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
let este = anon_func[${af++}];
varint.push(localvalue${multimodos.length+1});


${io_p}

${
    generator(ele.code.data, variable, "func")
}
}, localvalue${multimodos.length}.${ele.name}, ${ele.async?"true":"false"})`




            }
        } else if (modo == "class") {

            if (ele.tipo == "name") {
                
                
                if ((ele.nm+"").substr(0, 1) == ".") {

                    codigo = codigo + ` ${ele.nm} ` 
                    
                } else if ((ele.nm+"").substr(0, 3) == "me.") {

                    codigo = codigo + ` me.${(ele.nm+"").substr(3)} ` 
                    
                } else if (ele.nm == "me") {
                    codigo = codigo + ` me ` 
                    

                } else if ((ele.nm+"").substr(0, 8) == "private.") {

                    codigo = codigo + ` private.${(ele.nm+"").substr(8)} ` 
                    
                } else if (ele.nm == "private") {
                    codigo = codigo + ` private ` 
                    

                } else if ((ele.nm+"").substr(0, 7) == "global.") {

                    codigo = codigo + ` glb.${(ele.nm+"").substr(7)} ` 
                    
                } else if (ele.nm == "global") {
                    codigo = codigo + ` glb ` 
                    

                } else if (ele.nm == "new") {
                    codigo = codigo + ` new `

                } else if (ele.nm == "await") {
                    codigo = codigo + ` await `
                    
                } else if (ele.nm == "yield") {
                    codigo = codigo + ` yield `
                    
                } else if (ele.nm == "or") {
                    codigo = codigo + ` | `

                } else if (ele.nm == "and") {
                    codigo = codigo + ` & `

                } else {

                    codigo = codigo + ` localvalue${multimodos.length}.${ele.nm}`

                }


            } else if (ele.tipo == "val") {

                
                codigo = codigo + ` ${uvaly(ele)} `
                


            } else if (ele.tipo == "operator") {

                if (ele.c == "+" | ele.c == ":" | ele.c == "-" | ele.c == "/" | ele.c == "*" | ele.c == "%" | ele.c == "**"  | ele.c == "++" | ele.c == "--" | ele.c == "|") {
                    if (ele.c == "--") {
                        codigo = codigo + ` -1 `
                        
                    } else if (ele.c == "++") {
                        codigo = codigo + ` +1 `
                        
                    } else if (ele.c == "|") {
                        codigo = codigo + ` || `
                        
                    } else {
                        codigo =  codigo + ` ${ele.c} `
                    }
                } else if (ele.c == "==" | ele.c == "<=" | ele.c == ">=" | ele.c == "!=" | ele.c == "<" | ele.c == ">") {
                    codigo = codigo + ` ${ele.c} `
                } else if (ele.c == "!") {
                    codigo = codigo + `${ele.c} `
                }

            } else if (ele.tipo == "simbols") {
                if (ele.c == ",") {
                    codigo= codigo +`, `
                }
                
            } else if (ele.tipo == "()") {
                
                codigo = codigo + ` (${generator_one(ele.data, variable, "class")}) `; 
            } else if (ele.tipo == "[]") {
                //codigo = codigo + ` lista([${generator_one(ele.data, variable, "func")}]) `; 
                if (!ele.is_index) {
                    codigo = codigo + ` std.array([${generator_one(ele.data, variable, "class")}]) `; 
                    
                } else {
                    codigo = codigo + ` [${generator_one(ele.data, variable, "class")}] `; 
                    
                }

            } else if (ele.tipo == "code") {
                codigo = codigo + `{'__classname__': 'Object', ${generator_one(ele.data[0], variable, "class", true)}}`; 
                
            } else if (ele.tipo == "cml") {
                codigo = codigo + `cmlobj(${JSON.stringify(recml(ele.data))})`; 
                
            } else if (ele.tipo == "func-a") {
                    
                let io_p = "";
                
                let varioble = variable.split("_")[0]
                
                for (let l = 0; l < ele.argv.length; l++) {
                    const elemento = ele.argv[l].nm;
                    ele.argv[l] = elemento
                    const tida = ele.sta[l].nm;
                    let valua = ele.defval[l];
                    if ((valua[0]).nm == "null") {valua = "nulo"};
                    
                    io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                    io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                     (
                         (${elemento}!=undefined?${elemento}:${
                            valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                        })
                     ), 
                     sta${multimodos.length+1}); ${N}`;
                }



                codigo= codigo +` funcanom( ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
let este = anon_func[${af++}];
varint.push(localvalue${multimodos.length+1});


${io_p}

${
    generator(ele.code.data, variable, "func")
}
}, localvalue${multimodos.length}.${ele.name}, ${ele.async?"async":""})`




            }
        }
    
    } 
    
    return(
        codigo
    )

};
let varint = [];
let linearmo = [];
let lom = -1;
let tiolom = 0
let multimodos = [];
let f_no_sopport = true;
function defa(d) {
    return(
        ((d||{}).__default__||(()=>{}))
    )
};
function generator(code, variable, modo, count) {
    let outcode = "";
    let fcode= filenames[filenames.length-1]
    let just_class = "";
    
    if (count == undefined) {
        count = true
    }
    if (count) {
        multimodos[multimodos.length] = modo;
    }
    if (modo == "func") {
        //variable = variable.split("_")[0]
    }
    
    let va = (v, s) => {
        return(
            "workspace." + v + "." + s
        )
    }

    let addcode=(c)=>{

        if (true) {
            outcode = outcode+`
            tiolom = lom
            try {
                ${c};
            } catch (e) {
                msgerror(e, decode_cls(linearmo[${lom}]));
            };
                    
                    `    
        }else {
            outcode = outcode+`${c};`
        }
        
    }
    for (let i = 0; i < code.length; i++) {
        const line = code[i];
        let codigo = ""
        let tipao = "";

        linearmo[linearmo.length] = [line, fcode];
        lom= lom +1
        let error_ge = (e) => {
            msgerror(e, decode_cls(linearmo[linearmo.length-1]))

        }
        for (let x = 0; x < line.length; x++) {
            const ele = line[x];
            if (tipao == ele.tipo & tipao != "()" & tipao != "[]") {
                if (line[x-1].nm == "new") {
                    //tipao = ele.tipo;
                        
                } else if (line[x-1].nm == "or") {
                    //tipao = ele.tipo;
                        
                } else if (line[x-1].nm == "and") {
                    //tipao = ele.tipo;
                        
                } else if (ele.nm == "and") {
                    //tipao = ele.tipo;
                    
                } else if (ele.nm == "or") {
                    //tipao = ele.tipo;
                        
                } else {
                    msgerror("Error de sintaxis, uso proporcionados de un tipo de token", decode_cls(linearmo[linearmo.length-1]))
    
                }
                tipao = ele.tipo;
            } else if (ele.tipo == "[]") {
                if (tipao == "name") {
                    
                    ele.is_index = false;
                    
                };
                tipao = ele.tipo;
            } else {
                if (tipao == "()" | tipao == "[]" | tipao == "val") {
                    if (ele.tipo == "name") {
                        if (ele.nm.substr(0,1) == ".") {
                        
                        } else if (ele.nm == "or") {
    
                        } else if (ele.nm == "and") {
    
                        } else {
                            msgerror("Error de sintaxis, uso proporcionados de un tipo de token", decode_cls(line))
                            
                        }
                        
                    } else {

                        if (ele.tipo == "[]") {
                            ele.is_index = true;
                        }
                        //msgerror("Error de sintaxis, uso proporcionados de un tipo de token")
                        
                    }
                } else {
                    
                }
                tipao = ele.tipo;
            };

            if (modo == "normal") {

                if (ele.tipo == "name") {
                    
                    
                    if ((ele.nm+"").substr(0, 1) == ".") {

                        codigo = codigo + ` ${ele.nm} ` 
                        
                    } else if ((ele.nm+"").substr(0, 3) == "me.") {

                        codigo = codigo + ` workspace.${variable}.${(ele.nm+"").substr(3)} ` 
                        
                    } else if (ele.nm == "me") {
                        codigo = codigo + ` workspace.${variable} ` 
                        
    
                    } else if ((ele.nm+"").substr(0, 8) == "private.") {

                        codigo = codigo + ` workspace.${variable}.${(ele.nm+"").substr(8)} ` 
                        
                    } else if (ele.nm == "private") {
                        codigo = codigo + ` workspace.${variable} ` 
                        
    
                    } else if ((ele.nm+"").substr(0, 7) == "global.") {

                        codigo = codigo + ` glb.${(ele.nm+"").substr(7)} ` 
                        
                    } else if (ele.nm == "global") {
                        codigo = codigo + ` glb ` 
                        
    
                    } else if (ele.nm == "or") {
                        codigo = codigo + ` | `

                    } else if (ele.nm == "new") {
                        codigo = codigo + ` new `

                    } else if (ele.nm == "break") {
                        codigo = codigo + ` break `

                    } else if (ele.nm == "continue") {
                        codigo = codigo + ` continue `

                    } else if (ele.nm == "await") {
                        codigo = codigo + ` await `
                        
                    } else if (ele.nm == "yield") {
                        codigo = codigo + ` yield `
                        
                    } else if (ele.nm == "and") {
                        codigo = codigo + ` & `

                    } else {

                        codigo = codigo + ` workspace.${variable}.${ele.nm} `

                    }


                } else if (ele.tipo == "val") {


                    
                    codigo = codigo + ` ${uvaly(ele)} `
                    

                } else if (ele.tipo == "operator") {

                    if (ele.c == "+" | ele.c == "-" | ele.c == "/" | ele.c == "*" | ele.c == "%" | ele.c == "**"  | ele.c == "++" | ele.c == "--" | ele.c == "|") {
                        if (ele.c == "--") {
                            codigo = codigo + ` -1 `
                            
                        } else if (ele.c == "++") {
                            codigo = codigo + ` +1 `
                            
                        } else if (ele.c == "|") {
                            codigo = codigo + ` || `
                            
                        } else {
                            codigo =  codigo + ` ${ele.c} `
                        }
                    } else if (ele.c == "==" | ele.c == "<=" | ele.c == ">=" | ele.c == "!=" | ele.c == "<" | ele.c == ">") {
                        codigo = codigo + ` ${ele.c} `
                    } else if (ele.c == "!") {
                        codigo = codigo + `${ele.c} `
                    }

                } else if (ele.tipo == "simbols") {
                    if (ele.c == ",") {
                        codigo= codigo +`, `
                    }
                    
                } else if (ele.tipo == "func-def") {
                    
                    let io_p = "";
                    
                    let varioble = variable.split("_")[0]
                    
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};
                        
                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }
                    codigo= codigo +`
workspace.${variable}.${ele.name} = ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let este = workspace.${variable}.${ele.name};
    varint.push(localvalue${multimodos.length+1});
    
    ${io_p}
    
    ${
        generator(ele.code.data, variable, "func")
    }
};
workspace.${variable}.${ele.name}.__rt__ = workspace.${variable}.${ele.rt};
workspace.${variable}.${ele.name}.__type__ = "Function";
workspace.${variable}.${ele.name}.__classname__ = "Function";
workspace.${variable}.${ele.name}.__async__ = ${ele.async?"true":"false"}

                    
                    `
                } else if (ele.tipo == "class-def") {
                    
                    
                    
                    
                    let gen = generator(ele.code.data, variable, "class");
                    let gen2 = gen.split("$");
                    //vi = gen2[0].split(",")

                    let io_obj = "";
                    for (let l = 0; l < ele.ext.length; l++) {
                        const elemento = ele.ext[l].nm;
                        ele.ext[l] = elemento;
                        io_obj = io_obj + "localvalue"+(multimodos.length+1)+"." + elemento +",";
                    }


                    codigo = `
workspace.${variable}.${ele.name} = function(${gen2[0]}) {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    let private = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let u = cla.decode([${io_obj}], private, localvalue${multimodos.length+1}, sta${multimodos.length+1});
    let me = u.me;
    let exportar = u.exp;
    let statico = u.sta;
    me.__classname__ = pkg_name + '.${ele.name}';
    let este_c = workspace.${variable}.${ele.name};

    ${
        gen2.slice(1).join("$")
    }

    workspace.${variable}.${ele.name}.__export__ = exportar;
    
    me.__export__=Object.keys(exportar);
    workspace.${variable}.${ele.name} = Object.assign(
        workspace.${variable}.${ele.name}, statico
    );
    return(me);
};
workspace.${variable}.${ele.name}.__instance__ = false;
workspace.${variable}.${ele.name}.__instance__ = cla.encode(workspace.${variable}.${ele.name}());
workspace.${variable}.${ele.name}.__classname__ = pkg_name + '.${ele.name}';
workspace.${variable}.${ele.name}.__type__ = 'Class'
                    
                    `
                    
                    

                } else if (ele.tipo == "module-def") {
                    
                    
                    
                    
                    let gen = generator(ele.code.data, variable, "module");
                    
                    //vi = gen2[0].split(",")

                    


                    codigo = `
workspace.${variable}.${ele.name} = (function() {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    let me = {};
    let private = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    me.__type__ = 'Module';
    let este_c = workspace.${variable}.${ele.name};

    ${
        gen
    }
    return(me);
})();

                    
                    `
                    
                    

                } else if (ele.tipo == "if-def") {

                    let code_generador = ""

                    code_generador = `
                    
if ((${generator_one(ele.onif.cond.data, variable, "normal")})) {

    ${generator(ele.onif.code.data, variable, "normal", false)}

}`

                    for (let ala = 0; ala < ele.elif.length; ala++) {
                        const ili = ele.elif[ala];
                        code_generador = code_generador + `
                    
else if ((${generator_one(ili.cond.data, variable, "normal")})) {

    ${generator(ili.code.data, variable, "normal", false)}

}`                      
                    }

                    code_generador = code_generador + `
                    
else {

    ${generator(ele.outif.code.data, variable, "normal", false)}

}`


                    codigo = codigo + code_generador;
                    
                } else if (ele.tipo == "while-def") {
                    codigo = `
                    
while ((${generator_one(ele.cond.data, variable, "normal")})) {

    ${generator(ele.code.data, variable, "normal", false)}

}`
                } else if (ele.tipo == "for-e-def") {
                    codigo = `
i_l${multimodos.length}=std.array(${generator_one(ele.valor.data, variable, "normal")})
for (let i_i = 0; i_i < i_l${multimodos.length}.length; i_i++) {
    workspace.${variable}.${ele.iterador.nm} = i_l${multimodos.length}[i_i];
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    ${generator(ele.code.data, variable, "normal")}

}`
                } else if (ele.tipo == "for-def") {
                    codigo = `
i_l${multimodos.length}=range_cls(${generator_one(ele.valor.data, variable, "normal")})
for (let i_i = 0; i_i < i_l${multimodos.length}.length; i_i++) {
    workspace.${variable}.${ele.iterador.nm} = i_l${multimodos.length}[i_i];
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    ${generator(ele.code.data, variable, "normal")}

}`
                } else if (ele.tipo == "with-def") {
                    codigo = `
((a) => {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    localvalue${multimodos.length+1}.${ele.name.nm} = a;
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});

    ${generator(ele.code.data, variable, "func")}

})(${generator_one(ele.valor.data, variable, "normal")})`
                } else if (ele.tipo == "import-old.000") {
                    codigo = `
                    
workspace.${variable}.${ele.name} = importar(${ele.src}, "${variable}_${ele.name}", "${variable}", "${ele.name}")
                    
                    `
                } else if (ele.tipo == "import") {
                    let imimp = importar(eval(ele.src), variable)

                    if (imimp[1]) {
                        codigo=  `
workspace.${variable}.${ele.name} = (() => {
    let localvalue${multimodos.length+1} = Object.assign({}, plt.${variable});
    let pkg_name = "${ele.name}"
    let sta${multimodos.length+1} = {};
    ${
        imimp[0]
    }

    return localvalue${multimodos.length+1};
})()
                    
                    `
                    } else {
                        codigo=  `
workspace.${variable}.${ele.name} = ${imimp[0]}
                    
                    `
                    }
                    
                    
                } else if (ele.tipo == "f-import") {
                    let imimp = importar(eval(ele.src), variable)

                    if (imimp[1]) {
                        codigo=  `
workspace.${variable}.${ele.name} = ((() => {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    let pkg_name = "${ele.name}"
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    ${
        imimp[0]
    }

    return localvalue${multimodos.length+1};
})()).${ele.imp}
                    
                    `
                    } else {
                        codigo=  `
workspace.${variable}.${ele.name} = (${imimp[0]}).${ele.imp}
                    
                    `
                    }
                    
                    
                } else if (ele.tipo == "include") {

                    let src = eval(ele.src)

                    if (cls_execute =="native") {
                        
                        msgerror("include aun no esta soportado para cls en esta version", decode_cls(line));

                    } else {
                        msgerror("include aun no esta soportado para cls en esta version", decode_cls(line));
                        
                    }
                    
                } else if (ele.tipo == "try") {
                    codigo = `
                    
try {
    trys = trys+1
    ${generator(ele.code_try.data, variable, "normal")}
    trys = trys-1
} catch (e_e) {
    workspace.${variable}.${ele.vare} = std.str(e_e)
    ${generator(ele.code_error.data, variable, "normal")}

}`
                } else if (ele.tipo == "var-def") {
                    
                    if ((ele.name+"").substr(0,3) == "me.") {
                        codigo = `
sta${multimodos.length}['${ele.name.substr(3)}'] = sta${multimodos.length}['${ele.name.substr(3)}'] || "Any";
workspace.${variable}.${ele.name.substr(3)} = dim("${ele.name.substr(3)}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})
                        
                                            
                                            ` 
                    } else if ((ele.name+"").substr(0,8) == "private.") {
                        codigo = `
sta${multimodos.length}['${ele.name.substr(8)}'] = sta${multimodos.length}['${ele.name.substr(8)}'] || "Any";
workspace.${variable}.${ele.name.substr(8)} = dim("${ele.name.substr(8)}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})
                        
                                            
                                            ` 
                    } else if ((ele.name+"").substr(0,7) == "global.") {
                        codigo = `
sta1['${ele.name}'] = sta1['${ele.name}'] || "Any";
glb.${ele.name.substr(7)} = dim("${ele.name}", ${generator_one(ele.eval, variable, "normal")}, sta1)
                        
                                            
                                            ` 
                    } else {

                        codigo = `
sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
workspace.${variable}.${ele.name} = dim("${ele.name}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})

                    
                    `

                    }
                    
                } else if (ele.tipo == "global-def") {
                    
                    codigo = `
sta1['${ele.name}'] = sta1['${ele.name}'] || "Any";
sta${multimodos.length}['${ele.name}'] = sta1['${ele.name}'];
u_u = dim("${ele.name}", ${generator_one(ele.eval, variable, "normal")}, sta1);
workspace.${variable}.${ele.name} = u_u;

                    
                    `
                } else if (ele.tipo == "var_extend-def") {
                    
                    
                    if ((ele.subname[0].nm+"").substr(0,3) == "me.") {
                        codigo = `



workspace.${variable}.${generator_one(ele.subname, variable, "normal").substr(` workspace.${variable}.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "normal")}), "Any")
                        
                                            
                                            ` 
                    } else if ((ele.subname[0].nm+"").substr(0,7) == "global.") {
                        codigo = `



glb.${generator_one(ele.subname, variable, "normal").substr(` glb.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "normal")}), "Any")
                        
                                            
                                            ` 
                    } else if ((ele.subname[0].nm+"").substr(0,8) == "private.") {
                        codigo = `



workspace.${variable}.${generator_one(ele.subname, variable, "normal").substr(` workspace.${variable}.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "normal")}), "Any")
                        
                                            
                                            ` 
                    } else {

                        codigo = `




workspace.${variable}.${generator_one(ele.subname, variable, "normal").substr(` workspace.${variable}.`.length)} = dim("subname", ${generator_one(ele.eval, variable, "normal")}, "Any")

                    
                    `

                    }
                    
                } else if (ele.tipo == "global_extend-def") {
                    codigo = `



oo_t = dim("subname", ${generator_one(ele.eval, variable, "normal")}, "Any");
workspace.${variable}.${generator_one(ele.subname, variable, "normal").substr(` workspace.${variable}.`.length)} = oo_t
                    
                                        
                                        `
                } else if (ele.tipo == "dim-def") {
                    
                        let codi = `

setsta(sta${multimodos.length}, workspace.${variable}.${ele.dim}, '${ele.name}') 
                                            
                                            `;
                        if(ele.eval != false){
                            if ((ele.name+"").substr(0,3) == "me.") {
                                codigo = `
        sta${multimodos.length}['${ele.name.substr(3)}'] = sta${multimodos.length}['${ele.name.substr(3)}'] || "Any";
        workspace.${variable}.${ele.name.substr(3)} = dim("${ele.name.substr(3)}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})
                                
                                                    
                                                    ` 
                            } else if ((ele.name+"").substr(0,8) == "private.") {
                                codigo = `
        sta${multimodos.length}['${ele.name.substr(8)}'] = sta${multimodos.length}['${ele.name.substr(8)}'] || "Any";
        workspace.${variable}.${ele.name.substr(8)} = dim("${ele.name.substr(8)}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})
                                
                                                    
                                                    ` 
                            } else if ((ele.name+"").substr(0,7) == "global.") {
                                codigo = `
        sta1['${ele.name}'] = sta1['${ele.name}'] || "Any";
        glb.${ele.name.substr(7)} = dim("${ele.name}", ${generator_one(ele.eval, variable, "normal")}, sta1)
                                
                                                    
                                                    ` 
                            } else {
        
                                codigo = `
        sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
        workspace.${variable}.${ele.name} = dim("${ele.name}", ${generator_one(ele.eval, variable, "normal")}, sta${multimodos.length})
        
                            
                            `
        
                            }
                        } else {
                            codigo = `workspace.${variable}.${ele.name} = ((workspace.${variable}.${ele.dim}||{}).__default__||(()=>{}))()`
                        };

                        codigo = codi + ";" + codigo;
                    
                    
                } else if (ele.tipo == "flecha-def") {
                    
                    codigo = "";

                    for (let i = 0; i < ele.name.length; i++) {
                        const e = ele.name[i];
                        codigo = codigo + `

setsta(sta${multimodos.length}, workspace.${variable}.${ele.dim}, '${e.nm}') 
                                            
                                            `;
codigo = codigo+ `;workspace.${variable}.${e.nm} = ((workspace.${variable}.${ele.dim}||{}).__default__||(()=>{}))()`

                        
                    }
                        
                    
                    
                } else if (ele.tipo == "let-def.old") {
                    
                    codigo = `
workspace.${variable}.${ele.name} = new ${generator_one(ele.eval, variable, "normal")}
//workspace.${variable}.${ele.name}.create${generator_one(ele.eval.slice(1), variable, "normal")}

                    
                    `
                } else if (ele.tipo == "()") {
                    
    
                    codigo = codigo + ` (${generator_one(ele.data, variable, "normal")}) `; 
                        
                    
                } else if (ele.tipo == "[]") {
                    if (ele.is_index) {
                        codigo = codigo + ` std.array([${generator_one(ele.data, variable, "normal")}]) `; 
                        
                    } else {
                        
                        codigo = codigo + ` [${generator_one(ele.data, variable, "normal")}] `; 
                    }
                    //codigo = codigo + ` lista([${generator_one(ele.data, variable, "normal")}]) `; 

                } else if (ele.tipo == "return") {
                    codigo = `
                    
return dim("var out", (${generator_one(ele.eval, variable, "normal")}), "Any")

`
                } else if (ele.tipo == "func-a") {
                    
                    let io_p = "";
                    
                    let varioble = variable.split("_")[0]
                    
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};
                        
                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }



                    codigo= codigo +` funcanom( ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
    let localvalue${multimodos.length+1} = Object.assign({}, workspace.${variable});
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let este = anon_func[${af++}]
    varint.push(localvalue${multimodos.length+1})
    
    
    ${io_p}
    
    ${
        generator(ele.code.data, variable, "func")
    }
}, workspace.${variable}.${ele.name}, ${ele.async?"true":"false"})`




                } else if (ele.tipo == "all-var-def") {
                    let io_p =""
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};
                        
                        io_p = io_p + ` setsta(sta${multimodos.length}, workspace.${variable}.${tida}, '${elemento}'); `;
                        io_p = io_p + "workspace."+(variable)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${
                                valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(workspace.${variable}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length}); ${N}`;
                    };
                    codigo = codigo + io_p+";"+N;
                }
                //estas por aqui
                
            } else if ((modo == "func")|(modo == "func-imp")) {

                if (ele.tipo == "name") {
                    
                    if ((ele.nm+"").substr(0, 1) == ".") {

                        codigo = codigo + ` ${ele.nm} ` 
                        
                    } else if ((ele.nm+"").substr(0, 3) == "me.") {

                        codigo = codigo + ` this.${(ele.nm+"").substr(3)} ` 
                        
                    } else if (ele.nm == "me") {
                        codigo = codigo + ` this ` 
                        

                    } else if ((ele.nm+"").substr(0, 8) == "private.") {

                        codigo = codigo + ` private.${(ele.nm+"").substr(8)} ` 
                        
                    } else if (ele.nm == "private") {
                        codigo = codigo + ` private ` 
                        
    
                    } else if ((ele.nm+"").substr(0, 7) == "global.") {
    
                        codigo = codigo + ` glb.${(ele.nm+"").substr(7)} ` 
                        
                    } else if (ele.nm == "global") {
                        codigo = codigo + ` glb ` 
                        
    
                    } else if (ele.nm == "new") {
                        codigo = codigo + ` new `

                    } else if (ele.nm == "break") {
                        codigo = codigo + ` break `

                    } else if (ele.nm == "await") {
                        codigo = codigo + ` await `
                        
                    } else if (ele.nm == "yield") {
                        codigo = codigo + ` yield `
                        
                    } else if (ele.nm == "continue") {
                        codigo = codigo + ` continue `

                    } else if (ele.nm == "or") {
                        codigo = codigo + ` | `

                    } else if (ele.nm == "and") {
                        codigo = codigo + ` & `

                    } else {

                        codigo = codigo + ` localvalue${multimodos.length}.${ele.nm}`

                    }


                } else if (ele.tipo == "val") {


                    
                    codigo = codigo + ` ${uvaly(ele)} `
                    


                } else if (ele.tipo == "operator") {

                    if (ele.c == "+" | ele.c == "-" | ele.c == "/" | ele.c == "*" | ele.c == "%" | ele.c == "**"  | ele.c == "++" | ele.c == "--" | ele.c == "|") {
                        if (ele.c == "--") {
                            codigo = codigo + ` -1 `
                            
                        } else if (ele.c == "++") {
                            codigo = codigo + ` +1 `
                            
                        } else if (ele.c == "|") {
                            codigo = codigo + ` || `
                            
                        } else {
                            codigo =  codigo + ` ${ele.c} `
                        }
                    } else if (ele.c == "==" | ele.c == "<=" | ele.c == ">=" | ele.c == "!=" | ele.c == "<" | ele.c == ">") {
                        codigo = codigo + ` ${ele.c} `
                    } else if (ele.c == "!") {
                        codigo = codigo + `${ele.c} `
                    }

                } else if (ele.tipo == "simbols") {
                    if (ele.c == ",") {
                        codigo= codigo +`, `
                    }
                    
                } else if (ele.tipo == "func-def") {
                    
                    if (f_no_sopport) {
                        if (modo == "func") {
                            error_ge("no puedes crear funciones dentro de una funcion");
                        }
                    }
                    
                    varioble = variable.split("_")[0]
                    
                    let io_p = "";
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};

                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        //io_p = io_p + `localvalue${multimodos.length+1}.`+ elemento + " = " + `dim('${elemento}', (${elemento}!=undefined?${elemento}:${generator_one(valua, variable,"func")}), sta${multimodos.length+1}); ${N}`;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"func"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }
                    
                    codigo= codigo +`
localvalue${multimodos.length}.${ele.name} = ${ele.async?"async":""} function(${ele.argv.join(", ")}) {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let este = localvalue${multimodos.length}.${ele.name};
    varint.push(localvalue${multimodos.length+1});

    ${io_p}
    
    ${
        generator(ele.code.data, variable, "func")
    }
};
localvalue${multimodos.length}.${ele.name}.__rt__ = localvalue${multimodos.length}.${ele.rt};
localvalue${multimodos.length}.${ele.name}.__type__ = 'Function';
localvalue${multimodos.length}.${ele.name}.__classname__ = 'Function';
localvalue${multimodos.length}.${ele.name}.__async__ = ${ele.async?"true":"false"}
                    
                    `
                } else if (ele.tipo == "class-def") {
                    
                    //se a retirado las clases por crear inestabilidad en la creacion de codigo
                    if (f_no_sopport) {
                        if (modo == "func") {
                            error_ge("no puedes crear clases dentro de una funcion");
                        }
                    }

                    let gen = generator(ele.code.data, variable, "class");
                    let gen2 = gen.split("$");
                    //vi = gen2[0].split(",") workspace.${variable}.

                    let io_obj = "";
                    for (let l = 0; l < ele.ext.length; l++) {
                        const elemento = ele.ext[l].nm;
                        ele.ext[l] = elemento;
                        io_obj = io_obj + "localvalue"+(multimodos.length+1)+"." + elemento +",";
                    }


                    codigo = `

localvalue${multimodos.length}.${ele.name} = function(${gen2[0]}) {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let u = cla.decode([${io_obj}]);
    let private = {};
    let me = u.me;
    let exportar = u.exp;
    let statico = u.sta;
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    me.__classname__ = pkg_name + '.${ele.name}'
    let este_c = localvalue${multimodos.length}.${ele.name};
    

    ${
        gen2.slice(1).join("$")
    }

    localvalue${multimodos.length}.${ele.name}.__export__ = exportar'

    me.__export__ = Object.keys(exportar);
    localvalue${multimodos.length}.${ele.name} = Object.assign(
        localvalue${multimodos.length}.${ele.name}, estatico
        );
    return(me);
};
localvalue${multimodos.length}.${ele.name}.__instance__ = false
localvalue${multimodos.length}.${ele.name}.__instance__ = cla.encode(localvalue${multimodos.length}.${ele.name}());
localvalue${multimodos.length}.${ele.name}.__classname__ = pkg_name + '.${ele.name}'
                    `

                } else if (ele.tipo == "module-def") {
                    
                    if (f_no_sopport) {
                        if (modo == "func") {
                            error_ge("no puedes crear modulos dentro de una funcion");
                        }
                    }
                    

                    
                    
                    let gen = generator(ele.code.data, variable, "module");
                    
                    //vi = gen2[0].split(",")

                    


                    codigo = `
localvalue${multimodos.length}.${ele.name} = (function() {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let me = {};
    let private = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    me.__classname__ = 'Module';
    let este_c = localvalue${multimodos.length}.${ele.name};

    ${
        gen
    }
    return(me);
})();
localvalue${multimodos.length}.${ele.name}.__instance__ = true;
//localvalue${multimodos.length}.${ele.name}.__instance__ = cla.encode(workspace.${variable}.${ele.name}());
//localvalue${multimodos.length}.${ele.name}.__classname__ = pkg_name + '.${ele.name}';
localvalue${multimodos.length}.${ele.name}.__classname__ = 'Module';
                    
                    `
                    
                    

                } else if (ele.tipo == "if-def") {

                    let code_generador = ""

                    code_generador = `
                    
if (${generator_one(ele.onif.cond.data, variable, "func")}) {

    ${generator(ele.onif.code.data, variable, "func", false)}

}`

                    for (let ala = 0; ala < ele.elif.length; ala++) {
                        const ili = ele.elif[ala];
                        code_generador = code_generador + `
                    
else if (${generator_one(ili.cond.data, variable, "func")}) {

    ${generator(ili.code.data, variable, "func", false)}

}`                      
                    }

                    code_generador = code_generador + `
                    
else {
    
    ${generator(ele.outif.code.data, variable, "func", false)}

}`


                    codigo = code_generador;
                    
                } else if (ele.tipo == "while-def") {
                    codigo = `
                    
while (${generator_one(ele.cond.data, variable, "func")}) {

    ${generator(ele.code.data, variable, "func", false)}

}`
                } else if (ele.tipo == "for-e-def") {
                    codigo = `
i_l${multimodos.length} = std.array(${generator_one(ele.valor.data, variable, "func")});
for (let i_i = 0; i_i < i_l${multimodos.length}.length; i_i++) {
    let localvalue${multimodos.length+1} = localvalue${multimodos.length};
    localvalue${multimodos.length+1}.${ele.iterador.nm} = i_l${multimodos.length}[i_i];
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    ${generator(ele.code.data, variable, "func")}

};
localvalue${multimodos.length}.${ele.iterador.nm} = undefined

`
                } else if (ele.tipo == "for-def") {
                    codigo = `
i_l${multimodos.length} = range_cls(${generator_one(ele.valor.data, variable, "func")});
for (let i_i = 0; i_i < i_l${multimodos.length}.length; i_i++) {
    let localvalue${multimodos.length+1} = localvalue${multimodos.length};
    localvalue${multimodos.length+1}.${ele.iterador.nm} = i_l${multimodos.length}[i_i];
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    ${generator(ele.code.data, variable, "func")}

};
localvalue${multimodos.length}.${ele.iterador.nm} = undefined

`
                } else if (ele.tipo == "with-def") {
                    codigo = `
((a) => {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    localvalue${multimodos.length+1}.${ele.name.nm} = a;
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});

    ${generator(ele.code.data, variable, "func")}

})(${generator_one(ele.valor.data, variable, "func")})
`
                } else if (ele.tipo == "import-old.000") {
                    codigo = `
                    
localvalue.${ele.name} = importar(${ele.src}, "${variable}_${ele.name}", "${variable}", "${ele.name}")
                    
                    `
                } else if (ele.tipo == "import") {
                    let ommm = importar(eval(ele.src), variable);

                    if (ommm[1]) {
                        codigo=  `
localvalue${multimodos.length}.${ele.name} = (() => {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let pkg_name = "${ele.name}"
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    
    ${
        importar(eval(ele.src), variable)
    }

    return localvalue${multimodos.length+1};
})()
                    
                    `
                    }else {
                        codigo=  `
localvalue${multimodos.length}.${ele.name} = ${ommm[0]}
                    
                    `

                    }
                    
                    
                } else if (ele.tipo == "include") {

                    let src = eval(ele.src)

                    if (cls_execute =="native") {
                        
                        msgerror("include aun no esta soportado para cls en esta version");

                    } else {
                        msgerror("include aun no esta soportado para cls en esta version");
                        
                    }
                    
                } else if (ele.tipo == "try") {
                    codigo = `
                    
try {
    trys = trys+1
    ${generator(ele.code_try.data, variable, "func", false)}
    trys = trys-1
} catch (e_e) {
    localvalue${multimodos.length}.${ele.vare} = std.str(e_e)
    ${generator(ele.code_error.data, variable, "func", false)}

}`
                } else if (ele.tipo == "var-def") {
                    
                    if ((ele.name+"").substr(0,3) == "me.") {
                        codigo = `

sta${multimodos.length-1}['${(ele.name+"").substr(3)}'] = sta${multimodos.length-1}['${(ele.name+"").substr(3)}'] || "Any";
me.${(ele.name+"").substr(3)} = dim("${(ele.name+"").substr(3)}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length-1})
                        
                                            
                                            ` 
                    } else if ((ele.name+"").substr(0,8) == "private.") {
                        codigo = `

sta${multimodos.length-1}['${(ele.name+"")}'] = sta${multimodos.length-1}['${(ele.name+"")}'] || "Any";
private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length-1})
                        
                                            
                                            ` 
                    } else if ((ele.name+"").substr(0,7) == "global.") {
                        codigo = `

sta1['${(ele.name+"")}'] = sta1['${(ele.name+"")}'] || "Any";

glb.${(ele.name+"").substr(7)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "func")}, sta1)
                        
                                            
                                            ` 
                    } else {

                        codigo = `
sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
localvalue${multimodos.length}.${(ele.name)} = dim("${ele.name}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length})

                    
                    `

                    }
                } else if (ele.tipo == "var_extend-def") {
                    
                    
                    if ((ele.subname[0].nm+"").substr(0,3) == "me.") {
                        codigo = `



me.${generator_one(ele.subname, variable, "func").substr(` me.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "func")}), "Any")
                        
                                            
                                            ` 
                    } else if ((ele.subname[0].nm+"").substr(0,7) == "global.") {
                        
                        codigo = `



glb.${generator_one(ele.subname, variable, "func").substr(` glb.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "func")}), "Any")
                        
                                            
                                            ` 
                    } else if ((ele.subname[0].nm+"").substr(0,8) == "private.") {
                        
                        codigo = `



private.${generator_one(ele.subname, variable, "func").substr(` private.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "func")}), "Any")
                        
                                            
                                            ` 
                    } else {

                        codigo = `




localvalue${multimodos.length}.${generator_one(ele.subname, variable, "func").substr(` localvalue${multimodos.length}.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "func")}), "Any")
                    
                    `

                    }
                    
                } else if (ele.tipo == "dim-def") {
                    
                    let codi=""
                    if ((ele.name+"").substr(0,7) == "global.") {
                        codi = `

setsta(sta1, localvalue${multimodos.length}.${ele.dim}, '${ele.name}') 
                                            
                                            ` 
                    } else {

                        codi = `

setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${ele.name}') 
                                            
                                            ` 
                    }
                   
                    if(ele.eval != false){
                        if ((ele.name+"").substr(0,3) == "me.") {
                            codigo = `
    
    sta${multimodos.length-1}['${(ele.name+"").substr(3)}'] = sta${multimodos.length-1}['${(ele.name+"").substr(3)}'] || "Any";
    
    me.${(ele.name+"").substr(3)} = dim("${(ele.name+"").substr(3)}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length-1})
                            
                                                
                                                ` 
                        } else if ((ele.name+"").substr(0,8) == "private.") {
                            codigo = `
    
    sta${multimodos.length-1}['${(ele.name+"")}'] = sta${multimodos.length-1}['${(ele.name+"")}'] || "Any";
    
    private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length-1})
                            
                                                
                                                ` 
                        } else if ((ele.name+"").substr(0,7) == "global.") {
                            codigo = `
    
    sta1['${(ele.name+"")}'] = sta1['${(ele.name+"")}'] || "Any";
    
    glb.${(ele.name+"").substr(7)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "func")}, sta1)
                            
                                                
                                                ` 
                        } else {
    
                            codigo = `
    sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
    localvalue${multimodos.length}.${(ele.name)} = dim("${ele.name}", ${generator_one(ele.eval, variable, "func")}, sta${multimodos.length})
    
                        
                        `
    
                        }
                    } else {
                        codigo = `localvalue${multimodos.length}.${ele.name} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||(()=>{}))()`
                    };

                    codigo = codi + ";" + codigo;
                    
                    
                } else if (ele.tipo == "flecha-def") {
                    
                    codigo = "";

                    for (let i = 0; i < ele.name.length; i++) {
                        const e = ele.name[i];
                        codigo = codigo + `

setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${e.nm}') 
                                            
                                            `;
codigo = codigo+ `;localvalue${multimodos.length}.${e.nm} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||(()=>{}))()`
                
                    }
                        
                    
                    
                } else if (ele.tipo == "global-def") {
                    
                    codigo = `
sta1['${ele.name}'] = sta1['${ele.name}'] || "Any";
sta${multimodos.length}['${ele.name}'] = sta1['${ele.name}'];
u_u = dim("${ele.name}", ${generator_one(ele.eval, variable, "func")}, sta1);
${glparse(multimodos.length, "u_u", ele.name)}
workspace.${variable}.${ele.name} = u_u;

                    
                    `
                } else if (ele.tipo == "global_extend-def") {
                    codigo = `



oo_t = dim("subname", ${generator_one(ele.eval, variable, "normal")}, "Any");
localvalue${multimodos.length}.${generator_one(ele.subname, variable, "func").substr(` localvalue${multimodos.length}.`.length)} = oo_t;
workspace.${variable}.${generator_one(ele.subname, variable, "func").substr(` localvalue${multimodos.length}.`.length)} = oo_t
                    
                                        
                                        `
                } else if (ele.tipo == "let-def.old") {
                    
                    codigo = `
localvalue${multimodos.length}.${ele.name} = new ${generator_one(ele.eval, variable, "func")}
//localvalue.${ele.name}.create${generator_one(ele.eval.slice(1), variable, "func")}

                    
                    `
                } else if (ele.tipo == "()") {
                    
                    codigo = codigo + ` (${generator_one(ele.data, variable, "func")}) `; 
                } else if (ele.tipo == "[]") {
                    //codigo = codigo + ` lista([${generator_one(ele.data, variable, "func")}]) `; 
                    if (ele.is_index) {
                        codigo = codigo + ` std.array([${generator_one(ele.data, variable, "func")}]) `; 
                        
                    } else {
                        
                        codigo = codigo + ` [${generator_one(ele.data, variable, "func")}] `; 
                    }

                } else if (ele.tipo == "return") {
                    codigo = `
                    let salida_final = (${generator_one(ele.eval, variable, "func")});
                    varint.pop()
return dim("var out", (salida_final), este.__rt__.__classname__)

`
                } else if (ele.tipo == "func-a") {
                    
                    let io_p = "";
                    
                    let varioble = variable.split("_")[0]
                    
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};
                        
                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }



                    codigo= codigo +` funcanom( ${ele.async?"async":""} (${ele.argv.join(", ")}) => {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let este = anon_func[${af++}];
    varint.push(localvalue${multimodos.length+1});

    
    
    ${io_p}
    
    ${
        generator(ele.code.data, variable, "func")
    }
}, localvalue${multimodos.length}.${ele.name}, ${ele.async?"true":"false"})`




                } else if (ele.tipo == "all-var-def") {
                    let io_p=""
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};
                        
                        io_p = io_p + `setsta(sta${multimodos.length}, localvalue${multimodos.length}.${tida}, '${elemento}'); `;
                        io_p = io_p + "localvalue"+(multimodos.length)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${
                                valua!="nulo"?generator_one(valua, variable,"normal"):(`defa(localvalue${multimodos.length}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length}); ${N}`;
                    };
                    codigo = codigo + io_p+";"+N;
                }

                
            } else if (modo == "class") {
                
                if (ele.tipo == "func-def") {
                    
                    varioble = variable.split("_")[0]
                    let io_p = "";
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};

                        //(${elemento}!=undefined?${elemento}:${generator_one(valua, variable,"normal")})
                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        //io_p = io_p + "localvalue"+(multimodos.length+1)+"." + elemento + " = "  + `dim('${elemento}', (${elemento}!=undefined?${elemento}:${generator_one(valua, variable,"class")}), sta${multimodos.length+1}); ${N}`;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"class"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }
                    let mo = `workspace.${variable}`;
                    if (multimodos[multimodos.length-2] === "func") {
                        mo = "localvalue" + (multimodos.length);
                    };
                    mo = "localvalue" + (multimodos.length);

                    if (ele.name == "main") {
                        just_class = ele.argv.join(",")
                        codigo= codigo +`
                        if (este_c.__instance__ != false) {
    (function(${ele.argv.join(",")}){
        let localvalue${multimodos.length+1} = Object.assign({}, ${mo});
        let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
        let este = me;
        varint.push(localvalue${multimodos.length+1});

        
        ${io_p}
        
        ${
            generator(ele.code.data, variable, "func")
        }
    })(${ele.argv.join(",")})} 
                        
                        `
                    } else if (ele.name == "default") {
                        
                        codigo= codigo +`
                        este_c.__default__ = function(extra) {
            let k =(function(${ele.argv.join(",")}){
                let localvalue${multimodos.length+1} = Object.assign({}, ${mo});
                let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
                let este = k;
                varint.push(localvalue${multimodos.length+1});
                
                ${io_p}
                
                ${
                    generator(ele.code.data, variable, "func")
                }
            });
            k.__classname__ = "Function";
            k.__rt__ = {__classname__:"Any"};
            k.__type__ = "Function";
            return(k(este_c, extra))
    }
                        `
                    } else {
                        let seu= "me";
                        let post = ``;
                        if (ele.visible == "private") {
                            seu ="private"
                        }
                        else if (ele.visible == "static") {
                            seu ="statico"
                            post = `statico.${ele.name} = ${seu}.${ele.name};`;
                        }
                        else if (ele.visible == "export") {
                            seu ="exportar"
                            post = `exportar.${ele.name} = ${seu}.${ele.name};`;
                        } else {
                            
                        }

                        codigo= codigo +`
    ${seu}.${ele.name} = ${ele.async?"async":""} function(${ele.argv.join(",")}){
        /**/let localvalue${multimodos.length+1} = Object.assign({}, ${mo});/**//*localvalue${multimodos.length+1}*//**/
        /**/let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});/**//*sta${multimodos.length+1}*//**//*sta${multimodos.length}*//**/
        let este = me.${ele.name};
        varint.push(localvalue${multimodos.length+1})

        ${io_p}
        
        ${
            generator(ele.code.data, variable, "func")
        }
    };
    ${seu}.${ele.name}.__rt__ = ${mo}.${ele.rt};
    ${seu}.${ele.name}.__type__ = 'Function';
    ${seu}.${ele.name}.__classname__ = 'Function';
    ${seu}.${ele.name}.__classname__ = 'Function';
    ${seu}.${ele.name}.__async__ = ${ele.async?"true":"false"};
    ${mo}.${ele.name} = ${seu}.${ele.name};
    

    //me.${ele.name}.__rt__ = 'Function';
                        
                        `

                    }
                } else if (ele.tipo == "var-def") {
                    
                    if ((ele.name+"").substr(0,8) == "private.") {
                        
                        codigo = `

sta${multimodos.length}['${(ele.name+"")}'] = sta${multimodos.length}['${(ele.name+"")}'] || "Any";
private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                        
                                            
                                            `  
                    } else {
                        let seu= "me"
                        let post=``;
                        if (ele.visible ==="private") {
                            seu="private"
                        } else if (ele.visible ==="export") {
                            post = `exportar.${ele.name} = ${seu}.${ele.name};`
                        } else if (ele.visible ==="static") {
                            post = `statico.${ele.name} = ${seu}.${ele.name};`;
                        }

                        codigo = `
sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
localvalue${multimodos.length}.${(ele.name+"")} = dim("${ele.name}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length});
${seu}.${(ele.name+"")} = localvalue${multimodos.length}.${(ele.name+"")};
${post}
                           
                                            ` 
                    
                    }
                
                
                } else if (ele.tipo == "export-def") {
                    
                    if(ele.eval.length==0) ele.eval = [name("null")];
                    
                        codigo = `
sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
localvalue${multimodos.length}.${(ele.name+"")} = dim("${ele.name}", (
    ${generator_one(ele.eval, variable, "class")}||me.${(ele.name+"")}
    ), sta${multimodos.length});
me.${(ele.name+"")} = localvalue${multimodos.length}.${(ele.name+"")};
exportar.${(ele.name+"")} = true;                       
                                            
                                            ` 
                    
                    
                
                
                } else if (ele.tipo == "var_extend-def") {
                    
                    if ((ele.subname[0].nm+"").substr(0,8) == "private.") {
                        
                        codigo = `



private.${generator_one(ele.subname, variable, "class").substr(` private.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "class")}), "Any")
                        
                                            
                                            ` 
                    } else {
                    
                    codigo = `



me.${generator_one(ele.subname, variable, "class").substr(` localvalue${multimodos.length}.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "class")}), "Any")
                        
                                            
                                            ` 
                    };
                    
                } else if (ele.tipo == "dim-def") {
                    
                    
                        let codi = `

setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${ele.name}') 
                                            
                                            `;
                        if(ele.eval != false) {
                            if ((ele.name+"").substr(0,8) == "private.") {
                        
                                codigo = `
        
        sta${multimodos.length}['${(ele.name+"")}'] = sta${multimodos.length}['${(ele.name+"")}'] || "Any";
        private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                                
                                                    
                                                    `  
                            } else {
                            
                                codigo = `
        sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
        me.${(ele.name+"")} = dim("${ele.name}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                                
                                                    
                                                    ` 
                            
                            }
                        } else {
                            codigo = `localvalue${multimodos.length}.${ele.name} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||(()=>{}))()`
                        };

                        codigo = codi + ";" +codigo;
                    
                    
                } else if (ele.tipo == "flecha-def") {
                    
                    codigo = "";

                    for (let i = 0; i < ele.name.length; i++) {
                        const e = ele.name[i];
                        codigo = codigo + `

setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${e.nm}') 
                                            
                                            `;
        codigo = codigo+ `;localvalue${multimodos.length}.${e.nm} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||(()=>{}))()`
                        
                        
                    }
                        
                    
                    
                } else if (ele.tipo == "class-def") {
                    if (f_no_sopport) {
                        error_ge("no puedes crear clases dentro de una clase");
                    }
                    let gen = generator(ele.code.data, variable, "class");
                    let gen2 = gen.split("$");
                    //vi = gen2[0].split(",")

                    let io_obj = "";
                    for (let l = 0; l < ele.ext.length; l++) {
                        const elemento = ele.ext[l].nm;
                        ele.ext[l] = elemento;
                        io_obj = io_obj + "localvalue"+(multimodos.length+1)+"." + elemento +",";
                    }

                    


                    codigo = `
let iii${i} = me;
me.${ele.name} = function(${gen2[0]}) {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let este_c = iii${i}.${ele.name};
    let private = {};
    let exportar = {};
    let me = cla.decode([${io_obj}]);
    let statico= {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    
    me.__classname__ = pkg_name + '.${ele.name}';

    ${
        gen2.slice(1).join("$")
    }

    me.${ele.name}.__export__ = exportar;

    me.__export__ = Object.keys(exportar);
    me.${ele.name} = Object.assign(me.${ele.name}, statico);
    return(me);
};
delete iii${i};
me.${ele.name}.__instance__ = false;
me.${ele.name}.__instance__ = cla.encode(me.${ele.name}());
me.__classname__ = pkg_name + '.${ele.name}'
                    
                    `

                } else if (ele.tipo == "module-def") {
                    
                    
                    if (f_no_sopport) {
                        error_ge("no puedes crear modulos dentro de una clase");
                    }
                    
                    
                    let gen = generator(ele.code.data, variable, "module");
                    
                    //vi = gen2[0].split(",")

                    


                    codigo = `
let iii${i} = me;
me.${ele.name} = (function() {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let este_c = iii${i}.${ele.name};
    let me = {};
    let private = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    me.__classname__ = "Module";
    me.__proto__.type = "Module";

    ${
        gen
    }
    return(me);
})();
detele iii${i};
                    
                    `
                    
                    

                } 
                
            } else if (modo == "module") {
                
                if (ele.tipo == "func-def") {
                    
                    varioble = variable.split("_")[0]
                    let io_p = "";
                    for (let l = 0; l < ele.argv.length; l++) {
                        const elemento = ele.argv[l].nm;
                        ele.argv[l] = elemento
                        const tida = ele.sta[l].nm;
                        let valua = ele.defval[l];
                        if ((valua[0]).nm == "null") {valua = "nulo"};

                        io_p = io_p + `setsta(sta${multimodos.length+1}, localvalue${multimodos.length+1}.${tida}, '${elemento}'); `;
                        //io_p = io_p + "localvalue"+(multimodos.length+1)+"." + elemento + " = "  + `dim('${elemento}', (${elemento}!=undefined?${elemento}:${generator_one(valua, variable,"class")}), sta${multimodos.length+1}); ${N}`;
                        io_p = io_p + "localvalue"+(multimodos.length+1)+"."+ elemento + " = "  + `dim('${elemento}',
                         (
                             (${elemento}!=undefined?${elemento}:${
                                valua!="nulo"?generator_one(valua, variable,"class"):(`defa(localvalue${multimodos.length+1}.${tida})()`)
                            })
                         ), 
                         sta${multimodos.length+1}); ${N}`;
                    }
                    let mo = `workspace.${variable}`;
                    if (multimodos[multimodos.length-2] == "func") {
                        mo = "localvalue" + (multimodos.length);
                    };
                    mo = "localvalue" + (multimodos.length);

                    if (true) {

                        codigo= codigo +`
    me.${ele.name} = ${ele.async?"async":""} function(${ele.argv.join(",")}){
        let localvalue${multimodos.length+1} = Object.assign({}, ${mo});
        let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
        let este = me.${ele.name};
        varint.push(localvalue${multimodos.length+1});

        ${io_p}
        
        ${
            generator(ele.code.data, variable, "func")
        }
    };
    me.${ele.name}.__rt__ = ${mo}.${ele.rt};
    me.${ele.name}.__type__ = "Function";
    me.${ele.name}.__classname__ = "Function";
    me.${ele.name}.__classname__ = ${ele.async?"true":"false"};
                        
                        `

                    }
                } else if (ele.tipo == "var-def") {
                    
                    
                    if ((ele.name+"").substr(0,8) == "private.") {
                        
                        codigo = `

sta${multimodos.length}['${(ele.name+"")}'] = sta${multimodos.length}['${(ele.name+"")}'] || "Any";
private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                        
                                            
                                            `  
                    } else {
                    
                        codigo = `
sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
me.${(ele.name+"")} = dim("${ele.name}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                        
                                            
                                            ` 
                    
                    }
                
                
                } else if (ele.tipo == "var_extend-def") {
                    
                    if ((ele.subname[0].nm+"").substr(0,8) == "private.") {
                        
                        codigo = `



private.${generator_one(ele.subname, variable, "class").substr(` private.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "class")}), "Any")
                        
                                            
                                            ` 
                    } else {
                    
                    codigo = `



me.${generator_one(ele.subname, variable, "class").substr(` localvalue${multimodos.length}.`.length)} = dim("subname", (${generator_one(ele.eval, variable, "class")}), "Any")
                        
                                            
                                            ` 
                    };
                    
                } else if (ele.tipo == "dim-def") {
                    
                    
                    let codi = `

                    setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${ele.name}') 
                                                                
                                                                `;
                    if(ele.eval != false) {
                        if ((ele.name+"").substr(0,8) == "private.") {
                    
                            codigo = `
    
    sta${multimodos.length}['${(ele.name+"")}'] = sta${multimodos.length}['${(ele.name+"")}'] || "Any";
    private.${(ele.name+"").substr(8)} = dim("${(ele.name+"")}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                            
                                                
                                                `  
                        } else {
                        
                            codigo = `
    sta${multimodos.length}['${ele.name}'] = sta${multimodos.length}['${ele.name}'] || "Any";
    me.${(ele.name+"")} = dim("${ele.name}", ${generator_one(ele.eval, variable, "class")}, sta${multimodos.length})
                            
                                                
                                                ` 
                        
                        }
                    } else {
                        codigo = `localvalue${multimodos.length}.${ele.name} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||{()=>{}})()`
                    };

                    codigo = codi + ";" +codigo; 
                    
                    
                } else if (ele.tipo == "flecha-def") {
                    
                    codigo = "";

                    for (let i = 0; i < ele.name.length; i++) {
                        const e = ele.name[i];
                        codigo = codigo + `

setsta(sta${multimodos.length}, localvalue${multimodos.length}.${ele.dim}, '${e.nm}') 
                                            
                                            `;
        codigo = codigo+ `;localvalue${multimodos.length}.${e.nm} = ((localvalue${multimodos.length}.${ele.dim}||{}).__default__||(()=>{}))()`

                        
                    }
                        
                    
                    
                } else if (ele.tipo == "class-def") {
                    
                    let gen = generator(ele.code.data, variable, "class");
                    let gen2 = gen.split("$");
                    //vi = gen2[0].split(",")

                    let io_obj = "";
                    for (let l = 0; l < ele.ext.length; l++) {
                        const elemento = ele.ext[l].nm;
                        ele.ext[l] = elemento;
                        io_obj = io_obj + "localvalue"+(multimodos.length+1)+"." + elemento +",";
                    }

                    


                    codigo = `
let iii${i} = me;
me.${ele.name} = function(${gen2[0]}) {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let este_c = iii${i}.${ele.name};
    let private = {};
    let exportar = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    let me = cla.decode([${io_obj}], private, localvalue${multimodos.length+1}, sta${multimodos.length+1});
    
    
    me.__classname__ = pkg_name + '.${ele.name}';

    ${
        gen2.slice(1).join("$")
    }
    me.__export__ = Object.keys(exportar);
    me.${ele.name}.__export__ = exportar;
    me.${ele.name} = Object.assign(me.${ele.name}, statico);
    return(me);
};
delete iii${i};
me.${ele.name}.__instance__ = false;
me.${ele.name}.__instance__ = cla.encode(me.${ele.name}());
me.__classname__ = pkg_name + '.${ele.name}'
                    
                    `

                } else if (ele.tipo == "module-def") {
                    
                    
                    
                    
                    let gen = generator(ele.code.data, variable, "module");
                    
                    //vi = gen2[0].split(",")

                    


                    codigo = `
let iii${i} = me;
me.${ele.name} = (function() {
    let localvalue${multimodos.length+1} = Object.assign({}, localvalue${multimodos.length});
    let este_c = iii${i}.${ele.name};
    let me = {};
    let private = {};
    let sta${multimodos.length+1} = Object.assign({}, sta${multimodos.length});
    me.__classname__ = "Module";

    ${
        gen
    }
    return(me);
})();
delete iii${i};
me.${ele.name}.__instance__ = true;
//me.__instance__ = cla.encode(me.${ele.name}());
//me.${ele.name}.__classname__ = pkg_name + '.${ele.name}';
me.${ele.name}.__classname__ = 'Module';
                    
                    `
                    
                    

                }
                
            }


        }
        if (modo == "class") {
            addcode(codigo)
        } else {
            addcode(codigo) 
        }
        
        
    }
    if (modo == "class") {
        outcode = just_class+ "$" + outcode
    }

    if (count) {
        multimodos.pop()
    }
    
    return (
     outcode
    )
}

let apistring = "";

function tf() {
    return tf.caller;
}

function setsta(sta, clase, name) {
    
    clase = clase || {}
    sta[name] = clase.__classname__ || "Any"
}

function valuepass(d) {
    code = d;
    if (typeof(code) =="boolean") {
        code = std.bool(code);
    } else if (typeof(code) =="number") {
        let code1 = code+""
        if (code1.indexOf(".") != -1) {
            code = std.int(code);
            
        } else {
            code = std.float(code);
        }
    } else if (typeof(code) =="string") {
        code = std.str(code);
    } else if (typeof(code) =="undefined") {
        code = undefined;
    } else if (typeof(code) =="function") {
        code = code;
    } else {
        code = code;
    }

    return code
}
let test_count = 0
function dim(name, code, sta) {
    let sie =false;

    //sta["var out"] = "Any";

    try {

        if (typeof(code) != "string") {
            if (((code).toString() +"") == "NaN") {
                sie = true                
            }
            
        }   
        
        
    } catch (e) {
        if (typeof(code) != "string") {
            if ((code+"") == "NaN") {
                sie = true                
            }
            
        }    
    }
    if (sie){
        error(`Error semantico al intentar procesar un valor que resulto en NaN`)

    }

    //if ("var out" == name) console.log(sta)
	if (typeof(sta) == "string") {
        sta={
            "meh":sta
        };
        name = "meh"  
    };

    if(sta[name] == "Any") {
        
        code = valuepass(code);

        return code;
    } else if(sta[name] == "String") {
        return (std.str(code+""));
    } else if(sta[name] == "Int") {
        if (typeof(code) == "number") {
            return std.int(parseInt(code))
        } else if (typeof(code) == "string") {
            return std.int((code+"").length);

        } else {
            return std.int(0)
        }
    } else if(sta[name] == "Float") {
        if (typeof(code) == "number") {
            return std.float(parseFloat(code))
        } else if (typeof(code) == "string") {
            return std.float((code+"").length);

        } else {
            return std.float(0)
        }
    } else if(sta[name] == "Boolean") {
        if (typeof(code) == "boolean") {
            return std.bool(code)
        } else {
            return std.bool(code);
        }
    } else if(sta[name] == "Array") {
        return std.array(code)
    } else if(sta[name] == "Void") {
        return undefined
    } else {

        let code2 = code || {};
        if (sta[name] == code2.__classname__) {
            return(code);
        } else {

            

            let bob = code2.__classname__;
            if (code == undefined) {
                bob = "Void"
            } else if (["number"].includes(typeof(code))) {
                if ((code+"").includes(".")) {
                    bob = "Float";
                } else {
                    bob = "Int"
                }
            } else if (["string", "boolean"].includes(typeof(code))) {
                bob = ({string:"String", "boolean":"Boolean"})[typeof(code)]
            }else if (Array.isArray(code)) {
                bob = "Array"
            };
            
            
            error(`un valor del tipo '${bob}' no puede estar dentro de una variable asignada como '${sta[name]}'`)

        }


    }
    

};

function error(msg) {
    if (cls_execute == "native") {
        throw `${msg}`
    } else {
        throw `${msg}`
        //console.error(`${msg}`)
    }
};
let mode_cls = "cls";

cadena = (a) => {
    return a
};
let appdata = {};

let cla = {
    encode: function (value) {
        return (value)
    },
    decode: function (value, private, lc, st) {
        
        let me = {};
        let exportar = {};
        let statico = {};

        for (let i = 0; i < value.length; i++) {
            let e = value[i].__instance__;
            let add_exportar = Object.keys(value[i].__export__);
            let add_statico = Object.keys(value[i]);
            let y = Object.keys(e)
            for (let x = 0; x < y.length; x++) {
                let ma = y[x];
                if (typeof(e[ma]) == "function") {
                    if (e[ma].__type__ == "Function") {
                        let kat=(e[ma].toString().split("/**/").slice(0,7))
                        let kat_post=(e[ma].toString().split("/**/").slice(7).join("/**/"))
                        let bit =(`(${kat[0]} 
                            /**/let ${(kat[2]+"").substr(2, (kat[2]+"").length-4)} = Object.assign({}, lc);
                            /**/${kat[2]}/**/
                            /**/let ${(kat[5]+"").substr(2, (kat[5]+"").length-4)} = Object.assign({}, lc);
                            /**/${kat[5]}/**/${kat[6]}/**/
                            
                            
                            ${kat_post}
                            
                            )`);
                        bit=(tools.func.replace_chars(bit, (kat[6]+"").substr(2, (kat[6]+"").length-4), "st"));
                        me[ma] = eval(bit)
                        
                    } else {
                        me[ma] = eval(e[ma].toString())
                        
                    }
                } else {
                    me[ma] = e[ma]
                }
            };
            for (let x = 0; x < add_statico.length; x++) {
                let eto = add_statico[x];
                if (eto.substr(0,2) !== "__") {
                    statico[eto] = me[eto]
                } 
            };
            for (let x = 0; x < add_exportar.length; x++) {
                let eto = add_exportar[x];
                exportar[eto] = value[i].__export__[eto] 
            }
            
        }

        return({
            me:me,
            exp:exportar,
            sta:statico
        })
    }
}
entero= cadena;
decimal = cadena;
lista = cadena;
let sta1 = {}
function execute(eva, variable=String("clstd")) {
    let rt_salida_exe = undefined;
    variable = variable||"clstd";
    try {
        rt_salida_exe = eval(`(function(){
            let pkg_name = 'main';
            let sta1 = {};
            varint.push(workspace.${variable})
            ${eva}
        })()`);
        
    } catch (error) {
        try {
            global.setexitcode((error+"").length)
        } catch (error) {
            
        }
        
        console.log(`cls a cerrado la ejecucion`)
        rt_salida_exe = (function (e) {
            let me = {
                result:e,
                code:crath_report[0],
                file:crath_report[1]
            };

            return (me);
        })(error)

    };
    
    return(rt_salida_exe)
}

function execute_imp(eva) {
    eval(eva)
    //let load = event.load;
}
let argumento_global = []
let cls_execute = "native"

try {
    module.exports;
} catch (error) {
    cls_execute = "web"
}

let script_evals=0
let arg_list_last = [];

let evalscript = function (data, key) {
    key = key || {};
    key.name = key.name || "script"+(script_evals++);
    key.work = key.work || "clstd";
    key.argv = key.argv || undefined;

    if (key.argv != undefined){
        arg_list_last.push(workspace[key.work].argv);
        Setargvs(key.argv, key.work)
    };

    let codigocrudo=desline(data, key.name);
    let inter = parselex(codigocrudo);
    let ibu = estructuration(inter, false, key.work);
    let exe = generator(ibu, key.work, "normal");

    workspace[key.work] = workspace[key.work]|| {};

    if (workspace[key.work].argv == undefined) {
        if (cls_execute == "native") {
            process.argv.WebArg = {}
            Setargvs(process.argv, key.work)    
        } else {
            let crug= (document.location.search.substr(1)).split("&");
            
            if ((crug+ "")== '') {
                Setargvs([], key.work)
            } else {
                let argvs = [];
                let args = {}
                for (let i = 0; i < crug.length; i++) {
                    const element = crug[i];
                    argvs[i] = decodeURI((element+"").split("=").slice(1).join("="))
                };
                for (let i = 0; i < crug.length; i++) {
                    const element = crug[i];
                    args[(element+"").split("=")[0]] = decodeURI((element+"").split("=").slice(1).join("="))
                };
                argvs.WebArg = args;
                Setargvs(argvs, key.work)
            }
            
        }
        
    }

    
    
    let exi = execute(exe, key.name);
    workspace[key.work].argv = arg_list_last.pop();
    return(exi);
};

function addstr(pre, func) {
    _str[pre] = func
};

function arraytostr(d) {
    salida = "[";
    for (let i = 0; i < d.length; i++) {
        const e = d[i];
        
        if (typeof(e) =="string") {
            salida = salida + `"${tools.func.replace_chars(e, N, BAR_INVERTD+"n")}", `
        } else if (typeof(e) =="number") {
            
            salida = salida + `${e}, `
            

        } else if (typeof(e) =="boolean") {
            salida = salida + `${e}, `
        } else if (typeof(e) =="undefined") {
            salida = salida + `undefined, `
        } else if (typeof(e) =="function") {
            salida = salida + `[Function func]`
        } else {
            if (typeof(e) =="object") {
                if (ty(e) == "String") {
                    salida = salida + `"${tools.func.replace_chars(e.toString(), N, BAR_INVERTD+"n")}", `
                } else if (ty(e) == "Int") {
                    salida = salida + `${e.toString()}, `
                } else if (ty(e) == "Float") {
                    salida = salida + `${e.toString()}, `
                } else if (ty(e) == "Boolean") {
                    salida = salida + `${e.toString()}, `
                } else if (Array.isArray(e)) {
                    salida = salida + `${arraytostr(e)}, `
                } else {
                    salida = salida + `{Object: ${e.__classname__||'Object'}}, `
                }
                
                
            }
        }
    }

    salida = (salida+"").substr(0, salida.length-2) + "]"

    if (salida == "]") salida = "[]"

    return salida
};

let vstd = ["String", "Int", "Float", "Boolean"];
let std = {
    str_old:(v) => {

        if (typeof(v) == "object") {if(v.toString != undefined){v=v+""} else {v = JSON.stringify(v)}} else {v= v+ ""};
        let data = v;
        let me = {
            toString:()=> {
                return data
            },
            __classname__:"String",
            concat:(d)=>{return std.str(data +""+ d)},
            replace:(o, n) => {return std.str(tools.func.replace_chars(data, o+"", n+""))},
            format:(d) => {
                let la = me.toString()
                let a = Object.entries(d);
                for (let i = 0; i < a.length; i++) {
                    const e = a[i];
                    la = tools.func.replace_chars(la, "${"+e[0]+"}", e[1]+"")
                    
                }

                return std.str(la)
            }
        };
    
    
        return(me)
    },
    str:(v) => {return v+""},
    int:(v) => {return parseInt(v)},
    float:(v) => {return parseFloat(v)},
    bool:(v) => {return Boolean(v)},
    int_old:(v) => {
        
        let data = parseInt(v)
        let parsep = parseInt
        let me = {
            add:(d) => {return std.int(data+parsep(d))},
            toString:() => {return data},
            mul:(d) => {return std.int(data*parsep(d))},
            div:(d) => {return std.int(data/parsep(d))},
            sub:(d) => {return std.int(data-parsep(d))},
            mod:(d) => {return std.int(data%parsep(d))},
            rai:(d) => {return std.int(data**parsep(d))},
            __classname__:"Int"
        };
        return me;
    },
    float_old:(v) => {
        
        let data = parseFloat(v)
        let parsep = parseFloat
        let me = {
            add:(d) => {return std.int(data+parsep(d))},
            toString:() => {return data},
            mul:(d) => {return std.int(data*parsep(d))},
            div:(d) => {return std.int(data/parsep(d))},
            sub:(d) => {return std.int(data-parsep(d))},
            mod:(d) => {return std.int(data%parsep(d))},
            rai:(d) => {return std.int(data**parsep(d))},
            __classname__:"Float"
        };
        return me;
    },
    bool_old:(v) => {
        if (typeof(v) == "boolean") {
            v = v
        } else if (typeof(v) == "object") {
            if (v.__classname__ == "Boolean") {
                v= v.toString();
            } else {
                if (["String", "Int", "Float"].includes(v.__classname__)) {
                    if (v.__classname__ == "String") {
                        if (v.toString() =="") {
                            v = false
                        } else {
                            v = true
                        }
                    } else {
                        if (v.toString() == 0) {
                            v = false
                        } else {
                            v = true
                        }
                    }
                } else {
                    if (Array.isArray(v)) {
                        v = !(v.length==0)
                    }
                }
            }
        } else {
            if (v) {
                v = true
            } else {
                v = false
            }
        };
        
        let data = v;
        let me = {
            __classname__:"Boolean",
            toString:() => {return(data)},
            invert:() => {data = !data}
        };
        return me;
    },
    array:(v) => {
        v = v||{}

        if (typeof(v)=="object") {
            if (ty(v) == "Array") {
                return v
            }
        }
        if (vstd.includes(ty(v))) {
            v = v.toString();
        };
        let arra = [];
        
        if (v == undefined) {
            arra = [];
        } else if (typeof(v) == "string") {
            for (let i = 0; i < v.length; i++) {
                const element = v[i];
                //arra[arra.length] = std.str(element);
                arra[arra.length] = (element);
            }
        } else if (Array.isArray(v)) {
            arra = v;
        } else if (typeof(v) == "object" | ty(arra) != "Array") {
            arra = Object.keys(v);
            let fari = [];

            for (let i = 0; i < arra.length; i++) {
                let elemento = arra[i];

                if (!(elemento.substr(0, 2) == "__" & elemento.substr(elemento.length-2) == "__")) fari[fari.length] = elemento;
                
            };
            arra = fari;
            
        } else {
            arra = [v];
        };
        
        //arra= []
        
        
        
        let me = arra;

        me.map = undefined;
        me.forEach = undefined;
        me.reduce = undefined;
        me.reduceRight = undefined;
        me.__classname__ = "Array";
        

        return(me);


    },
    function:(v, arg, ti) => {
        if (typeof(v) =="function") {
            return v
        };
        let code ="func __0x00_function__(" + (arg||"") + ") -> " + (ti||"any") +" {" + (v||"") + "};return(__0x00_function__)"
        let salida =std.exec(code);
        return salida;
    },
    exec:(code) => {
        code = "" + code
        let ceval =CLSJS();
        ceval.setappdata(appdata);
        ceval.setlib(libs);
        let impa = impweb_plt;
        let arr = Object.keys(impa);
        for (let i = 0; i < arr.length; i++) {
            const e = arr[i];
            const x = impa[e]
            ceval.addmodule(e, x)
        };
        ceval.Setargvs(workspace["clstd"].argv, "clstd");
        let l = ceval.run(code)
        return(l)
    },
    eval:(code) => {
        return valuepass(std.exec(`return(${code})`))
    }
};
function range_cls(start, end, paso) {
    let salida = [];if(typeof(start) != "number") start = 1;if(typeof(end) != "number") end = 1;if(typeof(paso) != "number") paso = 1; if (paso ==0) paso =1;for (let i = start; i < end; i= i+paso) {salida[salida.length] = i};return(std.array(salida))
}

function ty(d) {

    return (d.__classname__)
};




let __render__ = "";
let u_u;
let defstd = (function () {
    let me = {
        cml:(data) => {
            return cmlobj(recml(data));
        },
        int:(a) => {return parseInt(a||0)},
        float:(a) => {return parseFloat(a||0)},
        bool:(a) => {return Boolean(a||false)},
        str:(a) => {if (typeof(a)=="undefined") a="" ;return a+""},
        array:std.array,
        range:(start, end) => {
            let salida = [];
            for (let i = start; i < end; i++) {
                salida[salida.length] = i
            }
            return(std.array(salida))
        },
        ranger:(start, end) => {
            let salida = [];
            for (let i = start; i < end+1; i++) {
                salida[salida.length] = i
            }
            return(std.array(salida))
        },
        len:(v)=>{
            if (typeof((v||{}).length) == "undefined") {
                return ((""+v).length)
                
            } else {
                return v.length
            }
        },
        obj:()=>{
            let ___salida = {
                __type__:"object",
                getkey:(key)=>{
                    return ___salida[key];
                },
                setkey:(key, valkey)=> {
                    ___salida[key] = valkey;
                },
                __classname__:"Object" 
            };
            
            return (___salida)
        },
        void:() => {
            return(undefined);
        },
        eval:std.eval,
        func:std.function,
        function:std.function,
        exec:std.exec,
        any:(value) => {
            return(value)
        },
        __info__:()=> {
            
            return (`
Proyecto CLS 2017-2021

            version: ${Ver.ver}
            version del paquete: ${Ver.pkg}
            version del compilador: ${Ver.compiler}
            nombre del paquete web: ${Ver.web}
            
    Cls es un proyectos de codigo abierto creado por Carlos Pages
iniciando su desarrollo en 2017 y terminando en el 2021.

    Cls es una mescla de tipado estatico y dinamico, multi paradigma 
y orientado a objetos.

    El objetivo de cls es ser una herramienta para facilitar el trabajo
haciendo mas facil el uso de algunos aspectos de la programacion
que nos ahorraria tiempo de creacion.
            
            `)
        },
        char:(con, len) => {
            con = con || "";
            len = len || Infinity;
            let private = {
                value:(con+"").substr(0, len),
                len:len
            }
            let me = {
                __classname__:"Char",
                set:(val) => {
                    private.value = (val+"").substr(0, private.len);
                },
                get:() => {
                    return private.value;
                }
            }
            return me;
        },
        persist:(val)=>{let private=val;let me={set:(v)=>{private=v},get:()=>{return(private)},toString:()=>{return(private)},__classname__:"persist"};return(me);},
        true:true,
        on:true,
        false:false,
        off:false,
        "null":undefined,
        "undefined":undefined,
        infinity:Infinity,
        type:function (d) {
            let i = ty(d)
            return(i)
        }
        
        
        
    };
    function str_format(cadena){
        let valor_actual = varint[varint.length-1];
        let data1 = (cadena+"").split("{");
        let salida = "";
        let data2 = [];
        for (let i = 0; i < data1.length; i++) {
            let e = data1[i];
            if (i%2 == 0) {
                data2.push(e);
            } else {
                let out = e.split("}");
                if (out.length>1) {
                    if (!out[0].includes(" ")) {
                        try{data2.push(eval("valor_actual."+out[0]));} catch(e){data2.push("{"+out[0]+"}")};
                    }else {
                        data2.push("{"+out[0]+"}")
                    };
                    data2.push(out[1]);
                } else {
                    data2.push(out[0]);  
                }
            }
            
        };
        salida = data2.join("")
        return salida
    };
    me.str.add = addstr;
    me.str.__default__ = (d) => {return((d||"") + "")};
    me.int.__default__ = (d) => {return((d||0) + 0)};
    me.float.__default__ = (d) => {return((d||0) + 0)};
    me.void.__default__ = (d) => {return(undefined)};
    me.bool.__default__ = (d) => {return(Boolean(d||false))};
    me.obj.__default__ = (d) => {return( (d||me.obj()) )};
    me.array.__default__ = (d) => {return( me.array(d||[]) )};
    me.function.__default__ = (d) => {if (!(typeof(d) == "function")) d = (()=>{}); return( (d||(()=>{})) )};
    me.char.__default__ = (d) => {return( me.char(d||"", 1024))};
    me.cml.__default__ = (d) => {return( me.cml(d||"<div></div>"))};

    addstr("f", str_format)
    addstr("c", me.char);
    addstr("c4", (v) => {return me.char(v, 4)});
    addstr("c8", (v) => {return me.char(v, 8)});
    addstr("c16", (v) => {return me.char(v, 16)});
    addstr("c32", (v) => {return me.char(v, 32)});
    addstr("c64", (v) => {return me.char(v, 64)});
    addstr("c128", (v) => {return me.char(v, 128)});
    addstr("c256", (v) => {return me.char(v, 256)});
    addstr("c512", (v) => {return me.char(v, 512)});
    addstr("c1024", (v) => {return me.char(v, 1024)});
    addstr("i", (v) => {return me.int(v)});
    addstr("int", (v) => {return me.int(v)});
    addstr("float", (v) => {return me.float(v)});
    addstr("fl", (v) => {return me.float(v)});
    addstr("cml", (v) => {return cmlobj(recml(v))});
    addstr("len", (v) => {return v.length});
    me.str.__classname__ = "String";
    me.function.__classname__ = "Function";
    me.float.__classname__ = "Float";
    me.int.__classname__ = "Int";
    me.bool.__classname__ = "Boolean";
    me.array.__classname__ = "Array";
    me.void.__classname__ = "Void";
    me.any.__classname__ = "Any";
    me.obj.__classname__ = "Object";
    me.char.__classname__ = "Char";
    me.persist.__classname__ = "Persist";
    me.cml.__classname__ = "CML";
    let doc = "";
    try {doc = document} catch (e) {};
    me.__render__ = "console";
    __render__ = "console";
    if (!("" == doc)) {
        me.__render__ = "window";
        __render__ = "window";
        if (cls_execute == "web") {
            me.__render__ = "web";
            __render__ = "web";
        };
    };

    me.String = me.str;
    me.Float = me.float;
    me.Boolean = me.bool;
    me.Array = me.array;
    me.Object = me.obj;
    me.Integer = me.int;
    me.Any = me.any;

    return me
})


let apidefault = (function () {
    
    let me = {};

    if (js_active) {
        me.ActiveObjectJS=(v)=>{
            return eval(v);  
        }
        addstr("js", (v) => {return eval(v)});

    }

    if (all) {
        me.print=(txt) => {
            if (typeof(txt) =="object") {

                if (ty(txt) =="String") {
                    txt = txt+""
                };
                if (Array.isArray(txt)) {

                    txt = String(arraytostr(txt))
                };
                if (txt.toString) {
                    if (!(txt.toString() == "[object Object]")) {
                        txt = txt.toString()
                    }
                };
                
            } else {
                txt = txt
            }
            console.log(txt)
            for (let i = 0; i < renderizados.length; i++) {
                const element = renderizados[i];
                
                if ("TEXTAREA" == element.tagName) {
                    element.value = element.value + txt + N ;
                }
            }
            try {
                remoto.print(txt);
            } catch (eee) {
                
            }
        };
        me.write =(msg) => {
            salida = ""
            if (typeof(msg) == "object") {
                if (ty(msg) =="String") {
                    msg = msg+""
                }
                if (Array.isArray(msg)) {
                    msg = arraytostr(msg)
                };
                if (msg.toString) {
                    if (!(msg.toString() == "[object Object]")) {
                        msg = msg.toString()
                    }
                };
                
            }

            try {
                process.stdout.write(msg)
            } catch (error) {
                
            }

            for (let i = 0; i < renderizados.length; i++) {
                const element = renderizados[i];
                if ("TEXTAREA" == element.tagName) {
                    element.value = element.value + msg;
                }
            }
            try {
                remoto.write(txt);
            } catch (eee) {
                
            }
            
            return salida
        };
        me.input=(msg, func) => {
            if (ty(msg) =="String") {
                msg = msg+""
            }
            let doc = "";
            try {doc = document} catch (e) {};
            
            if ((doc == "")) {
                let read = require('readline');
                let mon = read.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                })
                
                mon.question(msg, (texto) => {

                    try {
                        
                        func(texto);

                        
                    } catch (er) {
                        console.log(er)
                        try {
                            func[1]("hubo un error durante la ejecucion de la funcion");
                        } catch (e) {
                            error("hubo un error durante la ejecucion de la funcion")
                        }
                        
                        
                    }
                    mon.close();
                });
            } else {
                
                if (cls_execute == "web") {
                    return(prompt(msg))
                }
            }


            

        },
        me.interval=(time, func) => {
            if (ty(time) =="Int") {
                msg = time+0
            };
            setInterval(func, parseInt(msg))
        };
        me.timeout=(time, func) => {
            if (ty(time) =="Int") {
                msg = time+0
            };
            setTimeout(func, parseInt(msg))
        }
        addstr("pr", (v) => {return me.print(v)});
        addstr("p", (v) => {return me.print(v)});
        addstr("wr", (v) => {return me.write(v)});

    }
    
    let doc = "";
    try {doc = document} catch (e) {};
    me.__render__ = "console";
    __render__ = "console";
    if (!("" == doc)) {
        me.__render__ = "window";
        __render__ = "window";
        if (cls_execute == "web") {
            me.__render__ = "web";
            __render__ = "web";
        };
        if (all) {
            let ma = {}
            
            

            document.head.innerHTML = document.head.innerHTML + `
            <style id="_cls_style"></style>
            `
            function node2str(data) {
                let salida = "";
                for (let i = 0; i < data.length; i++) {
                    const e = data[i];
                    if (typeof(e) == "string") {
                        salida = salida + e;                        
                    } else if (typeof(e) == "object") {
                        if (Object.keys(me.WebCLS.Tags).includes(e.tag)) {
                            salida = salida + me.WebCLS.Tags[e.tag](e);
                        } else {
                            salida = salida + me.WebCLS.Tags["tag"](e);
                        }
                        
                    }
                }

                return salida;
            };

            function glb_tag_opt(data) {
                return(` ip="${data.arg.id || ''}" class="${data.arg.class || ''}"  style="${data.arg.style || ''}"`)
            }

            let me_web = {
                WebCLS:{
                    getobj:(id) => {
                        let oo = document.getElementById(id);
                        let yo = {
                            id:id,
                            obj:oo,
                            set:{
                                style:(estilo) => {
                                    if (typeof(estilo) == "string" | typeof(estilo) == "object") {
                                        
                                        if (Array.isArray(estilo)) {
                                            if (typeof(estilo[0]) == "string" & typeof(estilo[1]) == "string") {
                                                oo.style[estilo[0]] = estilo[1]
                                            } else {
                                                for (let i = 0; i < estilo.length; i++) {
                                                    const element = estilo[i];
                                                    oo.style[element[0]] = element[1]
                                                }
                                            }
                                        } else {
                                            oo.style = estilo;

                                        }
                                        
                                    }
                                },
                                event:(eve, func) => {
                                    //evento
                                    
                                    oo.addEventListener(eve, (event) => {

                                        event = event|| {}
                                        event.__classname__ = "$Web.Event"
                                        func(event)
                                    })
                                },
                                render:(data) => {
                                    if (!(typeof(data) =="object")) {
                                        oo.innerHTML = data;
                                    } else if (data.__classname__ == "CML") {
                                        oo.innerHTML = node2str(data);
                                    } else {
                                        oo.innerHTML = "[Object]"
                                    }
                                },
                                value:(v) => {
                                    oo.value = v
                                }
                            },
                            get: {
                                content:() => {
                                    return(oo.innerHTML)
                                },
                                style:() => {
                                    return (oo.style)
                                },
                                value:() => {
                                    return(
                                        oo.value
                                    )
                                }
                            }
                        };

                        return(yo)
                        
                    },
                    gui:{
                        event:{
                            __classname__:"engine.event"
                        },
                        obj:{
                            __classname__:"control.obj"
                        },
                    },
                    document:{
                       // title: document.title,
                        event:(eve, func) => {
                            //evento
                            
                            window.addEventListener(eve, (event) => {

                                event = event|| {}
                                event.__classname__ = "engine.event"
                                func(event)
                            })
                        },
                       // style:"",
                        getcontent:() => {
                            return(document.body.innerHTML);
                        },
                        render:(data) => {
                            let m = node2str(data.node())
                            
                            document.body.innerHTML = m;
                        }
                    },
                    Tags:{
                        textbox:(data)=> {
                            return(`<input value="${data.nodes[0] || ''}" ${glb_tag_opt(data)}>`)
                        },
                        button:(data)=> {
                            return(`<input type="button" value="${data.nodes[0] || ''}" ${glb_tag_opt(data)}>`)
                        },
                        cml:(data)=> {
                            return(`<div>
                                ${node2str(data.nodes)}
                            </div>`)
                        },
                        el:() => {return("<br>")},
                        sp:() => {return("<hr>")},
                        tag:(data)=> {
                            return(`<div ${glb_tag_opt(data)}>
                                ${node2str(data.nodes)}
                            </div>`)
                        },
                        box:(data)=> {
                            return(`<div ${glb_tag_opt(data)}>
                                ${node2str(data.nodes)}
                            </div>`)
                        },
                    },
                },
                node2str:node2str,
                document:document,
                window:window,
                CLSJS:CLSJS,
                localStorage:localStorage,
                sessionStorage:sessionStorage,
                get:{
                    id:o=>capturar(document.getElementById(o)),
                    query:o=>document.querySelector(o),
                    name:o=>document.getElementsByName(o),
                    style:()=>document.getElementById("_cls_style").innerHTML,
                },
                set:{
                    style:(o)=>{document.getElementById("_cls_style").innerHTML = o},

                },
                prompt:(t) => prompt(t),
                
            };
            function capturar(o) {
                let me=o;
                me.setevent = me.addEventListener;
                
                return(me)  
            };
            me_web.WebCLS.getobj.__classname__ = "control.obj";
            
            
            addstr("getobj", (v) => {return me_web.WebCLS.getobj(v)});







            let ma_a = Object.keys(me);
            let ma_web = Object.keys(me_web);
            for (let i = 0; i < ma_a.length; i++) {
                const key = ma_a[i];
                const value = me[key];
                ma[key] = value;
            };

            for (let i = 0; i < ma_web.length; i++) {
                const key = ma_web[i];
                const value = me_web[key];
                ma[key] = value;
            };      

            me = ma;
            setInterval(()=>{
                //document.title = me.document.title+"";
                //document.getElementById("_cls_style").innerHTML = me.WebCLS.document.style+"";
            },100)
        }
        
        
    }

    return (me)

})()

let Setargvs= (arg, work) =>{

    workspace[work].argv = arg
    argumento_global = arg

};

let impweb = {};


let is_compile = false;

function setmode(mode) {
    mode_cls = (mode||"cls")+"";
};

modulo_cls.set_render_console = (obj) => {
    obj.value = ""
    renderizados.push(obj)
};
modulo_cls.set_compile_state = (bole) => {
    is_compile = bole;
};
modulo_cls.setmode = setmode;
modulo_cls.detect = enterval;
modulo_cls.desline = desline;
modulo_cls.estructuration = estructuration;
modulo_cls.generator = generator;
modulo_cls.parselex = parselex;
modulo_cls.addworkspace = addworkspace;
modulo_cls.execute = execute;
modulo_cls.run = evalscript;
modulo_cls.Setargvs = (arg, work) =>{

    workspace[work].argv = arg
    argumento_global = arg

};
modulo_cls.setlib = (l)=>{
    libs = l
};
modulo_cls.setappdata = (data) => {
    appdata = data
};
modulo_cls.getapi = (a) => {
    a=a||"clstd";
    return(workspace[a])
};
modulo_cls.setapires = (data) => {
    apicls = data;
};
modulo_cls.cle = (data) => {
    cle_play = true;
    try {
        eval("let pkg_name = 'main'; let sta1 = {};"+data);
    } catch (error) {
        cle_error = cle_error +"+- " + error+ N;
        fs.writeFileSync("error.log", cle_error);
        console.log(N+`+- Hubo un error, leer el archivo 'error.log' `)
    }
    
};
modulo_cls.addmodule = (name, modulo) =>{
    
    impweb[name] = `(${modulo.toString()})()`;
    impweb_plt[name] = modulo;
    return eval(`(${modulo.toString()})()`);
};
//modulo_cls.setapires(apidefault);
modulo_cls.addworkspace(apidefault, "clstd");

let impweb_plt = {};








return(modulo_cls)});

let _CLS;


try {
    module.exports = CLSJS;
} catch (error) {
    _CLS = CLSJS();
    
    
    
    let sss = localStorage.getItem("cls_vfs") || "{}";
    vfs = JSON.parse(sss);
    localStorage.setItem("cls_vfs", JSON.stringify(vfs));

    window.addEventListener("load", () => {
        let consolaa = document.getElementById("ccls");
        if (consolaa) {
            _CLS.set_render_console(consolaa);
        };
        let cl_scripts = document.getElementsByTagName("script");
        let ceeles = [];

        for (let i = 0; i < cl_scripts.length; i++) {
            const element = cl_scripts[i];
            
            if (element.getAttribute("type") == "script/cls") {
                ceeles.push(element);
            }
        };
        for (let i = 0; i < ceeles.length; i++) {
            const e = ceeles[i];

            let key = {argv:[], work:"", name:""};
            
            try {key.argv = eval(e.getAttribute("argv"))} catch (error) {key.argv=undefined};
            try {key.work = e.getAttribute("work")} catch (error) {key.work = undefined};
            try {key.name = e.getAttribute("file")} catch (error) {key.name = undefined};
            try {key.src = e.getAttribute("src")} catch (error) {key.src = undefined};
            
            if (key.src) {

                fetch(key.src).then(
                        (body) => {
                            body.text().then(
                                b => {
                                    _CLS.run(b, key)
                                }
                            )
                            
                        }
                    ).catch(
                        (e) => {
                            console.error(
                                `Error, no se a podido acceder al archivo '${key.src}'`
                            )
                        }
                    )
                
            } else {
                _CLS.run(e.innerHTML, key)
            }

            
        }
    })

}