use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tower_lsp::lsp_types::Url;

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Server {
    fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_clx"))
            .args(["lsp", "--silent"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("no se pudo iniciar clx lsp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn send(&mut self, message: Value) {
        let body = serde_json::to_vec(&message).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.stdin.write_all(&body).unwrap();
        self.stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    fn read(&mut self) -> Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            assert!(!line.is_empty(), "el servidor LSP cerró stdout");
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                content_length = Some(value.trim().parse::<usize>().unwrap());
            }
        }
        let len = content_length.expect("respuesta sin Content-Length");
        let mut body = vec![0u8; len];
        self.stdout.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).expect("JSON-RPC inválido")
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let mut message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if !params.is_null() {
            message["params"] = params;
        }
        self.send(message);
        loop {
            let message = self.read();
            if message.get("id") == Some(&json!(id)) {
                assert!(
                    message.get("error").is_none(),
                    "{} falló: {}",
                    method,
                    message
                );
                return message;
            }
        }
    }

    fn wait_notification(&mut self, method: &str) -> Value {
        loop {
            let message = self.read();
            if message.get("method").and_then(Value::as_str) == Some(method) {
                return message;
            }
        }
    }

    fn initialize(&mut self, root: &Path) -> Value {
        let root_uri = Url::from_directory_path(root).unwrap().to_string();
        let response = self.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {},
                "workspaceFolders": [{ "uri": root_uri, "name": "lsp-test" }],
            }),
        );
        self.notify("initialized", json!({}));
        response
    }

    fn shutdown(mut self) {
        let _ = self.request("shutdown", json!(null));
        self.notify("exit", json!(null));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn temp_workspace() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("clx-lsp-{}-{}", std::process::id(), stamp));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("cls.json"),
        r#"{"name":"lsp-test","version":"0.1.0","entry":"src/main.clsx"}"#,
    )
    .unwrap();
    root
}

fn file_uri(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

fn line_col(source: &str, needle: &str) -> (u32, u32) {
    for (line_no, line) in source.lines().enumerate() {
        if let Some(col) = line.find(needle) {
            return (line_no as u32, col as u32);
        }
    }
    panic!("no se encontró '{}'", needle);
}

fn completion_labels(response: &Value) -> Vec<String> {
    let result = &response["result"];
    let items = result
        .as_array()
        .or_else(|| result["items"].as_array())
        .expect("completion no devolvió items");
    items
        .iter()
        .filter_map(|item| item["label"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn lsp_local_completion() {
    let root = temp_workspace();
    let main_path = root.join("src/main.clsx");
    let main_uri = file_uri(&main_path);
    let source = r#"function add(a: int, b: int) -> int {
    var local: int = 1;
    var result: int = a + b + local;
    return result;
}

function nested(x: int) -> int {
    if (x > 0) {
        var positivo: int = x;
        return positivo;
    }
    return 0;
}

class Persona {
    var nombre: String;

    function saludar(otro: String) -> String {
        var saludo: String = otro;
        return saludo;
    }
}
"#;
    fs::write(&main_path, source).unwrap();

    let mut server = Server::start(&root);
    server.initialize(&root);
    server.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": main_uri,
                "languageId": "clx",
                "version": 1,
                "text": source,
            }
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");

    let (line, col) = line_col(source, "return result;");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 4 },
        }),
    );
    let labels = completion_labels(&completion);
    for expected in ["a", "b", "local", "result", "add", "Persona"] {
        assert!(
            labels.iter().any(|label| label == expected),
            "faltó {} en completion local: {:?}",
            expected,
            labels
        );
    }

    let (line, col) = line_col(source, "return positivo;");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 4 },
        }),
    );
    let labels = completion_labels(&completion);
    for expected in ["x", "positivo", "nested"] {
        assert!(
            labels.iter().any(|label| label == expected),
            "faltó {} en completion de bloque anidado: {:?}",
            expected,
            labels
        );
    }

    let (line, col) = line_col(source, "return result;");
    let hover = server.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 8 },
        }),
    );
    let hover_text = hover["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover_text.contains("result"),
        "hover local inesperado: {}",
        hover_text
    );

    let definition = server.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 8 },
        }),
    );
    let declaration_line = source
        .lines()
        .position(|line| line.contains("var result"))
        .unwrap() as u64;
    assert_eq!(
        definition["result"]["range"]["start"]["line"].as_u64(),
        Some(declaration_line),
        "definición local inesperada: {}",
        definition
    );

    let (line, col) = line_col(source, "return saludo;");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 4 },
        }),
    );
    let labels = completion_labels(&completion);
    for expected in ["me", "otro", "saludo", "add", "Persona"] {
        assert!(
            labels.iter().any(|label| label == expected),
            "faltó {} en completion de método: {:?}",
            expected,
            labels
        );
    }

    // Completion de miembro sobre una variable local con el archivo
    // momentáneamente inválido por el `.` final.
    let member_source = r#"class Persona {
    var nombre: String;
}

function makePersona() -> Persona {
    return Persona("Ana");
}

function describe() -> String {
    var p = makePersona();
    return p.
}
"#;
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": member_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");
    let (line, col) = line_col(member_source, "return p.");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 9 },
        }),
    );
    let labels = completion_labels(&completion);
    assert!(
        labels.iter().any(|label| label == "nombre"),
        "faltó nombre en completion de miembro local: {:?}",
        labels
    );

    // Navegación: la clase no debe apuntar al `}` y el campo debe resolver
    // a la declaración de `nombre`, no a cualquier símbolo global con ese nombre.
    let nav_source = r#"class Persona {
    var nombre: String;
}

function makePersona() -> Persona {
    return Persona("Ana");
}

function describe() -> String {
    var p = makePersona();
    var n: String = p.nombre;
    return n;
}
"#;
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 3 },
            "contentChanges": [{ "text": nav_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");

    let (line, col) = line_col(nav_source, "Persona(\"Ana\")");
    let definition = server.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 1 },
        }),
    );
    assert_eq!(
        definition["result"]["range"]["start"]["line"].as_u64(),
        Some(0),
        "la clase apuntó a una ubicación incorrecta: {}",
        definition
    );

    let (line, col) = line_col(nav_source, "p.nombre");
    let definition = server.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 2 },
        }),
    );
    assert_eq!(
        definition["result"]["range"]["start"]["line"].as_u64(),
        Some(1),
        "el campo apuntó a una ubicación incorrecta: {}",
        definition
    );

    let hover = server.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 2 },
        }),
    );
    let hover_text = hover["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover_text.contains("nombre"),
        "hover de campo inesperado: {}",
        hover_text
    );

    server.shutdown();
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn lsp_mvp() {
    let root = temp_workspace();
    let main_path = root.join("src/main.clsx");
    let lib_path = root.join("src/lib.clsx");
    let main_uri = file_uri(&main_path);
    let lib_uri = file_uri(&lib_path);

    let initial_source = r#"function add(a: int, b: int) -> int {
    return a + b;
}

var value: int = add(1, 2);
"#;
    fs::write(&main_path, initial_source).unwrap();
    fs::write(
        &lib_path,
        "function greet(name: String) -> String {\n    return name;\n}\n",
    )
    .unwrap();

    let mut server = Server::start(&root);
    let init = server.initialize(&root);
    let caps = &init["result"]["capabilities"];
    assert_eq!(caps["completionProvider"]["triggerCharacters"][0], ".");
    assert_eq!(caps["definitionProvider"], true);
    assert_eq!(caps["documentSymbolProvider"], true);
    assert_eq!(caps["hoverProvider"], true);

    // DidOpen con error de sintaxis -> diagnósticos.
    let broken = "function broken(\n";
    server.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": main_uri,
                "languageId": "clx",
                "version": 1,
                "text": broken,
            }
        }),
    );
    let diagnostics = server.wait_notification("textDocument/publishDiagnostics");
    assert!(
        !diagnostics["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty(),
        "se esperaban diagnósticos para código inválido"
    );

    // Volver a código válido; no debe haber errores.
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 2 },
            "contentChanges": [{ "text": initial_source }],
        }),
    );
    let diagnostics = server.wait_notification("textDocument/publishDiagnostics");
    let has_error = diagnostics["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["severity"] == 1);
    assert!(!has_error, "código válido generó errores: {}", diagnostics);

    // Completion de miembro sin triggerCharacter (Ctrl+Space después de math.).
    let dotted =
        "function add(a: int, b: int) -> int {\n    return a + b;\n}\n\nvar value: int = math.\n";
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 3 },
            "contentChanges": [{ "text": dotted }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");
    let (line, col) = line_col(dotted, "math.");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 5 },
        }),
    );
    let labels = completion_labels(&completion);
    assert!(
        labels.iter().any(|l| l == "sqrt"),
        "faltó sqrt: {:?}",
        labels
    );
    assert!(labels.iter().any(|l| l == "abs"), "faltó abs: {:?}", labels);

    let hover = server.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 1 },
        }),
    );
    let hover_text = hover["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover_text.contains("module math"),
        "hover de módulo inesperado: {}",
        hover_text
    );

    // Completion con `::` (namespace access).
    let colon_source =
        "function add(a: int, b: int) -> int {\n    return a + b;\n}\n\nvar value: int = math::\n";
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 4 },
            "contentChanges": [{ "text": colon_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");
    let (line, col) = line_col(colon_source, "math::");
    let completion = server.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 6 },
        }),
    );
    let labels = completion_labels(&completion);
    assert!(
        labels.iter().any(|l| l == "sqrt"),
        "faltó sqrt con :: {:?}",
        labels
    );

    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 5 },
            "contentChanges": [{ "text": initial_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");

    // Hover sobre una función local.
    let (line, col) = line_col(initial_source, "add(a");
    let hover = server.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 1 },
        }),
    );
    let hover_text = hover["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(
        hover_text.contains("add"),
        "hover inesperado: {}",
        hover_text
    );

    // Go-to-definition local.
    let (line, col) = line_col(initial_source, "add(1, 2)");
    let definition = server.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 1 },
        }),
    );
    let decl_line = initial_source
        .lines()
        .position(|l| l.contains("function add"))
        .unwrap() as u64;
    assert_eq!(
        definition["result"]["range"]["start"]["line"].as_u64(),
        Some(decl_line),
        "definición inesperada: {}",
        definition
    );

    // Go-to-definition a través de un import relativo.
    let import_source = r#"from "./lib" import greet;

function main() -> int {
    var message: String = greet("CLS");
    return 0;
}
"#;
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 6 },
            "contentChanges": [{ "text": import_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");
    let (line, col) = line_col(import_source, "greet(\"CLS\")");
    let definition = server.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": line, "character": col + 1 },
        }),
    );
    assert_eq!(
        definition["result"]["uri"].as_str(),
        Some(lib_uri.as_str()),
        "no resolvió el import: {}",
        definition
    );

    // Document symbols jerárquicos.
    server.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": main_uri, "version": 7 },
            "contentChanges": [{ "text": initial_source }],
        }),
    );
    let _ = server.wait_notification("textDocument/publishDiagnostics");
    let symbols = server.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": { "uri": main_uri } }),
    );
    let names: Vec<&str> = symbols["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(names.contains(&"add"), "símbolos: {:?}", names);
    assert!(names.contains(&"value"), "símbolos: {:?}", names);

    // Al cerrar el documento se limpian los diagnósticos.
    server.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": main_uri } }),
    );
    let diagnostics = server.wait_notification("textDocument/publishDiagnostics");
    assert!(diagnostics["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());

    server.shutdown();
    let _ = fs::remove_dir_all(&root);
}
