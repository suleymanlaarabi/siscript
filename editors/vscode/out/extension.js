"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    const lspPath = vscode.workspace.getConfiguration('siscript').get('lsp.path') || 'si-lsp';
    const trace = vscode.workspace.getConfiguration('siscript').get('lsp.trace') || 'off';
    const serverOptions = {
        command: lspPath,
        args: ['--stdio']
    };
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'siscript' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{si,lang}')
        },
        traceOutputChannel: vscode.window.createOutputChannel('Siscript LSP Trace')
    };
    client = new node_1.LanguageClient('siscriptLsp', 'Siscript Language Server', serverOptions, clientOptions);
    client.start();
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map