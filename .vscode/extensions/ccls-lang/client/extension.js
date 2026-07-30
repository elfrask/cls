const vscode = require('vscode');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

function activate(context) {
    console.log('[cls-lsp] Activando extension...');

    const clxPath = findClx();
    if (!clxPath) {
        vscode.window.showErrorMessage(
            'CLS LSP: clx no encontrado.\n' +
            'Compilalo con: cargo build -p clx\n' +
            'O agrega target/debug/ a tu PATH'
        );
        return;
    }

    console.log('[cls-lsp] usando clx en:', clxPath);

    // 1. Spawn server process
    const serverProcess = spawn(clxPath, ['lsp', '--silent'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true
    });

    let serverReady = false;
    let buf = '';
    let pendingDiags = []; // cola de diagnostics pendientes hasta que el server responda initialize

    // Inicializacion LSP: enviar initialize inmediatamente
    function sendInitialize() {
        const root = vscode.workspace.workspaceFolders?.[0]?.uri?.toString() || null;
        sendMsg({
            jsonrpc: '2.0',
            id: 1,
            method: 'initialize',
            params: {
                processId: process.pid,
                capabilities: {},
                rootUri: root,
                workspaceFolders: null
            }
        });
        sendMsg({ jsonrpc: '2.0', method: 'initialized', params: {} });
        serverReady = true;
        // Enviar cola de diagnostics pendientes
        for (const { uri, text } of pendingDiags) {
            sendDidOpen(uri, text);
        }
        pendingDiags = [];
    }

    function sendMsg(msg) {
        if (serverProcess?.stdin?.writable) {
            const body = JSON.stringify(msg);
            const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
            serverProcess.stdin.write(header + body);
        }
    }

    function sendDidOpen(uri, text) {
        sendMsg({
            jsonrpc: '2.0', method: 'textDocument/didOpen',
            params: { textDocument: { uri, languageId: 'clx', version: 1, text } }
        });
    }

    // Cuando stdout recibe datos del server, procesar JSON-RPC
    serverProcess.stdout.on('data', (data) => {
        buf += data.toString();
        processMessages();
    });

    // Server listo apenas spawna
    serverProcess.on('spawn', () => {
        sendInitialize();
    });

    serverProcess.on('error', (err) => {
        if (err.code === 'ENOENT') {
            vscode.window.showErrorMessage('CLS LSP: binario no encontrado en: ' + clxPath);
        } else {
            vscode.window.showErrorMessage('CLS LSP: ' + err.message);
        }
    });

    serverProcess.on('exit', (code) => {
        console.log('[cls-lsp] Servidor terminado, codigo:', code);
    });

    // Stderr silenciado
    serverProcess.stderr.on('data', () => {});

    // Parse JSON-RPC con Content-Length
    function processMessages() {
        while (true) {
            const idx = buf.indexOf('\r\n\r\n');
            if (idx === -1) break;
            const m = buf.substring(0, idx).match(/Content-Length: (\d+)/i);
            if (!m) { buf = buf.substring(idx + 4); continue; }
            const len = parseInt(m[1]);
            const start = idx + 4;
            if (buf.length < start + len) break;
            const body = buf.substring(start, start + len);
            buf = buf.substring(start + len);
            handleMessage(JSON.parse(body));
        }
    }

    // Manejar mensajes del server
    const diagCollection = vscode.languages.createDiagnosticCollection('cls');
    context.subscriptions.push(diagCollection);

    function handleMessage(msg) {
        if (msg.method === 'textDocument/publishDiagnostics') {
            const p = msg.params;
            const uri = vscode.Uri.parse(p.uri);
            const diags = p.diagnostics.map(d => {
                const sev = d.severity === 1 ? vscode.DiagnosticSeverity.Error
                          : d.severity === 2 ? vscode.DiagnosticSeverity.Warning
                          : vscode.DiagnosticSeverity.Information;
                return new vscode.Diagnostic(
                    new vscode.Range(d.range.start.line, d.range.start.character,
                                     d.range.end.line, d.range.end.character),
                    d.message, sev
                );
            });
            diagCollection.set(uri, diags);
        }
    }

    // Register completion provider (local, no depende del server)
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            provideCompletionItems(document, position) {
                const linePrefix = document.lineAt(position).text.substring(0, position.character);
                const items = [];

                // Keywords
                const kws = ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch'];
                for (const kw of kws) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                // Snippets
                items.push(snippet('if', 'if (${1:cond}) {\n\t$0\n}'));
                items.push(snippet('for', 'for (${1:i}=0; ${1:i}<${2:n}; ${1:i}++) {\n\t$0\n}'));
                items.push(snippet('while', 'while (${1:cond}) {\n\t$0\n}'));
                items.push(snippet('function', 'function ${1:name}(${2:params}) -> ${3:void} {\n\t$0\n}'));
                items.push(snippet('structure', 'structure ${1:Name} {\n\t${2:field}: ${3:type}\n};'));
                items.push(snippet('interface', 'interface ${1:Name} {\n\t${2:method}(${3:params}): ${4:ret}\n};'));
                items.push(snippet('import', 'import "${1:module}" as ${2:alias};'));
                items.push(snippet('from', 'from "${1:module}" import ${2:func};'));
                items.push(snippet('ternary', '${1:cond} ? ${2:then} : ${3:else}'));

                return items;
            }
        }, '.', '"', '/', '>')
    );

    // Enviar didOpen para cada archivo .clsx que se abra
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument((doc) => {
            if (doc.languageId === 'clx') {
                if (serverReady) {
                    sendDidOpen(doc.uri.toString(), doc.getText());
                } else {
                    pendingDiags.push({ uri: doc.uri.toString(), text: doc.getText() });
                }
            }
        })
    );

    // Enviar didChange al editar
    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            if (event.document.languageId === 'clx') {
                sendMsg({
                    jsonrpc: '2.0', method: 'textDocument/didChange',
                    params: {
                        textDocument: { uri: event.document.uri.toString(), version: event.document.version },
                        contentChanges: [{ text: event.document.getText() }]
                    }
                });
            }
        })
    );

    // Enviar didClose
    context.subscriptions.push(
        vscode.workspace.onDidCloseTextDocument((doc) => {
            if (doc.languageId === 'clx') {
                sendMsg({
                    jsonrpc: '2.0', method: 'textDocument/didClose',
                    params: { textDocument: { uri: doc.uri.toString() } }
                });
            }
        })
    );

    // Cleanup
    context.subscriptions.push({
        dispose: () => {
            serverProcess.kill();
            diagCollection.clear();
        }
    });

    console.log('[cls-lsp] Extension activada');
}

function snippet(label, body) {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.insertText = new vscode.SnippetString(body);
    item.detail = 'snippet';
    return item;
}

function findClx() {
    const { execSync } = require('child_process');
    // 1. Intentar PATH
    try { execSync('clx --version', { stdio: 'pipe' }); return 'clx'; } catch {}

    // 2. Buscar relativo a la extension -> target/debug/clx
    // __dirname = .vscode/extensions/ccls-lang/client/
    // ../..      = .vscode/extensions/ccls-lang/
    const extDir = path.resolve(__dirname, '..');
    const candidates = [
        path.resolve(extDir, '../../../target/debug/clx'),       // workspace/target/debug/clx
        path.resolve(extDir, '../../../target/release/clx'),    // workspace/target/release/clx
        path.join(process.env.HOME || process.env.USERPROFILE || '', '.cargo', 'bin', 'clx'),
    ];

    for (const c of candidates) {
        try {
            if (fs.existsSync(c)) {
                execSync(`"${c}" --version`, { stdio: 'pipe' });
                return c;
            }
        } catch {}
    }
    return null;
}

function deactivate() {}

module.exports = { activate, deactivate };
