//! See the [`Parser`] type.

use std::collections::HashMap;
use std::convert::Infallible;
use std::io::{self, Read};
use std::str::{self, FromStr};
use std::time::{Duration, SystemTime};

use log::Level;

/// Create a new [`Parser`].
pub fn parse<R>(reader: R) -> Parser<R>
where
    R: Read,
{
    Parser {
        parsed: 0,
        reader,
        buf: Vec::with_capacity(4096),
        needs_read: true,
        hit_eof: false,
    }
}

/// A struct to parse logfmt formatted logs.
///
/// See the example below for usage.
///
/// # Notes
///
/// The parser assumses the log lines are mostly correct. This means it will
/// loosely check values but isn't too strict about it.
///
/// If this parser returns an [error] it will skip the problematic line and
/// continue with the next one. Note however that if a problem exists in
/// multi-line log message the records returned after might be invalid.
///
/// [error]: ParseError
///
/// # Examples
///
/// The API is simple, just call [`parse`] in a for loop.
///
/// ```
/// use std_logger_parser::parse;
///
/// # fn main() -> Result<(), std_logger_parser::ParseError> {
/// let logs = /* Open some log file, anything that implements `io::Read`. */
/// #    b"" as &[u8];
///
/// for record in parse(logs) {
///     let record = record?;
///
///     eprintln!("parsed a record: {:?}", record);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Parser<R> {
    reader: R,
    /// Amount of bytes parsed from the start of `buf`.
    parsed: usize,
    buf: Vec<u8>,
    /// If `true` `next` will read from `R` into `buf`.
    needs_read: bool,
    /// If `fale` `parse_line` will not return `None` if it hits the end of the
    /// item. Once its `false` `next` will return `None` and `parse_line` will
    /// return the remainder of the record (if any).
    hit_eof: bool,
}

impl<R: Read> Parser<R> {
    fn fill_buf(&mut self) -> io::Result<()> {
        // Remove already processed bytes.
        drop(self.buf.drain(..self.parsed));
        self.parsed = 0;

        // If a log message is the same size as the buffer's capacity double the
        // capacity to read more bytes.
        if self.buf.len() == self.buf.capacity() {
            self.buf.reserve(self.buf.capacity());
        }

        // Resize the buffer to read into the unused space.
        let original_len = self.buf.len();
        self.buf.resize(self.buf.capacity(), 0);
        match self.reader.read(&mut self.buf[original_len..]) {
            Ok(n) => {
                self.buf.truncate(original_len + n);
                if n == 0 {
                    self.hit_eof = true;
                }
                Ok(())
            }
            Err(err) => {
                self.buf.truncate(original_len);
                Err(err)
            }
        }
    }

    /// Returns `None` the log message is incomplete.
    fn parse_line(&mut self) -> Result<Option<Record>, ParseError> {
        let mut record = Record::empty();
        let mut record_is_empty = true;
        let mut input = &self.buf[self.parsed..];

        loop {
            input = eat_space(input);
            if input.is_empty() || input[0] == b'\n' {
                // Mark the line (new line included) as parser.
                self.parsed = (self.buf.len() - input.len()) + if input.is_empty() { 0 } else { 1 };

                if record_is_empty {
                    return Ok(None);
                } else {
                    return Ok(Some(record));
                }
            }

            let (i, key) = parse_key(input).map_err(|err| self.create_line_error(err))?;
            if i.is_empty() {
                return Ok(None);
            }
            input = i;

            let (i, value) = parse_value(input);
            if i.is_empty() && !self.hit_eof {
                // If this is the end of the input we expect it to be the end of
                // the value as well and we don't return here.
                return Ok(None);
            }
            input = i;

            match key {
                "ts" => {
                    let timestamp =
                        parse_timestamp(value).map_err(|err| self.create_line_error(err))?;
                    record.timestamp = Some(timestamp);
                }
                "lvl" => {
                    let level =
                        parse_log_level(value).map_err(|err| self.create_line_error(err))?;
                    record.level = level;
                }
                "msg" => {
                    let msg = parse_string(value).map_err(|err| self.create_line_error(err))?;
                    record.msg = msg.to_owned();
                }
                "target" => {
                    let target = parse_string(value).map_err(|err| self.create_line_error(err))?;
                    record.target = target.to_owned();
                }
                "module" => {
                    let module = parse_string(value).map_err(|err| self.create_line_error(err))?;
                    record.module = Some(module.to_owned());
                }
                "file" => {
                    let (file, line) =
                        parse_file(value).map_err(|err| self.create_line_error(err))?;
                    record.file = Some((file.to_owned(), line));
                }
                _ => {
                    let value = parse_string(value).map_err(|err| self.create_line_error(err))?;
                    // Safety: `FromStr` for `Value` never fails.
                    // TODO: what to do when overwriting a key?
                    let _ = record
                        .key_values
                        .insert(key.to_owned(), value.parse().unwrap());
                }
            }
            // If we get to here we've assigned at least a single field so we
            // want to keep the record.
            record_is_empty = false;
        }
    }

    fn create_line_error(&self, kind: ParseErrorKind) -> ParseError {
        let line = single_line(&self.buf[self.parsed..])
            .to_owned()
            .into_boxed_slice();
        ParseError {
            line: Some(line),
            kind,
            _private: (),
        }
    }
}

impl<R: Read> Iterator for Parser<R> {
    type Item = Result<Record, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.needs_read {
                match self.fill_buf() {
                    Ok(()) => { /* Continue below. */ }
                    Err(err) => {
                        return Some(Err(ParseError {
                            line: None,
                            kind: ParseErrorKind::Io(err),
                            _private: (),
                        }));
                    }
                }
            }

            match self.parse_line() {
                Ok(Some(record)) => return Some(Ok(record)),
                Ok(None) if self.hit_eof => return None,
                Ok(None) => {
                    self.needs_read = true;
                    continue; // Read again.
                }
                Err(err) => {
                    // Skip the troublesome line.
                    self.parsed += err.line.as_ref().map_or(0, |line| line.len());
                    return Some(Err(err));
                }
            }
        }
    }
}

/// Result returned by parsing functions.
type ParseResult<'a, T> = Result<(&'a [u8], T), ParseErrorKind>;

/// Error returned by the [`Parser`].
#[derive(Debug)]
pub struct ParseError {
    /// The line in which the error occurred. This will be `None` for [I/O]
    /// errors.
    ///
    /// [I/O]: ParseErrorKind::Io
    pub line: Option<Box<[u8]>>,
    /// Error detail.
    pub kind: ParseErrorKind,
    /// The creation of the struct is private for future extension.
    _private: (),
}

/// Error detail for [`ParseError`].
#[derive(Debug)]
pub enum ParseErrorKind {
    /// Key contains invalid UTF-8.
    KeyInvalidUt8,
    /// Timestamp (key `ts`) is invalid.
    InvalidTimestamp,
    /// Log level (key `lvl`) is not valid.
    InvalidLevel,
    /// File and line number from where the message oriented (key `file`) is
    /// invalid.
    InvalidFile,
    /// A value contains invalid UTF-8.
    InvalidValue,
    /// I/O error.
    Io(io::Error),
}

/// Returns a single line.
// FIXME: handle new lines inside qoutes.
fn single_line<'a>(input: &'a [u8]) -> &'a [u8] {
    let mut i = 0;
    for b in input.iter().rev().copied() {
        if b != b'\n' {
            break;
        }
        i += 1;
    }
    &input[..input.len() - i]
}

/// Removes all spaces and tabs at the start of `input`. It does not remove new
/// lines.
fn eat_space<'a>(input: &'a [u8]) -> &'a [u8] {
    let mut i = 0;
    for b in input.iter().copied() {
        if b != b' ' && b != b'\t' {
            break;
        }
        i += 1;
    }
    &input[i..]
}

/// Same as [`eat_space`], but removes from the start of the input.
fn eat_space_end<'a>(input: &'a [u8]) -> &'a [u8] {
    let mut i = 0;
    for b in input.iter().rev().copied() {
        if b != b' ' && b != b'\t' {
            break;
        }
        i += 1;
    }
    &input[..input.len() - i]
}

/// Parses a key, i.e. `key=`.
fn parse_key<'a>(input: &'a [u8]) -> ParseResult<'a, &'a str> {
    let mut i = 0;
    for b in input.iter().copied() {
        if b == b'=' {
            break;
        }
        i += 1;
    }
    let (key_bytes, input) = input.split_at(i);
    let input = &input[1..]; // Remove the `=`.
    let key_bytes = eat_space_end(key_bytes);
    match str::from_utf8(key_bytes) {
        Ok(key) => Ok((input, key)),
        Err(_) => Err(ParseErrorKind::KeyInvalidUt8),
    }
}

/// Parse a timestamp with the format: `yyyy-mm-ddThh:mm:ss.nnnnnnZ`, e.g.
/// `2021-02-23T13:15:48.624447Z`.
fn parse_timestamp<'a>(value: &'a [u8]) -> Result<SystemTime, ParseErrorKind> {
    // Invalid length or format.
    if value.len() != 27
        || value[4] != b'-'
        || value[7] != b'-'
        || value[10] != b'T'
        || value[13] != b':'
        || value[16] != b':'
        || value[19] != b'.'
        || value[26] != b'Z'
    {
        return Err(ParseErrorKind::InvalidTimestamp);
    }
    let value = match str::from_utf8(value) {
        Ok(value) => value,
        Err(_) => return Err(ParseErrorKind::InvalidTimestamp),
    };

    #[rustfmt::skip] // Rustfmt makes it 3 lines, it's fits on a single one just fine.
    let year: i32 = value[0..4].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let month: i32 = value[5..7].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let day: i32 = value[8..10].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let hour: i32 = value[11..13].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let min: i32 = value[14..16].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let sec: i32 = value[17..19].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;
    #[rustfmt::skip]
    let nanos: u32 = value[20..26].parse().map_err(|_| ParseErrorKind::InvalidTimestamp)?;

    // Convert the timestamp into the number of seconds sinch Unix Epoch.
    let mut tm = libc::tm {
        tm_sec: sec,
        tm_min: min,
        tm_hour: hour,
        tm_mday: day,
        tm_mon: month - 1,
        tm_year: year - 1900,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null_mut(),
    };
    let time_offset = unsafe { libc::timegm(&mut tm) };
    // Create the timestamp from the time offset and the nanosecond precision.
    Ok(SystemTime::UNIX_EPOCH + Duration::new(time_offset as u64, nanos))
}

/// Parse a log level, using [`Level::from_str`].
fn parse_log_level<'a>(value: &'a [u8]) -> Result<Level, ParseErrorKind> {
    match str::from_utf8(value) {
        Ok(value) => match value.parse() {
            Ok(level) => Ok(level),
            Err(_) => Err(ParseErrorKind::InvalidLevel),
        },
        Err(_) => Err(ParseErrorKind::InvalidLevel),
    }
}

fn parse_string<'a>(value: &'a [u8]) -> Result<&'a str, ParseErrorKind> {
    match str::from_utf8(value) {
        Ok(value) => Ok(value),
        Err(_) => Err(ParseErrorKind::InvalidValue),
    }
}

/// Parse file value, format: `path/to/file:column`, e.g.
/// `examples/simple.rs:51`.
fn parse_file<'a>(value: &'a [u8]) -> Result<(&'a str, u32), ParseErrorKind> {
    match str::from_utf8(value) {
        Ok(value) => {
            if let Some((file, column)) = value.rsplit_once(':') {
                match column.parse() {
                    Ok(column) => Ok((file, column)),
                    Err(_) => Err(ParseErrorKind::InvalidFile),
                }
            } else {
                Err(ParseErrorKind::InvalidFile)
            }
        }
        Err(_) => Err(ParseErrorKind::InvalidFile),
    }
}

/// Returns `(remaining_input, value)`.
fn parse_value<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    let input = eat_space(input);
    if input.first().copied() == Some(b'"') {
        parse_qouted_value(input)
    } else {
        parse_naked_value(input)
    }
}

/// See [`parse_value`], expects `input` to contain a qouted value, i.e. it
/// starts and ends with `"`.
fn parse_qouted_value<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    debug_assert!(input[0] == b'"');
    let mut i = 1;
    let mut qoute_count = 1; // Support qoutes inside qoutes.
    let mut bytes = input.iter().skip(1).copied().peekable();
    // FIXME: this doesn't work.
    // Different strategy: search for next `=`, then backtrace from there.
    while let Some(b) = bytes.next() {
        if b == b'"' {
            qoute_count += 1;
            let nb = bytes.peek().copied();
            if nb == Some(b' ') || nb == Some(b'\n') && qoute_count % 2 == 0 {
                break;
            }
        }
        i += 1;
    }

    let value = &input[1..i]; // Skip start qoute.
    let input = if i == input.len() {
        &[]
    } else {
        &input[i + 1..] // Skip end qoute.
    };
    (input, value)
}

/// Parses a single value, expecting a space (` `) as value end.
fn parse_naked_value<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    let mut i = 0;
    for b in input.iter().copied() {
        if b == b' ' {
            break;
        }
        i += 1;
    }
    let value = &input[..i];
    let input = &input[i..];
    (input, value)
}

/// A parser log record.
#[derive(Debug)]
pub struct Record {
    /// Timestamp *in UTC* (key `ts`).
    pub timestamp: Option<SystemTime>,
    /// Log level (key `lvl`).
    pub level: Level,
    /// Log message (key `msg`).
    pub msg: String,
    /// Log message (key `target`).
    pub target: String,
    /// Module that logged the message (key `module`).
    pub module: Option<String>,
    /// File and line number from where the message oriented (key `file`).
    pub file: Option<(String, u32)>,
    /// Additional key value pairs.
    pub key_values: HashMap<String, Value>,
    /// The creation of the struct is private for future extension.
    _private: (),
}

/// A parsed value from a key-value pair.
///
/// Note that parsing is done based on a best-effort basis, which means
/// integers, floats etc. might actual be represented as a [`Value::String`].
#[derive(Debug)]
pub enum Value {
    /// Parsed boolean.
    Bool(bool),
    /// Parsed integer.
    Int(i64),
    /// Parsed floating pointer number.
    Float(f64),
    /// Unparsed string.
    String(String),
}

impl FromStr for Value {
    /// This can always return [`Value::String`].
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(b) = value.parse() {
            Ok(Value::Bool(b))
        } else if let Ok(i) = value.parse() {
            Ok(Value::Int(i))
        } else if let Ok(f) = value.parse() {
            Ok(Value::Float(f))
        } else {
            Ok(Value::String(value.to_owned()))
        }
    }
}

impl Record {
    fn empty() -> Record {
        Record {
            timestamp: None,
            level: Level::Info,
            msg: String::new(),
            target: String::new(),
            module: None,
            file: None,
            key_values: HashMap::new(),
            _private: (),
        }
    }
}