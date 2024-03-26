use std::{
    env,
    fs::File,
    io::{self, Read, Write},
};

use lsp_server::{editor::EditorState, lsp::handle_message, rpc::BufferedReader};

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let mut logger: Box<dyn Write> = if let Some(filename) = args.get(1) {
        Box::new(File::create(filename).expect("Failed to create logger file"))
    } else {
        Box::new(io::empty())
    };

    let mut buff_reader = BufferedReader::new();
    let mut buff = [0; 512];
    let mut handle = io::stdin().lock();
    let mut editor_state = EditorState::new();
    while let Ok(n) = handle.read(&mut buff) {
        if n == 0 {
            break;
        }
        buff_reader.write(&buff[..n]);
        let res = buff_reader.pop_message();
        match res {
            Ok(Some(content)) => match handle_message(content, &mut editor_state, &mut logger) {
                Ok(()) => (),
                Err(e) => writeln!(
                    &mut logger,
                    "[Error] Error handling message {}",
                    e.to_string()
                )
                .unwrap(),
            },
            Ok(None) => (),
            Err(e) => writeln!(
                &mut logger,
                "[Error] Could not pop message: {}",
                e.to_string()
            )
            .unwrap(),
        }
        buff.fill(0);
    }
}
