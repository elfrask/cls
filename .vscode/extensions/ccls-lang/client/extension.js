const vscode = require('vscode');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

function activate(context) {
    console.log('[cls] Activando...');

    // Configuracion desde settings.json
    const config = vscode.workspace.getConfiguration('cls.options.unnestableFeatures');
    const enableLsp = config.get('lspServer', false);
    const useMapClsi = config.get('useMapClsi', true);

    // ─── Syntax highlighting y snippets (SIEMPRE) ────────────────────────

    // Completion provider con keywords + snippets
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            provideCompletionItems() {
                const items = [];
                for (const kw of ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch']) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }
                items.push(snip('if', 'if (${1:cond}) {\n\t$0\n}'));
                items.push(snip('for', 'for (${1:i}=0; ${1:i}<${2:n}; ${1:i}++) {\n\t$0\n}'));
                items.push(snip('while', 'while (${1:cond}) {\n\t$0\n}'));
                items.push(snip('function', 'function ${1:name}(${2:params}) -> ${3:void} {\n\t$0\n}'));
                items.push(snip('structure', 'structure ${1:Name} {\n\t${2:field}: ${3:type}\n};'));
                items.push(snip('import', 'import "${1:module}" as ${2:alias};'));
                items.push(snip('from', 'from "${1:module}" import ${2:func};'));
                items.push(snip('ternary', '${1:cond} ? ${2:then} : ${3:else}'));
                return items;
            }
        }, '.', '"', '/')
    );

    console.log('[cls] Resaltado + snippets activos');

    // ─── LSP Server (OPCIONAL, desactivado por defecto) ──────────────────

    if (!enableLsp) {
        console.log('[cls] LSP desactivado (lspServer: false)');
        return;
    }

    const clxPath = findClx();
    if (!clxPath) {
        vscode.window.showErrorMessage('CLS LSP: clx no encontrado. Compila con: cargo build -p clx');
        return;
    }
    console.log('[cls] Iniciando LSP...');

    const serverProcess = spawn(clxPath, ['lsp', '--silent'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true
    });

    let buf = '';
    let reqId = 1;
    const pending = {};

    function sendMsg(msg) {
        if (serverProcess?.stdin?.writable) {
            const body = JSON.stringify(msg);
            serverProcess.stdin.write(`Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n${body}`);
        }
    }

    function request(method, params) {
        return new Promise((resolve, reject) => {
            const id = ++reqId;
            pending[id] = { resolve, reject };
            sendMsg({ jsonrpc: '2.0', id, method, params });
            setTimeout(() => { if (pending[id]) { delete pending[id]; reject(new Error('timeout')); } }, 3000);
        });
    }

    // Initialize inmediatamente
    const root = vscode.workspace.workspaceFolders?.[0]?.uri?.toString() || null;
    sendMsg({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { processId: process.pid, capabilities: {}, rootUri: root } });
    sendMsg({ jsonrpc: '2.0', method: 'initialized', params: {} });

    serverProcess.stdout.on('data', (data) => {
        buf += data.toString();
        processMessages();
    });
    serverProcess.stderr.on('data', () => {});
    serverProcess.on('error', (err) => console.log('[cls] LSP error:', err.message));

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
            if (msg.id && pending[msg.id]) {
                if (msg.error) pending[msg.id].reject(new Error(msg.error.message));
                else pending[msg.id].resolve(msg.result);
                delete pending[msg.id];
            } else if (msg.method === 'textDocument/publishDiagnostics') {
                handleDiag(msg.params);
            }
        }
    }

    const diagCollection = vscode.languages.createDiagnosticCollection('cls');
    context.subscriptions.push(diagCollection);

    function handleDiag(params) {
        const uri = vscode.Uri.parse(params.uri);
        const diags = params.diagnostics.map(d => new vscode.Diagnostic(
            new vscode.Range(d.range.start.line, d.range.start.character, d.range.end.line, d.range.end.character),
            d.message,
            d.severity === 1 ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning
        ));
        diagCollection.set(uri, diags);
    }

    // Enviar documentos abiertos
    vscode.workspace.onDidOpenTextDocument((doc) => {
        if (doc.languageId === 'clx') {
            sendMsg({ jsonrpc: '2.0', method: 'textDocument/didOpen', params: { textDocument: { uri: doc.uri.toString(), languageId: 'clx', version: 1, text: doc.getText() } } });
        }
    });

    vscode.workspace.onDidChangeTextDocument((event) => {
        if (event.document.languageId === 'clx') {
            sendMsg({ jsonrpc: '2.0', method: 'textDocument/didChange', params: { textDocument: { uri: event.document.uri.toString(), version: event.document.version }, contentChanges: [{ text: event.document.getText() }] } });
        }
    });

    context.subscriptions.push({ dispose: () => { serverProcess.kill(); diagCollection.clear(); } });

    console.log('[cls] LSP activo');
}

function snip(label, body) {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.insertText = new vscode.SnippetString(body);
    item.detail = 'snippet';
    return item;
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
