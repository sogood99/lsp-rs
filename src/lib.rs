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

    pub struct BufferedReader {
        data: String,
    }

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

        pub fn get_data(&self) -> &String {
            &self.data
        }

        /// Extract the content specified in the [LSP/LSIF Docs](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#contentPart).
        /// Pop the whole LSP message from the buffer and return the content part as String.
        /// If Buffer has not finished filling, header length + 4 + content length > buffer size, return None
        /// If message doesn't start with `Content-Length: <content length>`, return Err
        pub fn pop_message(&mut self) -> Result<Option<String>, MsgParseError> {
            let Some((header, content)) = self.data.split_once("\r\n\r\n") else {
                return Ok(None);
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
                let total_length = header.len() + 4 + content_length;
                let content = String::from(&content[..content_length]);
                self.data = self.data.chars().skip(total_length).collect();
                Ok(Some(content))
            }
        }
    }

    /// Given the content of the message (json), encode it using LSP protocol such that it is ready to send
    pub fn encode_message(message: String) -> String {
        let n = message.len();
        let mut encoded_message = format!("Content-Length: {}\r\n\r\n", n);
        encoded_message.push_str(&message);
        return encoded_message;
    }

    /// Given the content of the message, return the corresponding object
    pub fn decode_message<T>(message: &String) -> Result<T, MsgParseError>
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
        rpc::{decode_message, encode_message, json_from_string, json_to_string, MsgParseError},
    };

    /// Given an arbitrary message (with method field), handle the message accordingly
    /// If initialize request, send the initialize response
    pub fn handle_message(
        message: String,
        editor_state: &mut EditorState,
        logger: &mut impl Write,
    ) -> Result<(), MsgParseError> {
        let method = match decode_message::<Notification>(&message) {
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
                                "[Error] modify {} file with text {:?} not successful",
                                msg.params.text_document.uri, msg.params.text_document.text
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                logger,
                                "[DidOpen] modify {} file with text {:?} successful",
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
                            modify_success |= editor_state.modify_file(
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
                    let response = HoverResponse::new(msg.request.id, "Hello World".to_string());
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

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Message {
        pub jsonrpc: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Notification {
        #[serde(flatten)]
        pub message: Message,
        pub method: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct RequestMessage {
        #[serde(flatten)]
        pub base_message: Notification,
        pub id: i64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct ResponseMessage {
        #[serde(flatten)]
        pub message: Message,
        pub id: i64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct InitializeRequest {
        #[serde(flatten)]
        pub request: RequestMessage,
        pub params: InitializeParams,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeParams {
        pub process_id: i64,
        pub client_info: Option<Info>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Info {
        pub name: String,
        pub version: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeResponse {
        #[serde(flatten)]
        pub response: ResponseMessage,
        pub result: InitializeResult,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InitializeResult {
        pub capabilities: ServerCapabilities,
        pub server_info: Info,
    }

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

    pub struct TextDocumentSyncKind {}
    impl TextDocumentSyncKind {
        // const NONE: usize = 0;
        const FULL: usize = 1;
        // const INCREMENTAL: usize = 2;
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ServerCapabilities {
        pub text_document_sync: usize,
        pub hover_provider: bool,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DidOpenTextDocumentNotification {
        #[serde(flatten)]
        pub notification: Notification,
        pub params: DidOpenTextDocumentParams,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DidOpenTextDocumentParams {
        pub text_document: TextDocumentItem,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentDidChangeNotification {
        #[serde(flatten)]
        notification: Notification,
        params: DidChangeTextDocumentParams,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DidChangeTextDocumentParams {
        text_document: VersionTextDocumentIdentifier,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct VersionTextDocumentIdentifier {
        uri: String,
        version: i32,
    }

    /**
     * An event describing a change to a text document. If only a text is provided
     * it is considered to be the full content of the document.
     */
    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentContentChangeEvent {
        // The new text of the whole document.
        text: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TextDocumentItem {
        pub uri: String,
        pub language_id: String,
        pub version: i64,
        pub text: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HoverRequest {
        #[serde(flatten)]
        request: RequestMessage,
        params: HoverParams,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HoverParams {
        #[serde(flatten)]
        pos_params: TextDocumentPositionParams,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HoverResponse {
        #[serde(flatten)]
        response: ResponseMessage,
        result: HoverResult,
    }

    impl HoverResponse {
        pub fn new(id: i64, response_str: String) -> Self {
            HoverResponse {
                response: ResponseMessage {
                    message: Message {
                        jsonrpc: "2.0".to_string(),
                    },
                    id,
                },
                result: HoverResult {
                    contents: response_str,
                },
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct HoverResult {
        contents: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TextDocumentPositionParams {
        text_document: TextDocumentIdentifier,
        position: Position,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct TextDocumentIdentifier {
        uri: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct Position {
        line: i32,
        character: i32,
    }
}

pub mod editor {
    use std::collections::HashMap;

    pub struct FileState {
        tree: Vec<String>,
    }

    pub struct EditorState {
        files: HashMap<String, FileState>,
    }

    impl FileState {
        pub fn new(file_content: String) -> Option<Self> {
            let v = Vec::new();

            let lines: Vec<&str> = file_content.lines().collect();
            let line_count = lines.len();
            for (d, line) in lines.iter().enumerate() {
                let n = usize::pow(2, d as u32);
                if line.len() != n || (d == line_count - 1 && line.len() > n) {
                    return None;
                }
                for c in line.chars().skip(1).step_by(2) {
                    if c != ' ' {
                        return None;
                    }
                }
            }
            return Some(FileState { tree: v });
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
    }
}

mod test;
