const vscode = require('vscode');
const path = require('path');
const fs = require('fs');
const { spawn } = require('child_process');

function activate(context) {
    console.log('[cls] Activando...');

    // Cargar type maps builtins
    const builtinMaps = loadBuiltinMaps();
    let maptypeProcess = null;

    // ─── Lanzar clx maptype --watch en el workspace ─────────────────────
    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (wsRoot) {
        const clxPath = findClx();
        if (clxPath) {
            console.log('[cls] Generando type maps iniciales...');
            runSync(clxPath, ['maptype', '.', '-o', '.cls-types'], { cwd: wsRoot });
            // Luego iniciar watch mode
            maptypeProcess = spawn(clxPath, ['maptype', '.', '-o', '.cls-types', '--watch'], {
                cwd: wsRoot,
                stdio: ['ignore', 'ignore', 'pipe'],
                windowsHide: true
            });
            maptypeProcess.stderr.on('data', d => { /* silenciado */ });
            maptypeProcess.on('error', err => console.log('[cls] maptype error:', err.message));
            console.log('[cls] maptype --watch activo');
            // Indicador en status bar
            const statusItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
            statusItem.text = '$(sync) CLS Types';
            statusItem.tooltip = 'CLS Type Maps: auto-generando .cls-types/';
            statusItem.show();
            context.subscriptions.push(statusItem);
        } else {
            console.log('[cls] clx no encontrado, type maps no se generaran automaticamente');
        }
    }

    // ─── Cargar workspace type maps ─────────────────────────────────────
    reloadWorkspaceMaps();

    // ─── Recargar type maps en eventos ──────────────────────────────────
    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(doc => {
            if (isClsFile(doc)) reloadWorkspaceMaps();
        })
    );
    context.subscriptions.push(
        vscode.workspace.onDidSaveTextDocument(doc => {
            if (isClsFile(doc)) {
                // El watch process regenera automaticamente, solo recargar cache
                setTimeout(() => reloadWorkspaceMaps(), 500);
            }
        })
    );
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(() => {
            reloadWorkspaceMaps();
        })
    );

    // ─── Completion provider ────────────────────────────────────────────
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

    // Cleanup
    context.subscriptions.push({
        dispose: () => {
            if (maptypeProcess) { maptypeProcess.kill(); }
        }
    });

    console.log('[cls] Listo. Builtin modules:', builtinMaps.size);
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

function findClx() {
    const { execSync } = require('child_process');
    try { execSync('clx --version', { stdio: 'pipe' }); return 'clx'; } catch {}
    // Buscar en target/debug relativo al workspace
    const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (wsRoot) {
        const debugPath = path.join(wsRoot, 'target', 'debug', 'clx');
        const releasePath = path.join(wsRoot, 'target', 'release', 'clx');
        for (const c of [debugPath, releasePath]) {
            if (fs.existsSync(c) || fs.existsSync(c + '.exe')) {
                const bin = fs.existsSync(c) ? c : c + '.exe';
                try { execSync(`"${bin}" --version`, { stdio: 'pipe' }); return bin; } catch {}
            }
        }
    }
    // Fallback: ~/.cargo/bin/clx
    const home = process.env.HOME || process.env.USERPROFILE || '';
    const cargoBin = path.join(home, '.cargo', 'bin', 'clx');
    if (fs.existsSync(cargoBin) || fs.existsSync(cargoBin + '.exe')) {
        const bin = fs.existsSync(cargoBin) ? cargoBin : cargoBin + '.exe';
        try { execSync(`"${bin}" --version`, { stdio: 'pipe' }); return bin; } catch {}
    }
    return null;
}

function runSync(cmd, args, opts) {
    const { execSync } = require('child_process');
    try {
        const full = `"${cmd}" ${args.join(' ')}`;
        console.log('[cls] Running:', full);
        execSync(full, { ...opts, timeout: 60000, encoding: 'utf8' });
        console.log('[cls] OK:', full);
    } catch (e) {
        console.log('[cls] Error running maptype:', e.message);
    }
}

function deactivate() {}

module.exports = { activate, deactivate };
