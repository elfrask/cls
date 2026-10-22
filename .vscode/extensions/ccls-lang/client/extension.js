const vscode = require('vscode');
const path = require('path');
const fs = require('fs');
const { spawn, spawnSync } = require('child_process');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

async function activate(context) {
    console.log('[cls] Activando...');

    const flags = getFeatureFlags();
    console.log('[cls] flags:', flags);

    // Cargar type maps builtins (solo se usan como fallback sin LSP).
    const builtinMaps = loadBuiltinMaps();
    let maptypeProcess = null;

    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    const clxPath = findClx(flags.serverPath);
    const useStaticTypes = !flags.lspServer && flags.useStaticTypes;
    let languageClient = null;

    // ─── Lanzar el servidor LSP ─────────────────────────────────────────
    if (flags.lspServer) {
        if (clxPath) {
            // No bloqueamos la activación esperando el handshake.
            startLanguageServer(clxPath, context, flags)
                .then(client => { languageClient = client; })
                .catch(err => {
                    console.error('[cls] Error al iniciar LSP:', err);
                    vscode.window.showErrorMessage(`No se pudo iniciar el LSP de CLS: ${err.message || err}`);
                });
        } else {
            console.log('[cls] clx no encontrado; el servidor LSP no se iniciará');
            vscode.window.showWarningMessage(
                'CLS: no se encontró el ejecutable `clx`. Configurá `cls.lsp.serverPath` o agregalo al PATH.'
            );
        }
    } else {
        console.log('[cls] LSP desactivado por cls.lsp.enabled=false');
    }

    // ─── Comando para reiniciar el LSP sin recargar la ventana ──────────
    context.subscriptions.push(
        vscode.commands.registerCommand('cls.restartLanguageServer', async () => {
            try {
                if (languageClient) {
                    await languageClient.stop();
                    languageClient = null;
                }
                const currentFlags = getFeatureFlags();
                if (!currentFlags.lspServer) {
                    vscode.window.showWarningMessage('CLS: el LSP está desactivado (`cls.lsp.enabled=false`).');
                    return;
                }
                const currentClx = findClx(currentFlags.serverPath);
                if (!currentClx) {
                    vscode.window.showWarningMessage(
                        'CLS: no se encontró `clx`. Configurá `cls.lsp.serverPath`.'
                    );
                    return;
                }
                languageClient = await startLanguageServer(currentClx, context, currentFlags);
                vscode.window.showInformationMessage('CLS: Language Server reiniciado.');
            } catch (err) {
                console.error('[cls] Error al reiniciar LSP:', err);
                vscode.window.showErrorMessage(`CLS: no se pudo reiniciar el LSP: ${err.message || err}`);
            }
        })
    );

    // ─── Fallback: type maps estáticos ──────────────────────────────────
    if (useStaticTypes && wsRoot) {
        if (clxPath) {
            console.log('[cls] Generando type maps iniciales...');
            runSync(clxPath, ['maptype', '.', '-o', '.cls-types'], { cwd: wsRoot });
            maptypeProcess = spawn(clxPath, ['maptype', '.', '-o', '.cls-types', '--watch'], {
                cwd: wsRoot,
                stdio: ['ignore', 'ignore', 'pipe'],
                windowsHide: true
            });
            maptypeProcess.stderr.on('data', () => { /* silenciado */ });
            maptypeProcess.on('error', err => console.log('[cls] maptype error:', err.message));
            console.log('[cls] maptype --watch activo');
            const statusItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
            statusItem.text = '$(sync) CLS Types';
            statusItem.tooltip = 'CLS Type Maps: auto-generando .cls-types/';
            statusItem.show();
            context.subscriptions.push(statusItem);
        } else {
            console.log('[cls] clx no encontrado; type maps no se generarán automáticamente');
        }
    } else {
        console.log('[cls] type maps estáticos desactivados (LSP activo o cls.types.useStaticTypes=false)');
    }

    // ─── Cargar workspace type maps (fallback) ──────────────────────────
    if (useStaticTypes) {
        reloadWorkspaceMaps();

        context.subscriptions.push(
            vscode.workspace.onDidOpenTextDocument(doc => {
                if (isClsFile(doc)) reloadWorkspaceMaps();
            })
        );
        context.subscriptions.push(
            vscode.workspace.onDidSaveTextDocument(doc => {
                if (isClsFile(doc)) {
                    setTimeout(() => reloadWorkspaceMaps(), 500);
                }
            })
        );
        context.subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor(() => {
                reloadWorkspaceMaps();
            })
        );
    }

    // ─── Completion provider (solo fallback sin LSP) ────────────────────
    if (useStaticTypes) {
        context.subscriptions.push(
            vscode.languages.registerCompletionItemProvider('clx', {
                provideCompletionItems(document, position) {
                    const linePrefix = document.lineAt(position).text.substring(0, position.character);
                    const items = [];

                    // Miembros de modulo
                    const dotMatch = linePrefix.match(/([a-zA-Z_][a-zA-Z0-9_]*)\.$/);
                    if (dotMatch) {
                        const allMaps = new Map([...builtinMaps.entries()]);
                        const wsMaps = globalThis._clsWorkspaceMaps || new Map();
                        for (const [modName, data] of wsMaps) {
                            if (!allMaps.has(modName)) allMaps.set(modName, data);
                        }
                        return completeMembers(dotMatch[1], allMaps);
                    }

                    // Keywords
                    for (const kw of ['var','function','if','else','while','for','return',
                        'import','from','as','export','structure','interface', "static", 
                        "is", "public", "protected", "private", "let", "const", "namespace",
                        "module", 'true','false','null','break','continue','loop','switch', "alias",
                        'async','await', 'readonly']) {
                        items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                    }

                    // Core intrinsics (top-level siempre)
                    if (builtinMaps.has('core')) {
                        for (const entry of builtinMaps.get('core').entries) {
                            if (entry.kind === 'variable' || entry.kind === 'function') {
                                const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                                ci.detail = entry.signature || entry.name;
                                ci.documentation = entry.doc || '';
                                items.push(ci);
                            }
                        }
                    }

                    // Workspace types: toplevel del archivo activo
                    const activeUri = document.uri;
                    const activePath = activeUri.fsPath || '';
                    let activeRel = '';
                    if (activePath.startsWith(wsRoot || '')) {
                        activeRel = activePath.substring((wsRoot || '').length + 1).replace(/\\/g, '/').replace(/\.clsx$/i, '');
                    }
                    const wsMaps = globalThis._clsWorkspaceMaps || new Map();
                    if (activeRel && wsMaps.has(activeRel)) {
                        for (const entry of wsMaps.get(activeRel).entries) {
                            if (entry.kind === 'function' || entry.kind === 'variable' || entry.kind === 'structure' || entry.kind === 'interface') {
                                const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                                ci.detail = entry.signature || entry.name;
                                ci.documentation = entry.doc || '';
                                items.push(ci);
                            }
                        }
                    }

                    // Modulos builtin
                    for (const [mName, data] of builtinMaps) {
                        if (mName === 'core') continue;
                        const mi = new vscode.CompletionItem(mName, vscode.CompletionItemKind.Module);
                        mi.detail = `module (${data.entries.length} members)`;
                        mi.documentation = `Import: import "${mName}" as ${mName}`;
                        items.push(mi);
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

                    return items;
                }
            }, '.', '"', '/')
        );
    }

    // Cleanup
    context.subscriptions.push({
        dispose: () => {
            if (maptypeProcess) { maptypeProcess.kill(); }
        }
    });

    console.log('[cls] Listo. Builtin modules:', builtinMaps.size);
}

function getFeatureFlags() {
    const config = vscode.workspace.getConfiguration('cls');
    return {
        lspServer: config.get('lsp.enabled', true),
        serverPath: config.get('lsp.serverPath', ''),
        trace: config.get('lsp.trace.server', 'off'),
        useStaticTypes: config.get('types.useStaticTypes', true),
        useMapClsi: config.get('types.useMapClsi', true),
    };
}

// ─── Type Maps ─────────────────────────────────────────────────────────

function loadBuiltinMaps() {
    const registry = new Map();
    const typesDir = path.resolve(__dirname, '../types');
    if (!fs.existsSync(typesDir)) return registry;
    for (const file of fs.readdirSync(typesDir)) {
        if (!file.endsWith('.type.json')) continue;
        const moduleName = file.replace('.type.json', '');
        try {
            const data = JSON.parse(fs.readFileSync(path.join(typesDir, file), 'utf8'));
            registry.set(moduleName, { entries: data.entries || [], source: data.source || '' });
        } catch (e) { /* skip */ }
    }
    return registry;
}

function reloadWorkspaceMaps() {
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (!workspaceRoot) return;
    const wsTypesDir = path.join(workspaceRoot, '.cls-types');
    if (!fs.existsSync(wsTypesDir)) return;

    const map = new Map();
    loadRecursive(wsTypesDir, map);
    globalThis._clsWorkspaceMaps = map;
}

function loadRecursive(dir, map) {
    for (const file of fs.readdirSync(dir)) {
        const full = path.join(dir, file);
        if (fs.statSync(full).isDirectory()) {
            loadRecursive(full, map);
        } else if (file.endsWith('.type.json')) {
            try {
                const data = JSON.parse(fs.readFileSync(full, 'utf8'));
                let relPath = (data.source || '').replace(/\\/g, '/').replace(/\.clsx$/i, '').replace(/\.clsi$/i, '');
                if (relPath.startsWith('./')) relPath = relPath.substring(2);
                map.set(relPath, { entries: data.entries || [], source: data.source });
            } catch (e) { /* skip */ }
        }
    }
}

function completeMembers(objName, registry) {
    const items = [];
    for (const [mName, data] of registry) {
        const entries = data.entries || data;
        if (mName === objName || objName.startsWith(mName + '.')) {
            for (const entry of entries) {
                const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                ci.detail = entry.signature || entry.name;
                ci.documentation = entry.doc || '';
                items.push(ci);
            }
        }
    }
    return items.length > 0 ? items : undefined;
}

// ─── Helpers ────────────────────────────────────────────────────────────

function isClsFile(doc) {
    return doc.languageId === 'clx' || doc.fileName.endsWith('.clsx') || doc.fileName.endsWith('.clsi');
}

function kindMap(kind) {
    const m = { function: vscode.CompletionItemKind.Function, variable: vscode.CompletionItemKind.Variable, structure: vscode.CompletionItemKind.Struct, interface: vscode.CompletionItemKind.Interface, class: vscode.CompletionItemKind.Class, module: vscode.CompletionItemKind.Module, constant: vscode.CompletionItemKind.Constant, import: vscode.CompletionItemKind.Reference };
    return m[kind] || vscode.CompletionItemKind.Text;
}

function snip(label, body) {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.insertText = new vscode.SnippetString(body);
    item.detail = 'snippet';
    return item;
}

function findClx(configuredPath) {
    const candidates = [];
    const add = (value) => {
        if (value && !candidates.includes(value)) candidates.push(value);
    };

    add(configuredPath);
    add(process.env.CLS_LSP_PATH);

    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (wsRoot) {
        add(path.join(wsRoot, 'target', 'debug', 'clx'));
        add(path.join(wsRoot, 'target', 'release', 'clx'));
    }

    const home = process.env.HOME || process.env.USERPROFILE || '';
    if (home) add(path.join(home, '.cargo', 'bin', 'clx'));

    const fromPath = findOnPath('clx');
    if (fromPath) add(fromPath);

    for (const candidate of candidates) {
        const executable = resolveExecutable(candidate);
        if (executable && canRun(executable)) return executable;
    }
    return null;
}

function findOnPath(name) {
    const pathValue = process.env.PATH || '';
    const extensions = process.platform === 'win32' ? ['.exe', '.cmd', '.bat', ''] : [''];
    for (const dir of pathValue.split(path.delimiter)) {
        if (!dir) continue;
        for (const ext of extensions) {
            const candidate = path.join(dir.replace(/^"|"$/g, ''), name + ext);
            if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
        }
    }
    return null;
}

function resolveExecutable(candidate) {
    if (!candidate) return null;
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
    if (process.platform === 'win32') {
        for (const ext of ['.exe', '.cmd', '.bat']) {
            if (fs.existsSync(candidate + ext)) return candidate + ext;
        }
    }
    return null;
}

function canRun(executable) {
    try {
        const result = spawnSync(executable, ['--version'], {
            stdio: 'ignore',
            windowsHide: true,
            timeout: 5000,
        });
        return result.status === 0;
    } catch {
        return false;
    }
}

async function startLanguageServer(clxPath, context, flags) {
    const outputChannel = vscode.window.createOutputChannel('CLS Language Server', { log: true });
    const traceOutputChannel = vscode.window.createOutputChannel('CLS LSP Trace', { log: true });
    context.subscriptions.push(outputChannel, traceOutputChannel);

    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;

    const serverOptions = {
        command: clxPath,
        args: ['lsp', '--silent'],
        transport: TransportKind.stdio,
        options: {
            cwd: wsRoot || undefined,
            env: { ...process.env },
        },
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'clx' }],
        synchronize: {
            fileEvents: [
                vscode.workspace.createFileSystemWatcher('**/*.clsx'),
                vscode.workspace.createFileSystemWatcher('**/*.clsi'),
                vscode.workspace.createFileSystemWatcher('**/cls.json'),
            ],
        },
        outputChannel,
        traceOutputChannel: flags.trace === 'off' ? undefined : traceOutputChannel,
        initializationOptions: {
            workspaceRoot: wsRoot || null,
        },
    };

    const client = new LanguageClient(
        'clsLanguageServer',
        'CLS Language Server',
        serverOptions,
        clientOptions
    );

    context.subscriptions.push(client);

    client.onDidChangeState(event => {
        console.log(`[cls] LSP state: ${event.oldState} -> ${event.newState}`);
    });

    await client.start();
    outputChannel.appendLine(`[cls] LSP conectado usando ${clxPath}`);
    console.log('[cls] LSP cliente conectado');

    const traceLevel = flags.trace === 'verbose' ? 3 : flags.trace === 'messages' ? 1 : 0;
    if (traceLevel > 0) {
        try {
            await client.setTrace(traceLevel);
            outputChannel.appendLine(`[cls] trace LSP: ${flags.trace}`);
        } catch (err) {
            console.log('[cls] no se pudo activar el trace LSP:', err.message);
        }
    }
    return client;
}

function runSync(cmd, args, opts) {
    try {
        console.log('[cls] Running:', cmd, args.join(' '));
        const result = spawnSync(cmd, args, {
            ...opts,
            timeout: 60000,
            encoding: 'utf8',
            windowsHide: true,
        });
        if (result.error) throw result.error;
        if (result.status !== 0) {
            throw new Error(`exit ${result.status}: ${result.stderr || ''}`.trim());
        }
        console.log('[cls] OK:', cmd, args.join(' '));
    } catch (e) {
        console.log('[cls] Error running maptype:', e.message);
    }
}

function deactivate() {}

module.exports = { activate, deactivate };
