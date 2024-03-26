use std::fmt;
use std::fmt::{Display, Formatter};

use std::fs::File;
use std::io::{self, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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
    pub fn read(&mut self, buffer: &[u8]) {
        // read buffer of bytes to String
        self.data.push_str(&String::from_utf8_lossy(buffer));
    }

    pub fn get_data(&self) -> &String {
        &self.data
    }

    pub fn decode_message(&mut self) -> Result<Option<(RequestMessage, String)>, MsgParseError> {
        //
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
            match json_from_string(&content) {
                Ok(msg) => Ok(Some((msg, content))),
                Err(e) => Err(MsgParseError(e.to_string())),
            }
        }
    }
}

pub fn encode_message(message: String) -> String {
    let n = message.len();
    let mut encoded_message = format!("Content-Length: {}\r\n\r\n", n);
    encoded_message.push_str(&message);
    return encoded_message;
}

pub fn handle_content(
    method: String,
    content: String,
    logger: &mut File,
) -> Result<(), MsgParseError> {
    write!(logger, "Method: {}", method).unwrap();
    match method.as_str() {
        "initialize" => {
            writeln!(logger, "Method: {}", method).unwrap();
            match serde_json::from_str::<InitializeRequest>(&content) {
                Ok(msg) => {
                    writeln!(
                        logger,
                        "Recieved Initialize from {:?} with id {}",
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
                    writeln!(logger, "Sent response: {:?}", encoded_response).unwrap();

                    io::stdout().write(encoded_response.as_bytes()).unwrap();
                    io::stdout().flush().unwrap();
                    Ok(())
                }
                Err(e) => Err(MsgParseError(format!(
                    "Could not parse InitializeRequest, error {}",
                    e.to_string()
                ))),
            }
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone)]
pub struct MsgParseError(String);
impl Display for MsgParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl PartialEq for MsgParseError {
    fn eq(&self, _othr: &MsgParseError) -> bool {
        true
    }
}
impl Eq for MsgParseError {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub jsonrpc: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMessage {
    #[serde(flatten)]
    pub message: Message,
    pub method: String,
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
                    text_document_sync: 1,
                    hover_provider: true,
                    definition_provider: true,
                    code_action_provider: true,
                    completion_provider: CompletionProvider {},
                },
                server_info: Info { name, version },
            },
        }
    }
}

pub struct TextDocumentSyncKind {}
impl TextDocumentSyncKind {
    const NONE: usize = 0;
    const FULL: usize = 1;
    const INCREMENTAL: usize = 2;
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    pub text_document_sync: usize,
    pub hover_provider: bool,
    pub definition_provider: bool,
    pub code_action_provider: bool,
    pub completion_provider: CompletionProvider,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionProvider {}

mod test;
