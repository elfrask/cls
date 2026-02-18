let express = require("express");
let app = express();
let fs =  require("fs");


app.get("/CLS", (req, res) => {
    res.send(
        fs.readFileSync("./CLSJS.js", "utf-8")
    )
});
app.get("/CLS_modules", (req, res) => {
    res.send(
        fs.readFileSync("./CLS_modules.js", "utf-8")
    )
});


app.listen(2020, () => {
    console.log("CLS open server in the port 2020");
    console.log("in the:");
    console.log();
    console.log("- /CLS: CLS Engine");
    console.log("- /CLS_modules: CLS default modules web developer"); 
});