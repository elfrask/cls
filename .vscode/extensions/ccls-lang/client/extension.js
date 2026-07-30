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
                    return completeMembers(objName, typeRegistry);
                }

                // Keywords
                for (const kw of ['var','function','if','else','while','for','return',
                    'import','from','as','export','structure','interface',
                    'true','false','null','break','continue','loop','switch',
                    'async','await']) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                // Entradas de type maps (intrinsics + modulos)
                for (const [moduleName, entries] of typeRegistry) {
                    for (const entry of entries) {
                        if (entry.kind === 'variable' || entry.kind === 'function') {
                            const ci = new vscode.CompletionItem(entry.name, kindMap(entry.kind));
                            ci.detail = entry.signature || entry.name;
                            ci.documentation = entry.doc || moduleName;
                            items.push(ci);
                        }
                    }
                    // El nombre del modulo como completion
                    const modItem = new vscode.CompletionItem(moduleName, vscode.CompletionItemKind.Module);
                    modItem.detail = `module (${entries.length} members)`;
                    modItem.documentation = `Module: ${moduleName}`;
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
    const registry = new Map();

    // 1. Builtins desde la extension
    const typesDir = path.resolve(__dirname, '../types');
    if (fs.existsSync(typesDir)) {
        for (const file of fs.readdirSync(typesDir)) {
            if (file.endsWith('.type.json')) {
                const moduleName = file.replace('.type.json', '');
                try {
                    const data = JSON.parse(fs.readFileSync(path.join(typesDir, file), 'utf8'));
                    registry.set(moduleName, data.entries || []);
                } catch (e) {
                    console.log('[cls] Error loading type map:', file, e.message);
                }
            }
        }
    }

    // 2. Workspace .clsi-types/ (sobrescribe builtins)
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
    if (workspaceRoot) {
        const wsTypesDir = path.join(workspaceRoot, '.clsi-types');
        if (fs.existsSync(wsTypesDir)) {
            for (const file of fs.readdirSync(wsTypesDir)) {
                if (file.endsWith('.type.json')) {
                    const moduleName = file.replace('.type.json', '');
                    try {
                        const data = JSON.parse(fs.readFileSync(path.join(wsTypesDir, file), 'utf8'));
                        registry.set(moduleName, data.entries || []);
                    } catch (e) {
                        console.log('[cls] Error loading workspace type map:', file, e.message);
                    }
                }
            }
        }
    }

    return registry;
}

function completeMembers(objName, registry) {
    const items = [];
    for (const [moduleName, entries] of registry) {
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
