pub mod editor {
    use std::collections::HashMap;

    pub struct FileState {
        tree: Vec<String>,
        char_count: usize,
    }

    pub struct EditorState {
        files: HashMap<String, FileState>,
    }

    impl FileState {
        pub fn new(file_content: String) -> Option<Self> {
            let mut v = Vec::new();

            let lines: Vec<&str> = file_content.lines().collect();
            let line_count = lines.len();
            for (d, line) in lines.iter().enumerate() {
                let n = usize::pow(2, d as u32 + 1) - 1;
                if (d != line_count - 1 && line.len() != n)
                    || (d == line_count - 1 && line.len() > n)
                {
                    return None;
                }
                for c in line.chars().skip(1).step_by(2) {
                    if c != ' ' {
                        return None;
                    }
                }
                for c in line.chars().step_by(2) {
                    v.push(c.to_string());
                }
            }
            return Some(FileState {
                tree: v,
                char_count: file_content.len(),
            });
        }

        pub fn get_char_count(&self) -> usize {
            self.char_count
        }

        pub fn get(&self, index: usize) -> Option<&String> {
            self.tree.get(index)
        }

        pub fn left_child(&self, index: usize) -> Option<&String> {
            self.tree.get(2 * index + 1)
        }

        pub fn right_child(&self, index: usize) -> Option<&String> {
            self.tree.get(2 * index + 2)
        }

        pub fn parent(&self, index: usize) -> Option<&String> {
            match index {
                0 => None,
                _ => self.tree.get((index - 1) / 2),
            }
        }
    }

    impl EditorState {
        pub fn new() -> Self {
            EditorState {
                files: HashMap::new(),
            }
        }

        pub fn modify_file(&mut self, file_name: String, file_content: String) -> bool {
            let new_file_state = FileState::new(file_content);
            match new_file_state {
                Some(fs) => {
                    self.files.insert(file_name, fs);
                    true
                }
                None => false,
            }
        }

        pub fn get_file_state(&self, file_name: String) -> Option<&FileState> {
            self.files.get(&file_name)
        }
    }
}

pub mod rpc {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::fmt;
    use std::fmt::{Display, Formatter};

    pub fn json_to_string<T>(json: &T) -> String
    where
        T: Serialize,
    {
        serde_json::to_string(json).unwrap()
    }

    pub fn json_from_string<'a, T>(s: &String) -> Result<T, serde_json::Error>
    where
        T: DeserializeOwned,
    {
        serde_json::from_str(s)
    }

    /// Given the content of the message (json), encode it using LSP protocol such that it is ready to send
    pub fn encode_message(message: String) -> String {
        let n = message.len();
        let mut encoded_message = format!("Content-Length: {}\r\n\r\n", n);
        encoded_message.push_str(&message);
        return encoded_message;
    }

    /// Extract the content specified in the [LSP/LSIF Docs](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#contentPart).
    /// Pop the whole LSP message from the buffer and return the content part as String.
    /// If Buffer has not finished filling, header length + 4 + content length > buffer size, return None
    /// If message doesn't start with `Content-Length: <content length>`, return Err
    /// Returns the parsed message, with the total message length (including 'Content-Length: ..')
    pub fn decode_message(message: &String) -> Result<Option<(String, usize)>, MsgParseError> {
        let Some((header, content)) = message.split_once("\r\n\r\n") else {
            return Err(MsgParseError(
                "Invalid format, contains no \\r\\n\\r\\n".to_string(),
            ));
        };
        if !header.starts_with("Content-Length: ") {
            return Err(MsgParseError(String::from(
                "Expected header starting with Content-Length",
            )));
        }
        let content_length_str = header.trim_start_matches("Content-Length: ");
        let Ok(content_length): Result<usize, _> = content_length_str.parse() else {
            return Err(MsgParseError(String::from(
                "Could not parse content length to number",
            )));
        };

        if content_length > content.len() {
            Ok(None)
        } else {
            let total_length = header.len() + 4 + content.len();
            let content = String::from(&content[..content_length]);
            Ok(Some((content, total_length)))
        }
    }

    pub struct BufferedReader {
        data: String,
    }

    /// BufferedReader buffers all the recieved content
    impl BufferedReader {
        pub fn new() -> BufferedReader {
            BufferedReader {
                data: String::new(),
            }
        }

        /// Write buffer of bytes to BufferReader::data
        pub fn write(&mut self, buffer: &[u8]) {
            self.data.push_str(&String::from_utf8_lossy(buffer));
        }

        /// Get data from current buffer
        pub fn get_data(&self) -> &String {
            &self.data
        }

        /// Parse the lsp message, and if buffer contains valid lsp message, pop it from the data
        pub fn pop_message(&mut self) -> Result<Option<String>, MsgParseError> {
            match decode_message(&self.data) {
                Ok(Some((content, total_len))) => {
                    self.data = self.data.chars().skip(total_len).collect();
                    Ok(Some(content))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e),
            }
        }
    }

    /// Given the content of the message, return the corresponding object
    pub fn message_to_object<T>(message: &String) -> Result<T, MsgParseError>
    where
        T: DeserializeOwned,
    {
        match json_from_string(message) {
            Ok(msg) => Ok(msg),
            Err(e) => Err(MsgParseError(e.to_string())),
        }
    }

    #[derive(Debug, Clone)]
    pub struct MsgParseError(pub String);
    impl Display for MsgParseError {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            self.0.fmt(f)
        }
    }
}

pub mod lsp {
    use serde::{Deserialize, Serialize};
    use std::io::{self, Write};

    use crate::{
        editor::EditorState,
        rpc::{encode_message, json_from_string, json_to_string, message_to_object, MsgParseError},
    };

    /// Given an arbitrary message (with method field), handle the message accordingly
    /// If initialize request, send the initialize response
    /// If didOpen or didChange, sync the editor_state
    /// If hover request, resond with hover response
    /// Writing debugging information to the logger is optional
    pub fn handle_message(
        message: String,
        editor_state: &mut EditorState,
        logger: &mut impl Write,
    ) -> Result<(), MsgParseError> {
        let method = match message_to_object::<Notification>(&message) {
            Ok(msg) => msg.method,
            Err(e) => return Err(MsgParseError(e.to_string())),
        };
        writeln!(logger, "[Method] {}", method).unwrap();
        writeln!(logger, "[Content] {}", message).unwrap();
        match method.as_str() {
            "initialize" => match json_from_string::<InitializeRequest>(&message) {
                Ok(msg) => {
                    writeln!(
                        logger,
                        "[Initialize] Recieved from {:?} with id {}",
                        msg.params.client_info, msg.request.id
                    )
                    .unwrap();
                    let response = InitializeResponse::new(
                        msg.request.id,
                        "LSP-Server".to_string(),
                        "0".to_string(),
                    );
                    let response_str = json_to_string(&response);
                    let encoded_response = encode_message(response_str);
                    writeln!(logger, "[Sent Response] {:?}", encoded_response).unwrap();

                    io::stdout().write(encoded_response.as_bytes()).unwrap();
                    io::stdout().flush().unwrap();
                    Ok(())
                }
                Err(e) => Err(MsgParseError(format!(
                    "Could not parse InitializeRequest, error {}",
                    e.to_string()
                ))),
            },
            "textDocument/didOpen" => {
                match json_from_string::<DidOpenTextDocumentNotification>(&message) {
                    Ok(msg) => {
                        writeln!(
                            logger,
                            "[Initialize] Recieved didOpen on file {} with version {}",
                            msg.params.text_document.uri, msg.params.text_document.version
                        )
                        .unwrap();
                        let modify_success = editor_state.modify_file(
                            msg.params.text_document.uri.clone(),
                            msg.params.text_document.text.clone(),
                        );
                        if !modify_success {
                            writeln!(
                                logger,
                                "[Error] open {} file with text {:?} not successful",
                                msg.params.text_document.uri, msg.params.text_document.text
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                logger,
                                "[DidOpen] open {} file with text {:?} successful",
                                msg.params.text_document.uri, msg.params.text_document.text
                            )
                            .unwrap();
                        }
                        Ok(())
                    }
                    Err(e) => Err(MsgParseError(format!(
                        "Could not parse DidOpenNotification, error {}",
                        e.to_string()
                    ))),
                }
            }
            "textDocument/didChange" => {
                match json_from_string::<TextDocumentDidChangeNotification>(&message) {
                    Ok(msg) => {
                        writeln!(
                            logger,
                            "[DidChange] Recieved didChange on file {} with version {}",
                            msg.params.text_document.uri, msg.params.text_document.version
                        )
                        .unwrap();
                        let mut modify_success = true;
                        for change in msg.params.content_changes {
                            modify_success &= editor_state.modify_file(
                                msg.params.text_document.uri.clone(),
                                change.text.clone(),
                            );
                        }
                        if !modify_success {
                            writeln!(
                                logger,
                                "[Error] modify {} file with text not successful",
                                msg.params.text_document.uri
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                logger,
                                "[DidChange] modify {} file successful",
                                msg.params.text_document.uri
                            )
                            .unwrap();
                        }
                        Ok(())
                    }
                    Err(e) => Err(MsgParseError(format!(
                        "[Err] Could not parse DidOpenNotification, error {}",
                        e.to_string()
                    ))),
                }
            }
            "textDocument/hover" => match json_from_string::<HoverRequest>(&message) {
                Ok(msg) => {
                    writeln!(
                        logger,
                        "[HoverRequest] Recieved from {:?}",
                        msg.params.pos_params.text_document.uri
                    )
                    .unwrap();

                    let Some(fs) = editor_state
                        .get_file_state(msg.params.pos_params.text_document.uri.clone())
                    else {
                        return Err(MsgParseError(format!(
                            "Could not find file {}",
                            msg.params.pos_params.text_document.uri
                        )));
                    };

                    let line_num = msg.params.pos_params.position.line as u32;
                    let char_num = msg.params.pos_params.position.character as usize;
                    let n = usize::pow(2, line_num) - 1;
                    let index = n + char_num / 2;
                    let hover_rsp_msg = if char_num % 2 != 0 {
                        format!("Character count: {}", fs.get_char_count())
                    } else {
                        if let Some(c) = fs.parent(index) {
                            format!("Parent: {}", c)
                        } else {
                            format!("Could not find parent to {} {}", index, (index - 1) / 2)
                        }
                    };

                    let response = HoverResponse::new(msg.request.id, hover_rsp_msg);
                    let response_str = json_to_string(&response);
                    let encoded_response = encode_message(response_str);
                    writeln!(logger, "[Sent Response] {:?}", encoded_response).unwrap();

                    io::stdout().write(encoded_response.as_bytes()).unwrap();
                    io::stdout().flush().unwrap();
                    Ok(())
                }
                Err(e) => Err(MsgParseError(format!(
                    "Could not parse HoverRequest, error {}",
                    e.to_string()
                ))),
            },

            _ => Ok(()),
        }
    }

    // This code defines various structs used for representing messages within the LSP

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Message {
        // The LSP message header specifying the JSON RPC version ("2.0")
        pub jsonrpc: String,
    }

    // Notification messages are sent from the client to the server
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Notification {
        #[serde(flatten)]
        pub message: Message,
        pub method: String, // The specific notification method name (e.g., "textDocument/didOpen")
    }

    // Request messages are sent from the client to the server and expect a response
    #[derive(Debug, Deserialize, Serialize)]
    pub struct RequestMessage {
        #[serde(flatten)]
        pub base_message: Notification, // Contains message header and method
        pub id: i64, // Unique identifier for the request
    }

    // Response messages are sent from the server to the client in response to requests
    #[derive(Debug, Deserialize, Serialize)]
    pub struct ResponseMessage {
        #[serde(flatten)]
        pub message: Message,
        pub id: i64, // The id that matches the original request
    }

    // Initialize request is sent by the client to the server during initialization
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InitializeRequest {
        #[serde(flatten)]
        pub request: RequestMessage, // Contains message header, method, and id
        pub params: InitializeParams, // Specific parameters for initialization
    }

    // Parameters for the InitializeRequest
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeParams {
        pub process_id: i64, // process ID of the client process (different from id)
        pub client_info: Option<Info>, // Optional information about the client
    }

    // Information about the client/server application
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Info {
        pub name: String,
        pub version: String,
    }

    // Initialize response sent by the server after initialization
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeResponse {
        #[serde(flatten)]
        pub response: ResponseMessage,
        pub result: InitializeResult,
    }

    // Result of the initialization process
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeResult {
        pub capabilities: ServerCapabilities, // Capabilities offered by the server
        pub server_info: Info,                // Information about the server
    }

    // Helper function to create an InitializeResponse message
    impl InitializeResponse {
        pub fn new(id: i64, name: String, version: String) -> InitializeResponse {
            InitializeResponse {
                response: ResponseMessage {
                    id,
                    message: Message {
                        jsonrpc: String::from("2.0"),
                    },
                },
                result: InitializeResult {
                    capabilities: ServerCapabilities {
                        text_document_sync: TextDocumentSyncKind::FULL,
                        hover_provider: true,
                    },
                    server_info: Info { name, version },
                },
            }
        }
    }

    // Different TextDocumentSync options (currently only FULL is supported)
    pub struct TextDocumentSyncKind {}

    impl TextDocumentSyncKind {
        const _NONE: usize = 0;
        const FULL: usize = 1;
        const _INCREMENTAL: usize = 2;
    }

    // Description of the server's capabilities
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ServerCapabilities {
        pub text_document_sync: usize, // Type of text document synchronization supported
        pub hover_provider: bool,      // Whether the server can provide hover information
    }

    // Notification sent by the client when a document is opened
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DidOpenTextDocumentNotification {
        #[serde(flatten)]
        pub notification: Notification,
        pub params: DidOpenTextDocumentParams, // Parameters for the notification
    }

    // Parameters for the DidOpenTextDocumentNotification
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DidOpenTextDocumentParams {
        pub text_document: TextDocumentItem,
    }

    // Notification sent by the client when a text document is changed
    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentDidChangeNotification {
        #[serde(flatten)]
        notification: Notification,
        params: DidChangeTextDocumentParams, // Change-specific parameters
    }

    // Parameters for the TextDocumentDidChangeNotification
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DidChangeTextDocumentParams {
        text_document: VersionTextDocumentIdentifier, // Identifier of the changed document
        content_changes: Vec<TextDocumentContentChangeEvent>, // Array of changes made to the document
    }

    // Identifies a text document using a URI and a version
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct VersionTextDocumentIdentifier {
        uri: String,
        version: i32, // Version of the document
    }

    // Describes a change made to a text document
    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentContentChangeEvent {
        text: String, // The new text content of the entire document
    }

    // Represents a text document within the LSP
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TextDocumentItem {
        pub uri: String,
        pub language_id: String, // Identifier of the language used in the document
        pub version: i64,        // Version of the document, usually starts at 0
        pub text: String,        // The text content of the document
    }

    // Request for hover information at a specific text position
    #[derive(Debug, Deserialize, Serialize)]
    struct HoverRequest {
        #[serde(flatten)]
        request: RequestMessage,
        params: HoverParams, // Parameters containing the position for hover
    }

    // Parameters for the HoverRequest
    #[derive(Debug, Deserialize, Serialize)]
    struct HoverParams {
        #[serde(flatten)]
        pos_params: TextDocumentPositionParams, // Position information within a text document
    }

    // Response containing hover information
    #[derive(Debug, Deserialize, Serialize)]
    struct HoverResponse {
        #[serde(flatten)]
        response: ResponseMessage,
        result: HoverResult, // The hover information itself
    }

    // Helper function to create a HoverResponse message
    impl HoverResponse {
        pub fn new(id: i64, response_str: String) -> Self {
            HoverResponse {
                response: ResponseMessage {
                    id,
                    message: Message {
                        jsonrpc: "2.0".to_string(),
                    },
                },
                result: HoverResult {
                    contents: response_str,
                },
            }
        }
    }

    // Structure holding the actual hover information
    #[derive(Debug, Deserialize, Serialize)]
    struct HoverResult {
        contents: String, // Textual content to be displayed in the hover tooltip
    }

    // Parameters used to specify a position within a text document
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TextDocumentPositionParams {
        text_document: TextDocumentIdentifier, // Identifier of the text document
        position: Position,                    // Line and character position
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentIdentifier {
        uri: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct Position {
        line: i32,      // Line number within the text document
        character: i32, // Character offset within the line
    }
}

mod test;
