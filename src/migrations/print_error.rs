use std::default::Default;
use std::fs;
use std::path::Path;
use std::str;

use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::emit;
use termcolor::{ColorChoice, StandardStream};

use edgedb_protocol::error_response::ErrorResponse;
use edgedb_protocol::error_response::FIELD_POSITION_END;
use edgedb_protocol::error_response::FIELD_POSITION_START;
use edgedb_protocol::error_response::FIELD_SERVER_TRACEBACK;
use edgedb_protocol::error_response::{FIELD_DETAILS, FIELD_HINT};
use edgeql_parser::tokenizer::TokenStream;

use crate::migrations::create::SourceName;
use crate::migrations::source_map::SourceMap;

fn end_of_last_token(data: &str) -> Option<u64> {
    let mut tokenizer = TokenStream::new(data);
    let mut off = 0;
    for tok in &mut tokenizer {
        off = tok.ok()?.end.offset;
    }
    Some(off)
}

fn get_error_info<'x>(
    err: &ErrorResponse,
    source_map: &'x SourceMap<SourceName>,
) -> Option<(&'x Path, String, usize, usize, bool)> {
    let pstart = err
        .attributes
        .get(&FIELD_POSITION_START)
        .and_then(|x| str::from_utf8(x).ok())
        .and_then(|x| x.parse::<u32>().ok())? as usize;
    let pend = err
        .attributes
        .get(&FIELD_POSITION_END)
        .and_then(|x| str::from_utf8(x).ok())
        .and_then(|x| x.parse::<u32>().ok())? as usize;
    let (src, offset) = source_map.translate_range(pstart, pend).ok()?;
    let res = match src {
        SourceName::File(path) => {
            let data = fs::read_to_string(&path).ok()?;
            (path.as_ref(), data, pstart - offset, pend - offset, false)
        }
        SourceName::Semicolon(path) => {
            let data = fs::read_to_string(&path).ok()?;
            let tok_offset = end_of_last_token(&data)? as usize;
            (path.as_ref(), data, tok_offset, tok_offset, true)
        }
        _ => return None,
    };
    Some(res)
}

pub fn print_migration_error(
    err: &ErrorResponse,
    source_map: &SourceMap<SourceName>,
) -> Result<(), anyhow::Error> {
    let (file_name, data, pstart, pend, eof) = match get_error_info(err, source_map) {
        Some(pair) => pair,
        None => {
            eprintln!("{}", err.display(false));
            return Ok(());
        }
    };

    let message = if eof {
        "Unexpected end of file"
    } else {
        &err.message
    };
    let hint = err
        .attributes
        .get(&FIELD_HINT)
        .and_then(|x| str::from_utf8(x).ok())
        .unwrap_or("error");
    let detail = err
        .attributes
        .get(&FIELD_DETAILS)
        .and_then(|x| String::from_utf8(x.to_vec()).ok());
    let file_name_display = file_name.display();
    let files = SimpleFile::new(&file_name_display, data);
    let diag = Diagnostic::error()
        .with_message(message)
        .with_labels(vec![Label {
            file_id: (),
            style: LabelStyle::Primary,
            range: pstart..pend,
            message: hint.into(),
        }])
        .with_notes(detail.into_iter().collect());

    emit(
        &mut StandardStream::stderr(ColorChoice::Auto),
        &Default::default(),
        &files,
        &diag,
    )?;

    if err.code == 0x_01_00_00_00 {
        let tb = err.attributes.get(&FIELD_SERVER_TRACEBACK);
        if let Some(traceback) = tb {
            if let Ok(traceback) = str::from_utf8(traceback) {
                eprintln!("  Server traceback:");
                for line in traceback.lines() {
                    eprintln!("      {}", line);
                }
            }
        }
    }
    Ok(())
}
