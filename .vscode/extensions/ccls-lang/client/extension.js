const vscode = require('vscode');
const { spawn } = require('child_process');
const path = require('path');

function activate(context) {
    const clxPath = findClx();
    if (!clxPath) {
        vscode.window.showErrorMessage('CLS LSP: clx no encontrado');
        return;
    }

    // Usar stdin/stdout (estandar LSP), sin stderr noise
    const serverProcess = spawn(clxPath, ['lsp', '--silent'], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true
    });

    // Buffer de lectura
    let buf = '';

    serverProcess.stdout.on('data', data => {
        buf += data.toString();
        processMessages();
    });

    // Loggear stderr del server solo si hay error
    serverProcess.stderr.on('data', () => {});  // silenciado

    serverProcess.on('error', err => {
        vscode.window.showErrorMessage('CLS LSP: ' + err.message);
    });

    const diagCollection = vscode.languages.createDiagnosticCollection('cls');
    context.subscriptions.push(diagCollection);

    function send(msg) {
        if (serverProcess && serverProcess.stdin.writable) {
            const body = JSON.stringify(msg);
            const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
            serverProcess.stdin.write(header + body);
        }
    }

    function processMessages() {
        const idx = buf.indexOf('\r\n\r\n');
        if (idx === -1) return;
        const m = buf.substring(0, idx).match(/Content-Length: (\d+)/);
        if (!m) return;
        const len = parseInt(m[1]);
        const start = idx + 4;
        if (buf.length < start + len) return;
        const body = buf.substring(start, start + len);
        buf = buf.substring(start + len);
        handle(JSON.parse(body));
        processMessages();
    }

    function handle(msg) {
        if (msg.method === 'textDocument/publishDiagnostics') {
            const p = msg.params;
            const uri = vscode.Uri.parse(p.uri);
            const diags = p.diagnostics.map(d => new vscode.Diagnostic(
                new vscode.Range(d.range.start.line, d.range.start.character,
                                 d.range.end.line, d.range.end.character),
                d.message,
                d.severity === 1 ? vscode.DiagnosticSeverity.Error : vscode.DiagnosticSeverity.Warning
            ));
            diagCollection.set(uri, diags);
        }
    }

    // Inicializar
    setTimeout(() => {
        send({
            jsonrpc: '2.0', id: 1, method: 'initialize',
            params: { processId: process.pid, capabilities: {}, rootUri: null }
        });
        send({ jsonrpc: '2.0', method: 'initialized', params: {} });
    }, 500);

    // Completions locales
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            provideCompletionItems() {
                const kws = ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch'];
                const fns = ['print','input','toString','int','float','str','bool',
                    'len','type','now','exit','sleep','throw'];
                const mods = ['math','json','fs','http','Lib'];
                return [
                    ...kws.map(k => new vscode.CompletionItem(k, vscode.CompletionItemKind.Keyword)),
                    ...fns.map(f => { const i = new vscode.CompletionItem(f, vscode.CompletionItemKind.Function); i.detail = 'intrinsic'; return i; }),
                    ...mods.map(m => { const i = new vscode.CompletionItem(m, vscode.CompletionItemKind.Module); i.detail = 'module'; return i; }),
                ];
            }
        }, '.', '"', '/')
    );

    // Enviar docs al servidor
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (doc.languageId === 'clx') {
                send({
                    jsonrpc: '2.0', method: 'textDocument/didOpen',
                    params: { textDocument: { uri: doc.uri.toString(), languageId: 'clx', version: 1, text: doc.getText() } }
                });
            }
        })
    );

    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument(event => {
            if (event.document.languageId === 'clx') {
                send({
                    jsonrpc: '2.0', method: 'textDocument/didChange',
                    params: { textDocument: { uri: event.document.uri.toString(), version: event.document.version },
                             contentChanges: [{ text: event.document.getText() }] }
                });
            }
        })
    );

    context.subscriptions.push({
        dispose: () => { serverProcess.kill(); }
    });
}

function findClx() {
    const { execSync } = require('child_process');
    try { execSync('clx --version', { stdio: 'pipe' }); return 'clx'; } catch {}
    for (const c of [
        path.resolve(__dirname, '../../target/debug/clx'),
        path.resolve(__dirname, '../../target/release/clx'),
        path.join(process.env.HOME || process.env.USERPROFILE || '', '.cargo', 'bin', 'clx'),
    ]) {
        try { execSync(`"${c}" --version`, { stdio: 'pipe' }); return c; } catch {}
    }
    return null;
}

function deactivate() {}

module.exports = { activate, deactivate };
