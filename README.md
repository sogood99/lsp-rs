# Basic LSP Server

This repo focuses on designing and implementing modules for handling messages in the Language Server Protocol (LSP) format using Rust. The official LSP specifications can be seen in the [homepage](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification), which involves encoding and decoding messages, parsing specific LSP requests, and processing them accordingly. 

### What are LSP's

The Language Server Protocol (LSP) is a client-server architecture designed to establish communication between code editors and language servers. For instance, when a user hovers over a function within an editor equipped with LSP, they will be presented with the documentation pertaining to that function.

```
#[tokio::main]
async fn main() {               ╭──────────────────────────────────────────────────────────────────────────╮
    let listener = TcpListener::│tokio::net::tcp::listener::TcpListener                                    │
        .await                  │                                                                          │
        .unwrap();              │pub async fn bind<A>(addr: A) -> io::Result<TcpListener>                  │
    loop {                      │where                                                                     │
        let (socket, _) = listen│    A: ToSocketAddrs,                                                     │
        tokio::spawn(async move │──────────────────────────────────────────────────────────────────────────│
            connection_handler(s│Creates a new TcpListener, which will be bound to the specified address.  │
        });                     │                                                                          │
    }                           │The returned listener is ready for accepting connections.                 │
}                               │                                                                          │
```
This is done because the code editor acts as a LSP client, and sends a hover request to the LSP server(in this case, the server is rust-analyzer). These language servers provide programming language-specific functionalities like code completion, syntax highlighting, and code navigation. LSP utilizes JSON-RPC messages for communication. An example message in the LSP format is: 
```json
Content-Length: 233\r\n\r\n
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///path/to/file.txt",
      "languageId": "java",
      "version": 1,
      "text": "public class MyClass {\n  public void myMethod() {\n    // Implement me\n  }\n}"
    }
  }
}
```
The example JSON message illustrates an LSP "textDocument/didOpen" notification sent from the editor to the server. This informs the server about a newly opened document (file:///path/to/file.txt) with its language ("java") and initial content.

### Serde JSON
To parse the JSON content, this project uses serde_json. Serde JSON is a Rust library that provides support for serializing and deserializing data in JSON format. It allows converting Rust data structures to JSON representations and vice versa. The user simply has to label the struct with `#[derive(Serialize, Deserialize, Debug)]`, and can then serialize the struct to and from JSON. The following example shows how serde_json is used:
```rust
use serde::{Deserialize, Serialize};
use serde_json::Result;

// Define a struct representing a person
#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u32,
    is_student: bool,
}

fn main() {
    // Create an instance of the Person struct
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        is_student: false,
    };

    // Serialize the person struct to JSON
    let json_str = serde_json::to_string(&person)?;
    println!("Serialized JSON: {}", json_str);

    // Deserialize JSON string back into a Person struct
    let deserialized_person: Person = serde_json::from_str(&json_str)?;
    println!("Deserialized Person: {:?}", deserialized_person);
}
```

### ABC Language

As an illustration of the language server, we create a simple, domain-specific language. ABC is a toy language designed to represent complete binary search trees. A complete binary tree has nodes at every level, except possibly the last, is completely filled. It uses a concise notation to specify the structure of the tree, making it easy to define and manipulate BSTs in a textual format.

#### Syntax

An example ABC file is:
```
a
b c
d e f
```
This represents the tree:
```
        a
      /   \
     b     c
    / \   /
   d   e f
```
More specifically, each line specifies the nodes at that depth, each node is represented by a singe character, with 2^d nodes in that depth. There is always a space between consecutive nodes. Since ABC represents complete binary trees, the nodes are filled left to right.

### Editor Module (`editor`)
The Editor module defines the structs (`FileState`, `EditorState`) for managing the editor and file states. Because code editors can have multiple files open at the same time, the `EditorState` should contain all the `FileState`s open. The module also implements functions for modifying file content and retrieving file state. The `FileStates` should have one to one correspondence with the file content (assuming the content represents a complete binary tree), with functions to retrieve parent and children. Ideally the `FileState` should be using an `Vec` to represent the binary tree. The `FileState` should also contain the character count.

### RPC Module (`rpc`)
The RPC module provides functions for encoding and decoding messages to and from LSP format. In the module, the `BufferReader` struct manages message buffers, and handles partial messages. `BufferReader` also implements `pop_message` to pop the message from the buffer if the buffer starts with a valid message, and contains error handling for message parsing failures. `EncodeMessage` should encode the message in the format: 
```
Content-Length: <Content Length>r\n
\r\n
<Content>
```
and `DecodeMessage` should take in the LSP message, verify the content-length, and return the content along with the total message size (containing `Content-Length: \r\n\r\n`).

### LSP Module (`lsp`)
The module contains message processing logic for each LSP request type. `lsp` module utilizes the `rpc` module for message encoding and decoding. The LSP module handles specific LSP message types such as `initialize`, `textDocument/didOpen`, `textDocument/didChange`, and `textDocument/hover`. Specifically, the LSP server does the following when encountering LSP messages:
- **Initialize**: repsonds with initialize response. A sample request from NeoVim is:
    ```json
    {"method":"initialize","params":{"workspaceFolders":null,"processId":437656,"clientInfo":{"version":"0.9.1","name":"Neovim"},"trace":"off","capabilities":{"window":{"workDoneProgress":true,"showMessage":{"messageActionItem":{"additionalPropertiesSupport":false}},"showDocument":{"support":true}}, ....}},"jsonrpc":"2.0","id":1}
    ```
    The server should respond with InitializeResponse
    ```json
    {"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1,"hoverProvider":true},"serverInfo":{"name":"LSP-Server","version":"0"}}}"
    ```
    and the client will then respond with `Intitialized`
    ```json
    {"method":"initialized","params":{},"jsonrpc":"2.0"}
    ```
- **didOpen** and **didChange**: update the `EditorState` to sync with the editor. A typical `didOpen` notification looks like such:
    ```json
    {"method":"textDocument\/didOpen","params":{"textDocument":{"text":"0\n5 1\n1 0 1 2\n","version":0,"uri":"file://path/to/your/file","languageId":"abc"}},"jsonrpc":"2.0"}
    ```

- **hover**: if the user hovers a node, return the parent of the node, eg. (█ is the cursor location) 
    ```
    0
    5 1
    1 █ 1 2
    ╭─────────╮
    │Parent: 5│
    ╰─────────╯
    ```
    and if the user hovers a blank space, return the number of characters in the document.
    ```
    0
    5 1
    1 0█1 2
    ╭───────────────────╮
    │Character count: 14│
    ╰───────────────────╯
    ```
    A typical hover request is:
    ```json
    {"method":"textDocument\/hover","params":{"position":{"character":0,"line":2},"textDocument":{"uri":"file://path/to/your/file"}},"jsonrpc":"2.0","id":2}
    ```
    and the hover text is sent as a `HoverResponse`, such as
    ```json
    {"jsonrpc":"2.0","id":2,"result":{"contents":"Parent: 5"}}
    ```


### Running 

Because NeoVim has native support for LSP servers, to run this LSP server:
```lua
vim.api.nvim_create_autocmd("FileType", {
    pattern = "abc", -- Pattern of the file extension to run on
    callback = function()
        local client = vim.lsp.start_client {
            name = "lsp-server-test",
            cmd = { "file:///path/to/lsp_executable"} -- filepath of the cargo executable
        }

        if not client then
            vim.notify "didnt succeed"
            return
        end
        local b = vim.lsp.buf_attach_client(0, client)
        vim.notify("LSP Status: " .. tostring(b))
    end
})
```

## Known Issues/Limitations

- The current implementation is only tested on Neovim version 0.9.1, some other editors such as Emacs and VSCode are not tested.
- There are some performance issues with how the `main` function is waiting for inputs from the client:
    ```rust
    while let Ok(n) = handle.read(&mut buff) {
        ...
    }
    ```
    This could be fixed with async read.
- I wanted for the LSP server to push diagnostics if the file doesn't represent a valid complete binary search tree (wrong number of nodes in some depth, no space between two nodes).
- Currently the LSP server requests that the client and the server syncs with option `TextDocumentSync.Full`, which resends the whole file everytime. I wanted to test out `TextDocumentSync.Incremental`, which only sends the diff.

### Possible improvements

- Finish writing the VSCode extension to let people test it out. A sample VSCode extension code:
```typescript
import * as vscode from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind,
	Trace
  } from 'vscode-languageclient/node';

export async function activate(context: vscode.ExtensionContext) {

    let serverExe = "file:///path/to/lsp_executable";

	const serverOptions: ServerOptions = {
		command:serverExe, 
	  };
	  
	  const clientOptions: LanguageClientOptions = {
		documentSelector: [
            {
                pattern: '**/*.abc',
            }
		],
	  };

    let lspClient = new LanguageClient("LSP-server test", serverOptions, clientOptions);

	lspClient.setTrace(Trace.Verbose);

	console.log('Starting LSP Server');
	await lspClient.start();
	console.log('Congratulations, your extension "ext-lsp" is now active!');
}
```
- Make the writer and reader generalizable (instead of simple stdin and stdout) to allow communication over other channels (for example using TCP, which is quite common for LSP).


