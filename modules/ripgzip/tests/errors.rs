fn check_decompression_error(mut data: &[u8], msg: &'static str) {
    let res = ripgzip::decompress(&mut data, &mut std::io::sink());
    if res.is_ok() {
        panic!("expected Err, got Ok");
    }
    for inner in res.unwrap_err().chain() {
        println!("{}", inner.to_string());
        if inner.to_string().contains(msg) {
            return;
        }
    }
    panic!("error does not contain message: {}", msg);
}

#[cfg(test)]
mod errors {
    use super::*;

    #[test]
    fn idk() {
        check_decompression_error(include_bytes!("../data/ok/10-header-crc16.gz"), "a");
    }

    #[test]
    fn length_check_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/00-bad-length.gz"),
            "length check failed",
        );
    }

    #[test]
    fn crc32_check_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/01-bad-crc32.gz"),
            "crc32 check failed",
        );
    }

    #[test]
    fn unexpected_eof_error() {
        check_decompression_error(include_bytes!("../data/corrupted/02-unexpected-eof.gz"), "");
    }

    #[test]
    fn wrong_id_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/03-wrong-id.gz"),
            "wrong id values",
        );
    }

    #[test]
    fn header_eof_error() {
        check_decompression_error(include_bytes!("../data/corrupted/04-header-eof.gz"), "");
    }

    #[test]
    fn crc16_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/05-bad-header-crc16.gz"),
            "header crc16 check failed",
        );
    }

    #[test]
    fn unsupported_block_type_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/06-invalid-btype.gz"),
            "unsupported block type",
        );
    }

    #[test]
    fn unsupported_compression_method() {
        check_decompression_error(
            include_bytes!("../data/corrupted/07-invalid-cm.gz"),
            "unsupported compression method",
        );
    }

    #[test]
    fn nlen_check_error() {
        check_decompression_error(
            include_bytes!("../data/corrupted/08-bad-nlen.gz"),
            "nlen check failed",
        );
    }
}
