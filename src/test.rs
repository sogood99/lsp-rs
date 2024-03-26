mod buffer_reader {
    use serde::{Deserialize, Serialize};
    use std::process::ExitCode;

    use crate::{encode_message, BufferedReader};

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
        #[derive(Debug, Deserialize, Serialize)]
        struct EncodingExample {
            testing: bool,
        }

        let encoding_example = EncodingExample { testing: true };
        let expected = "Content-Length: 16\r\n\r\n{\"testing\":true}";
        let actual = encode_message(serde_json::to_string(&encoding_example).unwrap());
        assert_eq!(expected, actual);
    }
}
