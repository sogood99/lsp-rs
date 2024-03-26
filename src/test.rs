mod buffer_reader {
    use serde::{Deserialize, Serialize};
    use std::process::ExitCode;

    #[test]
    fn test001() -> ExitCode {
        // let mut buff_reader = BufferedReader::new();
        // buff_reader.read(String::from(
        //     "Content-Length: 15\r\n\r\n{\"method\":\"hi\"}",
        // ));
        // let res = buff_reader.decode_message();
        // match res {
        //     Err(e) => {
        //         println!("\texpected parse successful, instead got{}", e.to_string());
        //         return ExitCode::FAILURE;
        //     }
        //     Ok(Some((m, _content))) => {
        //         assert!(m.method == "hi");
        //         return ExitCode::SUCCESS;
        //     }
        //     Ok(None) => {
        //         println!("\texpected parse successful, instead got None");
        //         return ExitCode::FAILURE;
        //     }
        // }
        return ExitCode::SUCCESS;
    }

    #[test]
    fn test002() {
        // #[derive(Debug, Deserialize, Serialize)]
        // struct EncodingExample {
        //     testing: bool,
        // }
        //
        // let encoding_example = EncodingExample { testing: true };
        // let expected = "Content-Length: 16\r\n\r\n{\"testing\":true}";
        // let actual = encode_message(serde_json::to_string(&encoding_example).unwrap());
        // assert_eq!(expected, actual);
    }
}

mod states {
    use crate::editor::FileState;

    #[test]
    fn test001() {
        // let filestate = FileState::new("A\nB\tC\nD".to_string());
        // let n0 = String::from(filestate.get(0).unwrap());
        // let n1 = String::from(filestate.get(1).unwrap());
        // let n2 = String::from(filestate.get(2).unwrap());
        // let n3 = String::from(filestate.left_child(1).unwrap());
        // assert_eq!(n0, String::from("A"));
        // assert_eq!(n1, String::from("B"));
        // assert_eq!(n2, String::from("C"));
        // assert_eq!(n3, String::from("D"));
    }
}
