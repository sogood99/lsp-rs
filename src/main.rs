use std::{
    env,
    fs::File,
    io::{self, Read, Write},
};

use lsp_server::{handle_content, BufferedReader};

fn main() {
    let mut buff_reader = BufferedReader::new();
    let args = env::args().collect::<Vec<String>>();
    let mut file = if let Some(filename) = args.get(1) {
        File::create(filename)
    } else {
        File::create("lsp_logger.txt")
    }
    .expect("Failed to create file");
    let mut buff = [0; 512];
    let mut handle = io::stdin().lock();
    while let Ok(n) = handle.read(&mut buff) {
        buff_reader.read(&buff[..n]);
        writeln!(
            &mut file,
            "Read data: {:?}",
            String::from_utf8_lossy(&buff[..n])
        )
        .unwrap();
        let res = buff_reader.decode_message();
        match res {
            Err(e) => write!(&mut file, "{}", e.to_string()).unwrap(),
            Ok(Some((msg, content))) => {
                let _ = handle_content(msg.method, content, &mut file);
            }
            Ok(None) => (),
        }
        buff.fill(0);
    }
}
