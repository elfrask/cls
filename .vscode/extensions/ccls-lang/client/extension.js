const vscode = require('vscode');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

let reqId = 1;
const pending = {}; // id -> { resolve, reject }

function activate(context) {
    console.log('[cls-lsp] Activando...');
    const clxPath = findClx();
    if (!clxPath) {
        vscode.window.showErrorMessage('CLS LSP: clx no encontrado. Compila con: cargo build -p clx');
        return;
    }
    console.log('[cls-lsp] clx en:', clxPath);

    const serverProcess = spawn(clxPath, ['lsp', '--silent'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true
    });

    let buf = '';
    let ready = false;

    // Timer de reintento de initialize
    const initTimer = setInterval(() => {
        if (serverProcess?.stdin?.writable && !ready) {
            sendInitialize();
            ready = true;
        }
    }, 200);

    function sendInitialize() {
        const root = vscode.workspace.workspaceFolders?.[0]?.uri?.toString() || null;
        sendMsg({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: process.pid, capabilities: {}, rootUri: root } });
        sendMsg({ jsonrpc: '2.0', method: 'initialized', params: {} });
        clearInterval(initTimer);
    }

    function sendMsg(msg) {
        if (serverProcess?.stdin?.writable) {
            const body = JSON.stringify(msg);
            serverProcess.stdin.write(`Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n${body}`);
        }
    }

    // JSON-RPC request con promesa
    function request(method, params) {
        return new Promise((resolve, reject) => {
            const id = ++reqId;
            pending[id] = { resolve, reject };
            sendMsg({ jsonrpc: '2.0', id, method, params });
            setTimeout(() => { if (pending[id]) { delete pending[id]; reject(new Error('timeout')); } }, 5000);
        });
    }

    serverProcess.stdout.on('data', (data) => {
        buf += data.toString();
        processMessages();
    });
    serverProcess.on('error', (err) => { vscode.window.showErrorMessage('CLS LSP: ' + err.message); });
    serverProcess.stderr.on('data', () => {});

    function processMessages() {
        while (true) {
            const idx = buf.indexOf('\r\n\r\n');
            if (idx === -1) break;
            const match = buf.substring(0, idx).match(/Content-Length: (\d+)/i);
            if (!match) { buf = buf.substring(idx + 4); continue; }
            const len = parseInt(match[1]);
            const start = idx + 4;
            if (buf.length < start + len) break;
            const body = buf.substring(start, start + len);
            buf = buf.substring(start + len);
            const msg = JSON.parse(body);
            // Si es respuesta a un request, resolver promesa
            if (msg.id && pending[msg.id]) {
                if (msg.error) pending[msg.id].reject(new Error(msg.error.message));
                else pending[msg.id].resolve(msg.result);
                delete pending[msg.id];
            } else if (msg.method) {
                handleNotification(msg);
            }
        }
    }

    // Diagnostics
    const diagCollection = vscode.languages.createDiagnosticCollection('cls');
    context.subscriptions.push(diagCollection);

    function handleNotification(msg) {
        if (msg.method === 'textDocument/publishDiagnostics') {
            const p = msg.params;
            const uri = vscode.Uri.parse(p.uri);
            const diags = p.diagnostics.map(d => new vscode.Diagnostic(
                new vscode.Range(d.range.start.line, d.range.start.character,
                                 d.range.end.line, d.range.end.character),
                d.message,
                d.severity === 1 ? vscode.DiagnosticSeverity.Error
                    : d.severity === 2 ? vscode.DiagnosticSeverity.Warning
                    : vscode.DiagnosticSeverity.Information
            ));
            diagCollection.set(uri, diags);
        }
    }

    // Completion provider que consulta al LSP server
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            async provideCompletionItems(document, position) {
                const items = [];

                // Keywords (locales)
                for (const kw of ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch']) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                // Snippets
                items.push(snip('if', 'if (${1:cond}) {\n\t$0\n}'));
                items.push(snip('for', 'for (${1:i}=0; ${1:i}<${2:n}; ${1:i}++) {\n\t$0\n}'));
                items.push(snip('while', 'while (${1:cond}) {\n\t$0\n}'));
                items.push(snip('function', 'function ${1:name}(${2:params}) -> ${3:void} {\n\t$0\n}'));
                items.push(snip('structure', 'structure ${1:Name} {\n\t${2:field}: ${3:type}\n};'));
                items.push(snip('import', 'import "${1:module}" as ${2:alias};'));
                items.push(snip('from', 'from "${1:module}" import ${2:func};'));
                items.push(snip('ternary', '${1:cond} ? ${2:then} : ${3:else}'));

                // Consultar al LSP server
                if (serverProcess?.stdin?.writable) {
                    try {
                        const linePrefix = document.lineAt(position).text.substring(0, position.character);
                        const triggerChar = linePrefix.endsWith('.') ? '.' : '';
                        const result = await request('textDocument/completion', {
                            textDocument: { uri: document.uri.toString() },
                            position: { line: position.line, character: position.character },
                            context: { triggerKind: triggerChar ? 2 : 1, triggerCharacter: triggerChar || undefined }
                        });
                        if (result && result.items) {
                            for (const item of result.items) {
                                const ci = new vscode.CompletionItem(item.label, mapKind(item.kind));
                                ci.detail = item.detail || '';
                                ci.documentation = item.documentation || '';
                                items.push(ci);
                            }
                        }
                    } catch (e) {
                        console.log('[cls-lsp] completion error:', e.message);
                    }
                }

                return items;
            }
        }, '.', '"', '/', '>')
    );

    // Enviar didOpen/didChange al server
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (doc.languageId === 'clx') {
                sendMsg({
                    jsonrpc: '2.0', method: 'textDocument/didOpen',
                    params: { textDocument: { uri: doc.uri.toString(), languageId: 'clx', version: 1, text: doc.getText() } }
                });
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument(event => {
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

    context.subscriptions.push({
        dispose: () => { clearInterval(initTimer); serverProcess.kill(); diagCollection.clear(); }
    });

    console.log('[cls-lsp] Lista');
}

function snip(label, body) {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.insertText = new vscode.SnippetString(body);
    item.detail = 'snippet';
    return item;
}

function mapKind(k) {
    const map = { 1: vscode.CompletionItemKind.Text, 2: vscode.CompletionItemKind.Method, 3: vscode.CompletionItemKind.Function, 4: vscode.CompletionItemKind.Constructor, 5: vscode.CompletionItemKind.Field, 6: vscode.CompletionItemKind.Variable, 7: vscode.CompletionItemKind.Class, 8: vscode.CompletionItemKind.Interface, 9: vscode.CompletionItemKind.Module, 10: vscode.CompletionItemKind.Property, 11: vscode.CompletionItemKind.Unit, 12: vscode.CompletionItemKind.Value, 13: vscode.CompletionItemKind.Enum, 14: vscode.CompletionItemKind.Keyword, 15: vscode.CompletionItemKind.Snippet, 16: vscode.CompletionItemKind.Color, 17: vscode.CompletionItemKind.File, 18: vscode.CompletionItemKind.Reference, 19: vscode.CompletionItemKind.Constant, 20: vscode.CompletionItemKind.Struct, 21: vscode.CompletionItemKind.Event, 22: vscode.CompletionItemKind.Operator, 23: vscode.CompletionItemKind.TypeParameter };
    return map[k] || vscode.CompletionItemKind.Text;
}

function findClx() {
    const { execSync } = require('child_process');
    try { execSync('clx --version', { stdio: 'pipe' }); return 'clx'; } catch {}
    const extDir = path.resolve(__dirname, '..');
    for (const c of [
        path.resolve(extDir, '../../../target/debug/clx'),
        path.resolve(extDir, '../../../target/release/clx'),
        path.join(process.env.HOME || process.env.USERPROFILE || '', '.cargo', 'bin', 'clx'),
    ]) {
        try { if (fs.existsSync(c)) { execSync(`"${c}" --version`, { stdio: 'pipe' }); return c; } } catch {}
    }
    return null;
}

function deactivate() {}

module.exports = { activate, deactivate };
