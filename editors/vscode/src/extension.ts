import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const lspPath = vscode.workspace.getConfiguration('siscript').get<string>('lsp.path') || 'si-lsp';
    const trace = vscode.workspace.getConfiguration('siscript').get<string>('lsp.trace') || 'off';

    const serverOptions: ServerOptions = {
        command: lspPath,
        args: ['--stdio']
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'siscript' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{si,lang}')
        },
        traceOutputChannel: vscode.window.createOutputChannel('Siscript LSP Trace')
    };

    client = new LanguageClient(
        'siscriptLsp',
        'Siscript Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
