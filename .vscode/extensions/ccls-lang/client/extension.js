const vscode = require('vscode');
const { spawn } = require('child_process');

function activate(context) {
    console.log('[cls-lsp] Activando extension CLS...');

    // Buscar clx
    const clxPath = findClx();
    if (!clxPath) {
        vscode.window.showErrorMessage(
            'CLS LSP: clx no encontrado en PATH. ' +
            'Ejecuta "cargo build -p clx" o configura cls.server.path'
        );
        return;
    }

    // Spawn server process
    let serverProcess = spawn(clxPath, ['lsp'], {
        stdio: ['pipe', 'pipe', process.stderr],
        windowsHide: true
    });

    serverProcess.on('error', (err) => {
        vscode.window.showErrorMessage('CLS LSP: error al iniciar: ' + err.message);
    });

    serverProcess.on('exit', (code) => {
        console.log('[cls-lsp] Servidor terminado, codigo:', code);
    });

    // Crear coleccion de diagnostics
    const diagCollection = vscode.languages.createDiagnosticCollection('cls');
    context.subscriptions.push(diagCollection);

    // Leer respuestas del server (formato JSON-RPC)
    let buffer = '';
    serverProcess.stdout.on('data', (data) => {
        buffer += data.toString();
        // Procesar mensajes JSON-RPC del servidor
        processMessages(buffer, (msg) => {
            buffer = '';
            handleServerMessage(msg, diagCollection);
        });
    });

    // Registrar completion provider
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            provideCompletionItems(document, position) {
                const items = [
                    // Keywords
                    ...['var','function','if','else','while','for','return',
                       'import','from','as','export','structure','interface',
                       'true','false','null','break','continue','loop','switch'
                    ].map(k => new vscode.CompletionItem(k, vscode.CompletionItemKind.Keyword)),
                    // Intrinsics
                    ...['print','input','toString','int','float','str','bool',
                       'len','type','now','exit','sleep','throw'
                    ].map(f => {
                        const item = new vscode.CompletionItem(f, vscode.CompletionItemKind.Function);
                        item.detail = 'intrinsic';
                        return item;
                    }),
                    // Modules
                    ...['math','json','fs','http','Lib'].map(m => {
                        const item = new vscode.CompletionItem(m, vscode.CompletionItemKind.Module);
                        item.detail = 'module';
                        return item;
                    }),
                ];
                return items;
            }
        }, '.', '"', '/')
    );

    context.subscriptions.push({
        dispose: () => { serverProcess.kill(); }
    });

    console.log('[cls-lsp] Extension CLS activada');
}

function findClx() {
    const { execSync } = require('child_process');
    try {
        execSync('clx --version', { stdio: 'pipe' });
        return 'clx';
    } catch {
        // Buscar en ubicaciones comunes
        const candidates = [
            '../../target/debug/clx.exe',
            '../../target/release/clx.exe',
            process.env.HOME + '/.cargo/bin/clx',
            process.env.USERPROFILE + '\\.cargo\\bin\\clx',
        ];
        for (const c of candidates) {
            try {
                const p = require('path').resolve(__dirname, c);
                execSync(`"${p}" --version`, { stdio: 'pipe' });
                return p;
            } catch {}
        }
        return null;
    }
}

function processMessages(buffer, onMessage) {
    // JSON-RPC over stdin/stdout: Content-Length header
    const headerEnd = buffer.indexOf('\r\n\r\n');
    if (headerEnd === -1) return;
    const header = buffer.substring(0, headerEnd);
    const match = header.match(/Content-Length: (\d+)/);
    if (!match) return;
    const length = parseInt(match[1]);
    const bodyStart = headerEnd + 4;
    if (buffer.length < bodyStart + length) return;
    const body = buffer.substring(bodyStart, bodyStart + length);
    onMessage(JSON.parse(body));
}

function handleServerMessage(msg, diagCollection) {
    // Manejar notificaciones del servidor
    if (msg.method === 'textDocument/publishDiagnostics') {
        const params = msg.params;
        const uri = vscode.Uri.parse(params.uri);
        const diagnostics = params.diagnostics.map(d => {
            return new vscode.Diagnostic(
                new vscode.Range(d.range.start.line, d.range.start.character,
                                 d.range.end.line, d.range.end.character),
                d.message,
                d.severity === 1 ? vscode.DiagnosticSeverity.Error
                    : d.severity === 2 ? vscode.DiagnosticSeverity.Warning
                    : vscode.DiagnosticSeverity.Information
            );
        });
        diagCollection.set(uri, diagnostics);
    }
}

function deactivate() {}

module.exports = { activate, deactivate };
