#[cfg(test)]
mod buffer_reader {
    use std::process::ExitCode;

    use crate::rpc::BufferedReader;

    #[test]
    fn test_buffer_reader() -> ExitCode {
        let mut buff_reader = BufferedReader::new();
        buff_reader.write("Content-Length: 15\r\n\r\n{\"method\":\"hi\"}".as_bytes());
        let res = buff_reader.pop_message();
        match res {
            Err(e) => {
                println!("\texpected parse successful, instead got{}", e.to_string());
                return ExitCode::FAILURE;
            }
            Ok(Some(content)) => {
                assert_eq!(content, "{\"method\":\"hi\"}");
                return ExitCode::SUCCESS;
            }
            Ok(None) => {
                println!("\texpected parse successful, instead got None");
                return ExitCode::FAILURE;
            }
        }
    }

    #[test]
    fn test_buffer_reader_none() -> ExitCode {
        let mut buff_reader = BufferedReader::new();
        buff_reader.write("Content-Length: 18\r\n\r\n{\"method\":\"hi\"}".as_bytes());
        let res = buff_reader.pop_message();
        match res {
            Err(_e) => {
                return ExitCode::FAILURE;
            }
            Ok(Some(_content)) => {
                println!("\texpected parse unsuccessful, instead got some");
                return ExitCode::FAILURE;
            }
            Ok(None) => {
                return ExitCode::SUCCESS;
            }
        }
    }

    #[test]
    fn test_buffer_reader_err() -> ExitCode {
        let mut buff_reader = BufferedReader::new();
        buff_reader.write("ABC \r\n\r\n".as_bytes());
        let res = buff_reader.pop_message();
        match res {
            Err(_e) => {
                return ExitCode::SUCCESS;
            }
            Ok(Some(_content)) => {
                return ExitCode::FAILURE;
            }
            Ok(None) => {
                return ExitCode::FAILURE;
            }
        }
    }
}

#[cfg(test)]
mod states {
    use crate::editor::FileState;

    #[test]
    fn test_filestate() {
        let filestate = FileState::new("A\nB C\nD".to_string()).unwrap();
        let n0 = String::from(filestate.get(0).unwrap());
        let n1 = String::from(filestate.get(1).unwrap());
        let n2 = String::from(filestate.get(2).unwrap());
        let n3 = String::from(filestate.left_child(1).unwrap());
        assert_eq!(n0, String::from("A"));
        assert_eq!(n1, String::from("B"));
        assert_eq!(n2, String::from("C"));
        assert_eq!(n3, String::from("D"));
    }
}
