//! Headless tests for the line-numbered text/code viewer.
//!
//! Exercises `viewer` without a display.

mod text_viewer {
    use std::io::Write;
    use std::path::Path;

    /// Compile-time check: the viewer module must exist and expose its types.
    #[test]
    fn viewer_module_exists() {
        let _ = astynex::viewer::TextContent::Empty;
        // Verify Lines variant can be named
        fn _check_lines(_: &astynex::viewer::TextContent) {
            match &astynex::viewer::TextContent::Empty {
                astynex::viewer::TextContent::Empty => {}
                astynex::viewer::TextContent::Lines(_) => {}
            }
        }
        _check_lines(&astynex::viewer::TextContent::Empty);
    }

    fn write_temp_file(content: &[u8]) -> tempfile::NamedTempFile {
        let f = tempfile::NamedTempFile::new().unwrap();
        f.as_file().write_all(content).unwrap();
        f
    }

    /// Opening a valid UTF-8 file returns lines with correct line numbers.
    #[test]
    fn open_utf8_file_returns_lines() {
        let tmp = write_temp_file(b"line one\nline two\nline three\n");
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("open_file should succeed for valid UTF-8");
        let lines = match content {
            astynex::viewer::TextContent::Lines(ls) => ls,
            astynex::viewer::TextContent::Empty => panic!("expected lines, got Empty"),
        };

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].number, 1);
        assert_eq!(lines[0].text, "line one");
        assert_eq!(lines[1].number, 2);
        assert_eq!(lines[1].text, "line two");
        assert_eq!(lines[2].number, 3);
        assert_eq!(lines[2].text, "line three");
    }

    /// Empty file returns `Empty` variant.
    #[test]
    fn empty_file_returns_empty() {
        let tmp = write_temp_file(b"");
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("open_file should succeed for empty file");
        assert!(matches!(content, astynex::viewer::TextContent::Empty));
    }

    /// Single line without trailing newline is handled.
    #[test]
    fn single_line_no_newline() {
        let tmp = write_temp_file(b"no newline");
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("open_file should succeed");
        let lines = match content {
            astynex::viewer::TextContent::Lines(ls) => ls,
            astynex::viewer::TextContent::Empty => panic!("expected lines"),
        };

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].number, 1);
        assert_eq!(lines[0].text, "no newline");
    }

    /// Invalid UTF-8 returns a diagnostic error, not a panic.
    #[test]
    fn invalid_utf8_returns_error() {
        // Valid UTF-8 prefix followed by invalid byte sequence (overlong encoding)
        let invalid_utf8 = b"hello\xff\xfe bye";
        let tmp = write_temp_file(invalid_utf8);
        let result = astynex::viewer::open_file(tmp.path());

        // Must return an error, not panic
        assert!(
            result.is_err(),
            "invalid UTF-8 must return error, not panic"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("utf") || err.contains("Utf") || err.contains("UTF"),
            "error message must mention UTF: {err}"
        );
    }

    /// I/O error (missing file) returns a diagnostic error, not a panic.
    #[test]
    fn missing_file_returns_error() {
        let result =
            astynex::viewer::open_file(Path::new("/nonexistent/this/file/does/not/exist.txt"));
        assert!(result.is_err(), "missing file must return error, not panic");
        let err = result.unwrap_err();
        // Should describe the failure clearly
        assert!(!err.is_empty(), "error message must not be empty");
    }

    /// File larger than 1 MiB opens but skips structural parse with diagnostic.
    #[test]
    fn large_file_opens_with_diagnostic() {
        // Create a file larger than 1 MiB (1_048_576 bytes)
        let large_content = vec![b'x'; 1_100_000];
        let tmp = write_temp_file(&large_content);
        let result = astynex::viewer::open_file(tmp.path());

        // Must open (not reject) but signal structural parse unavailable
        let content = result.expect("large file must open, not reject");
        match content {
            astynex::viewer::TextContent::Lines(lines) => {
                // Content is visible, lines exist
                assert!(!lines.is_empty(), "large file lines must not be empty");
                // The diagnostic is accessible through the viewer API
                let diag = astynex::viewer::large_file_diagnostic();
                assert!(
                    diag.contains("large")
                        || diag.contains("Large")
                        || diag.contains("structural")
                        || diag.contains("Structural"),
                    "diagnostic must mention large/structural: {diag}"
                );
            }
            astynex::viewer::TextContent::Empty => panic!("large file should not be empty"),
        }
    }

    /// Very long single line is handled without crashing.
    #[test]
    fn very_long_line_handled() {
        let long_line = vec![b'y'; 500_000];
        let tmp = write_temp_file(&long_line);
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("very long line must open");
        match content {
            astynex::viewer::TextContent::Lines(ls) => {
                assert_eq!(ls.len(), 1);
                assert_eq!(ls[0].number, 1);
                // Text is present
                assert!(!ls[0].text.is_empty());
            }
            astynex::viewer::TextContent::Empty => panic!("long line should not be empty"),
        }
    }

    /// Line numbers are 1-indexed.
    #[test]
    fn line_numbers_are_1_indexed() {
        let tmp = write_temp_file(b"a\nb\nc\nd\ne\n");
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("open_file should succeed");
        let lines = match content {
            astynex::viewer::TextContent::Lines(ls) => ls,
            astynex::viewer::TextContent::Empty => panic!("expected lines"),
        };

        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.number, (i + 1) as u32, "line {} must be 1-indexed", i);
        }
    }

    /// Trailing whitespace in lines is preserved.
    #[test]
    fn trailing_whitespace_preserved() {
        let content_with_trailing = b"no trailing\nwith trailing   \nend spaces  \n";
        let tmp = write_temp_file(content_with_trailing);
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("open_file should succeed");
        let lines = match content {
            astynex::viewer::TextContent::Lines(ls) => ls,
            astynex::viewer::TextContent::Empty => panic!("expected lines"),
        };

        assert_eq!(lines[0].text, "no trailing");
        assert_eq!(lines[1].text, "with trailing   ");
        assert_eq!(lines[2].text, "end spaces  ");
    }

    /// File with CRLF line endings is normalized to LF.
    #[test]
    fn crlf_normalized() {
        let crlf_content = b"line one\r\nline two\r\n";
        let tmp = write_temp_file(crlf_content);
        let result = astynex::viewer::open_file(tmp.path());

        let content = result.expect("CRLF file should open");
        let lines = match content {
            astynex::viewer::TextContent::Lines(ls) => ls,
            astynex::viewer::TextContent::Empty => panic!("expected lines"),
        };

        assert_eq!(lines.len(), 2);
        assert!(
            !lines[0].text.contains('\r'),
            "CR should be normalized: {:?}",
            lines[0].text
        );
        assert!(
            !lines[1].text.contains('\r'),
            "CR should be normalized: {:?}",
            lines[1].text
        );
    }

    /// Diagnostic for large files is exposed as a non-empty string.
    #[test]
    fn large_file_diagnostic_is_non_empty() {
        let diag = astynex::viewer::large_file_diagnostic();
        assert!(!diag.is_empty(), "large file diagnostic must be non-empty");
        assert!(diag.len() >= 10, "diagnostic must have meaningful content");
    }

    /// Error message for invalid UTF-8 is non-empty.
    #[test]
    fn invalid_utf8_error_message_non_empty() {
        let invalid_utf8 = b"test\xfe\xff";
        let tmp = write_temp_file(invalid_utf8);
        let result = astynex::viewer::open_file(tmp.path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "error message must be non-empty");
        assert!(err.len() >= 5, "error message must be meaningful: {err}");
    }
}
