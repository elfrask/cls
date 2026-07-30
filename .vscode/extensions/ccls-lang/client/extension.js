const vscode = require('vscode');
const path = require('path');
const fs = require('fs');

function activate(context) {
    console.log('[cls] Activando...');

    // Cargar type maps (builtin + workspace)
    const typeRegistry = loadTypeMaps(context);

    // Completion provider con type maps
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider('clx', {
            provideCompletionItems(document, position) {
                const linePrefix = document.lineAt(position).text.substring(0, position.character);
                const items = [];

                // Si termina en '.', completar miembros del objeto
                const dotMatch = linePrefix.match(/([a-zA-Z_][a-zA-Z0-9_]*)\.$/);
                if (dotMatch) {
                    const objName = dotMatch[1];
                    // Buscar en todos los type maps (builtins + workspace)
                    const allMaps = new Map([...typeRegistry.entries()]);
                    const wsMaps = globalThis._clsWorkspaceMaps || new Map();
                    for (const [modName, data] of wsMaps) {
                        if (!allMaps.has(modName)) allMaps.set(modName, data);
                    }
                    return completeMembers(objName, allMaps);
                }

                // Keywords
                for (const kw of ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch',
                    'async','await']) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                // Core: funciones globales en top-level (sin import)
                if (typeRegistry.has('core')) {
                    for (const entry of typeRegistry.get('core').entries) {
                        if (entry.kind === 'variable' || entry.kind === 'function') {
                            const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                            ci.detail = entry.signature || entry.name;
                            ci.documentation = entry.doc || '';
                            items.push(ci);
                        }
                    }
                }

                // Workspace types toplevel: entradas del archivo actual
                const activeUri = document.uri;
                const activePath = activeUri.fsPath || '';
                const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath || '';
                let activeRelPath = '';
                if (activePath.startsWith(wsRoot)) {
                    activeRelPath = activePath.substring(wsRoot.length + 1).replace(/\\/g, '/').replace(/\.clsx$/i, '');
                }
                const wsMaps = globalThis._clsWorkspaceMaps || new Map();
                if (wsMaps.has(activeRelPath)) {
                    for (const entry of wsMaps.get(activeRelPath).entries) {
                        if (entry.kind === 'function' || entry.kind === 'variable' || entry.kind === 'structure' || entry.kind === 'interface') {
                            const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                            ci.detail = entry.signature || entry.name;
                            ci.documentation = entry.doc || '';
                            items.push(ci);
                        }
                    }
                }

                // Modulos builtin (math, json, fs...) → solo nombre
                for (const [moduleName, data] of typeRegistry) {
                    if (moduleName === 'core') continue;
                    const modItem = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
                    modItem.detail = `module (${data.entries.length} members)`;
                    modItem.documentation = `Import: import "${moduleName}" as ${moduleName}`;
                    items.push(modItem);
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

    console.log('[cls] Listo. Type maps cargados:', typeRegistry.size);
}

// ─── Type Map Registry ────────────────────────────────────────────────

function loadTypeMaps(context) {
    const registry = new Map();       // moduleName -> { entries, source }
    const workspaceMaps = new Map();  // relativePath -> { entries, source }

    // 1. Builtins desde la extension
    const typesDir = path.resolve(__dirname, '../types');
    if (fs.existsSync(typesDir)) {
        for (const file of fs.readdirSync(typesDir)) {
            if (file.endsWith('.type.json')) {
                const moduleName = file.replace('.type.json', '');
                try {
                    const data = JSON.parse(fs.readFileSync(path.join(typesDir, file), 'utf8'));
                    registry.set(moduleName, { entries: data.entries || [], source: data.source || '' });
                } catch (e) {
                    console.log('[cls] Error loading type map:', file, e.message);
                }
            }
        }
    }

    // 2. Workspace .cls-types/ (preserva estructura de directorios)
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (workspaceRoot) {
        const wsTypesDir = path.join(workspaceRoot, '.cls-types');
        if (fs.existsSync(wsTypesDir)) {
            loadWorkspaceTypes(wsTypesDir, wsTypesDir, workspaceRoot, workspaceMaps);
        }
    }

    // Store workspace maps globally
    globalThis._clsWorkspaceMaps = workspaceMaps;
    return registry;
}

function loadWorkspaceTypes(dir, baseDir, workspaceRoot, map) {
    for (const file of fs.readdirSync(dir)) {
        const full = path.join(dir, file);
        const stat = fs.statSync(full);
        if (stat.isDirectory()) {
            loadWorkspaceTypes(full, baseDir, workspaceRoot, map);
        } else if (file.endsWith('.type.json')) {
            try {
                const data = JSON.parse(fs.readFileSync(full, 'utf8'));
                // source es relativo al workspace root (ej: "examples/tests/hello.clsx")
                let relPath = (data.source || '').replace(/\\/g, '/').replace(/\.clsx$/i, '').replace(/\.clsi$/i, '');
                if (relPath.startsWith('./')) relPath = relPath.substring(2);
                map.set(relPath, { entries: data.entries || [], source: data.source });
            } catch (e) {
                console.log('[cls] Error:', e.message);
            }
        }
    }
}

function completeMembers(objName, registry) {
    const items = [];
    for (const [moduleName, data] of registry) {
        const entries = data.entries || data;  // support both formats
        if (moduleName === objName || objName.startsWith(moduleName + '.')) {
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

function kindMap(kind) {
    const map = { 'function': vscode.CompletionItemKind.Function, 'variable': vscode.CompletionItemKind.Variable, 'structure': vscode.CompletionItemKind.Struct, 'interface': vscode.CompletionItemKind.Interface, 'class': vscode.CompletionItemKind.Class, 'module': vscode.CompletionItemKind.Module, 'import': vscode.CompletionItemKind.Reference };
    return map[kind] || vscode.CompletionItemKind.Text;
}

function snip(label, body) {
    const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Snippet);
    item.insertText = new vscode.SnippetString(body);
    item.detail = 'snippet';
    return item;
}

function deactivate() {}

module.exports = { activate, deactivate };
