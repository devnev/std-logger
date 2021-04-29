//! A crate that holds a logging implementation that logs to standard error and
//! standard out. It uses standard error for all regular messages and standard
//! out for requests.
//!
//! This crate provides only a logging implementation. To do actual logging use
//! the [`log`] crate and it's various macros.
//!
//!
//! # Setting severity
//!
//! You can use various environment variables to change the severity (log level)
//! of the messages to actually log and which to ignore.
//!
//! `LOG` and `LOG_LEVEL` can be used to set the severity to a specific value,
//! see the [`log`]'s package [`LevelFilter`] type for available values.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Set the log severity to only print log message with info severity or
//! # higher, trace and debug messages won't be printed anymore.
//! $ LOG=info ./my_binary
//!
//! # Set the log severity to only print log message with warning severity or
//! # higher, informational (or lower severity) messages won't be printed
//! # anymore.
//! $ LOG=warn ./my_binary
//! ```
//!
//! Alternatively setting the `TRACE` variable (e.g. `TRACE=1`) sets the
//! severity to the trace, meaning it will log everything. Setting `DEBUG` will
//! set the severity to debug.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Enables trace logging.
//! $ TRACE=1 ./my_binary
//!
//! # Enables debug logging.
//! $ DEBUG=1 ./my_binary
//! ```
//!
//! If none of these environment variables are found it will default to an
//! information severity.
//!
//!
//! # Logging requests
//!
//! To log requests a special target is provided: [`REQUEST_TARGET`] and a
//! special macro: [`request`]. This will cause the message to be logged to
//! standard out, rather then standard error. This allows for separate
//! processing of error messages and request logs.
//!
//! ```
//! use std_logger::request;
//!
//! # fn main() {
//! request!("Got a request!");
//! # }
//! ```
//!
//!
//! # Limiting logging targets
//!
//! Sometimes it's useful to only log messages related to a specific target, for
//! example when debugging a single function you might want only see messages
//! from the module the function is in. This can be achieved by using the
//! `LOG_TARGET` environment variable.
//!
//! ```bash
//! # In your shell of choose:
//!
//! # Only log messages from your crate.
//! $ LOG_TARGET=my_crate ./my_binary
//!
//! # Only log messages from the `my_module` module in your crate.
//! $ LOG_TARGET=my_crate::my_module ./my_binary
//!
//! # Multiple log targets are also supported by separating the values by a
//! # comma.
//! $ LOG_TARGET=my_crate::my_module,my_crate::my_other_module ./my_binary
//!
//! # Very useful in combination with trace severity to get all messages you
//! # want, but filter out the message you don't need.
//! $ LOG_LEVEL=trace LOG_TARGET=my_crate::my_module ./my_binary
//! ```
//!
//! Note that [requests] and panics (with target="panic") are always logged.
//!
//! [requests]: index.html#logging-requests
//!
//!
//! # Format
//!
//! The format follows the [logfmt] format. For regular messages, printed to
//! standard error, the following format is used:
//!
//! ```text
//! ts="YYYY-MM-DDTHH:MM:SS.MICROSZ" lvl="$LOG_LEVEL" msg="$message" target="$target" module="$module"
//!
//! For example:
//!
//! ts="2018-03-24T13:48:28.820588Z" lvl="ERROR" msg="my error message" target="my_module" module="my_module"
//! ```
//!
//! For requests, logged using the [`REQUEST_TARGET`] target or the [`request`]
//! macro and printed to standard out, the following format is used:
//!
//! ```text
//! ts="YYYY-MM-DDTHH:MM:SS.MICROSZ" lvl="INFO" msg="$message" target="request" module="$module"
//!
//! For example:
//!
//! ts="2018-03-24T13:30:28.820588Z" lvl="INFO" msg="my request message" target="request" module="my_module"
//! ```
//!
//! Note: the timestamp is not printed when the *timestamp* feature is not
//! enabled, this feature is enabled by default, see [Timestamp feature] below.
//!
//! [logfmt]: https://www.brandur.org/logfmt
//!
//!
//! # Crate features
//!
//! This crate has three features:
//! * *timestamp*, enabled by default.
//! * *log-panic*, enabled by default.
//! * *nightly*, disabled by default.
//!
//!
//! ## Timestamp feature
//!
//! The *timestamp* feature adds a timestamp in front of every message. It uses
//! the format defined in [`RFC3339`] with 6 digit microsecond precision, e.g.
//! `2018-03-24T13:48:48.063934Z`. The timestamp is **always** logged in UTC.
//!
//! ### Notes
//!
//! This feature uses [`SystemTime`] as time source, which **is not monotonic**.
//! This means that a log message created after an *earlier* log message can
//! have a timestamp **before** the earlier created log message.
//!
//! [`SystemTime`]: std::time::SystemTime
//!
//!
//! ## Log-panic feature
//!
//! The *log-panic* feature will log all panics using the `error` severity,
//! rather then using the default panic handler. It will log the panic message
//! as well as the location and a backtrace, see the log output below for an
//! example (this example doesn't include a timestamp).
//!
//! ```log
//! lvl="ERROR" msg="thread 'main' panicked at 'oops', examples/panic.rs:24" target="panic" module="" backtrace="
//! stack backtrace:
//!    0:        0x106ba8f74 - backtrace::backtrace::trace<closure>
//!                         at backtrace-0.3.2/src/backtrace/mod.rs:42
//!    1:        0x106ba49af - backtrace::capture::Backtrace::new::h54d7cfa8f40c5b43
//!                         at backtrace-0.3.2/src/capture.rs:64
//!    2:        0x106b9f4e6 - log_panics::init::{{closure}}
//!                         at log-panics-1.2.0/src/lib.rs:52
//!    3:        0x106bc6951 - std::panicking::rust_panic_with_hook::h6c19f9ba35264287
//!                         at src/libstd/panicking.rs:612
//!    4:        0x106b93146 - std::panicking::begin_panic<&str>
//!                         at src/libstd/panicking.rs:572
//!    5:        0x106b93bf1 - panic::main
//!                         at examples/panic.rs:24
//!    6:        0x106bc751c - __rust_maybe_catch_panic
//!                         at src/libpanic_unwind/lib.rs:98
//!    7:        0x106bc6c08 - std::rt::lang_start::h6f338c4ae2d58bbe
//!                         at src/libstd/rt.rs:61
//!    8:        0x106b93c29 - main
//! "
//! ```
//!
//! If the *timestamp* feature is enable the first line of the message will be
//! prefixed with a timestamp as described in the [Timestamp feature].
//!
//!
//! ## Nightly feature
//!
//! Enabling this feature enables the crate to use unstable (i.e. nightly-only)
//! features from the compiler and standard library.
//!
//! Currently this is limited to using the [`std::backtrace`] for creating
//! backtraces, rather than an external library.
//!
//!
//! # Examples
//!
//! ```
//! # use std::time::Duration;
//! #
//! use log::info;
//! use std_logger::request;
//!
//! fn main() {
//!     // First thing we need to do is initialise the logger before anything
//!     // else.
//!     std_logger::init();
//!
//!     // Now we can start logging!
//!     info!("Our application started!");
//!
//!     // Do useful stuff, like starting a HTTP server.
//! #   log_handler(Request { url: "/some_page".to_owned(), status: 200,
//! #       response_time: Duration::from_millis(100) });
//! }
//!
//! # struct Request {
//! #   url: String,
//! #   status: u16,
//! #   response_time: Duration,
//! # }
//! #
//! /// This our example request handler, just pretend it gets called with a
//! /// request.
//! fn log_handler(req: Request) {
//!     // This will be logged to standard out, rather then standard error.
//!     request!("url = {}, status = {}, response_time = {:?}",
//!         req.url, req.status, req.response_time);
//! }
//! ```
//!
//! [`REQUEST_TARGET`]: constant.REQUEST_TARGET.html
//! [`log`]: https://crates.io/crates/log
//! [`RFC3339`]: https://tools.ietf.org/html/rfc3339
//! [Timestamp feature]: #timestamp-feature

#![warn(missing_debug_implementations, missing_docs, unused_results)]
#![cfg_attr(all(feature = "log-panic", feature = "nightly"), feature(backtrace))]

use std::cell::RefCell;
use std::env;
use std::io::{self, IoSlice, Write};

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

mod format;
use format::{Buffer, BUFS_SIZE};

#[cfg(test)]
mod tests;

/// Target for logging requests.
///
/// The [`request`] macro provides a convenient way to log requests.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html#logging-requests
pub const REQUEST_TARGET: &str = "request";

/// Logs a request.
///
/// This uses [info] level severity and the [`REQUEST_TARGET`] target to log a
/// request. See the [crate level documentation] for more.
///
/// [info]: log::Level::Info
/// [crate level documentation]: index.html#logging-requests
#[macro_export]
macro_rules! request {
    ($( $arg: tt )*) => (
        $crate::_log::log!(target: $crate::REQUEST_TARGET, $crate::_log::Level::Info, $($arg)*);
    )
}

// Not part of the API. Only here for use in the `request!` macro.
#[doc(hidden)]
pub use log as _log;

/// Initialise the logger.
///
/// See the [crate level documentation] for more.
///
/// [crate level documentation]: index.html
///
/// # Panics
///
/// This will panic if the logger fails to initialise. Use [`try_init`] if you
/// want to handle the error yourself.
pub fn init() {
    try_init().unwrap_or_else(|err| panic!("failed to initialise the logger: {}", err));
}

/// Try to initialise the logger.
///
/// Unlike [`init`] this doesn't panic when the logger fails to initialise. See
/// the [crate level documentation] for more.
///
/// [`init`]: fn.init.html
/// [crate level documentation]: index.html
pub fn try_init() -> Result<(), SetLoggerError> {
    let filter = get_max_level();
    let targets = get_log_targets();
    let logger = Logger { filter, targets };
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(filter);

    #[cfg(all(feature = "log-panic", not(feature = "nightly")))]
    log_panics::init();
    #[cfg(all(feature = "log-panic", feature = "nightly"))]
    std::panic::set_hook(Box::new(log_panic));
    Ok(())
}

/// Get the maximum log level based on the environment.
fn get_max_level() -> LevelFilter {
    for var in &["LOG", "LOG_LEVEL"] {
        if let Ok(level) = env::var(var) {
            if let Ok(level) = level.parse() {
                return level;
            }
        }
    }

    if env::var("TRACE").is_ok() {
        LevelFilter::Trace
    } else if env::var("DEBUG").is_ok() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    }
}

/// Get the targets to log, if any.
fn get_log_targets() -> Targets {
    match env::var("LOG_TARGET") {
        Ok(ref targets) if !targets.is_empty() => {
            Targets::Only(targets.split(',').map(|target| target.into()).collect())
        }
        _ => Targets::All,
    }
}

/// Panic hook that logs the panic using [`log::error!`].
#[cfg(all(feature = "log-panic", feature = "nightly"))]
fn log_panic(info: &std::panic::PanicInfo<'_>) {
    use std::backtrace::Backtrace;
    use std::thread;

    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &**s,
            None => "<unknown>",
        },
    };
    let (file, line) = match info.location() {
        Some(location) => (location.file(), location.line()),
        None => ("<unknown>", 0),
    };
    let backtrace = Backtrace::force_capture();

    log::logger().log(
        &Record::builder()
            .args(format_args!(
                // NOTE: we include file in here because it's only logged when
                // debug severity is enabled.
                "thread '{}' panicked at '{}', {}:{}",
                thread_name, msg, file, line
            ))
            .level(log::Level::Error)
            .target("panic")
            .file(Some(file))
            .line(Some(line))
            .key_values(&("backtrace", &backtrace as &dyn std::fmt::Display))
            .build(),
    );
}

/// Our `Log` implementation.
struct Logger {
    /// The filter used to determine what messages to log.
    filter: LevelFilter,
    /// What logging targets to log.
    targets: Targets,
}

#[derive(Debug, Eq, PartialEq)]
enum Targets {
    /// Log all targets.
    All,
    /// Only log certain targets.
    Only(Box<[Box<str>]>),
}

impl Targets {
    /// Returns `true` if the `target` should be logged.
    fn should_log(&self, target: &str) -> bool {
        if target == REQUEST_TARGET || target == "panic" {
            // Always log requests.
            true
        } else if let Targets::Only(targets) = self {
            // Log all targets that start with an allowed target. This way we
            // can just use `LOG_TARGET=my_crate`, rather then
            // `LOG_TARGET=my_crate::module1,my_crate::module2` etc.
            targets
                .iter()
                .any(|log_target| target.starts_with(&**log_target))
        } else {
            // All targets should be logged.
            true
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter >= metadata.level() && self.targets.should_log(metadata.target())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            log(record, self.filter >= LevelFilter::Debug);
        }
    }

    fn flush(&self) {
        // Can't flush standard error/out.
    }
}

/// The actual logging of a record.
fn log(record: &Record, debug: bool) {
    // Thread local buffer for logging. This way we only lock standard out/error
    // for a single writev call and don't create half written logs.
    thread_local! {
        static BUF: RefCell<Buffer> = RefCell::new(Buffer::new());
    }

    BUF.with(|buf| {
        let mut bufs = [IoSlice::new(&[]); BUFS_SIZE];
        match buf.try_borrow_mut() {
            Ok(mut buf) => {
                // NOTE: keep in sync with the `Err` branch below.
                let bufs = format::record(&mut bufs, &mut buf, record, debug);
                match record.target() {
                    REQUEST_TARGET => write_once(stdout(), bufs),
                    _ => write_once(stderr(), bufs),
                }
                .unwrap_or_else(log_failure);
            }
            Err(_) => {
                // NOTE: We only get to this branch if we're panicking while
                // calling `format::record`, e.g. when a `fmt::Display` impl in
                // the `record` panics, and the `log-panic` feature is enabled
                // which calls `error!` and in turn this function again, while
                // still borrowing `BUF`.
                let mut buf = Buffer::new();
                // NOTE: keep in sync with the `Ok` branch above.
                let bufs = format::record(&mut bufs, &mut buf, record, debug);
                match record.target() {
                    REQUEST_TARGET => write_once(stdout(), bufs),
                    _ => write_once(stderr(), bufs),
                }
                .unwrap_or_else(log_failure);
            }
        }
    });
}

/// Write the entire `buf`fer into the `output` or return an error.
#[inline(always)]
fn write_once<W>(mut output: W, bufs: &[IoSlice]) -> io::Result<()>
where
    W: Write,
{
    output.write_vectored(bufs).and_then(|written| {
        let total_len = bufs.iter().map(|b| b.len()).sum();
        if written == total_len {
            Ok(())
        } else {
            // Not completely correct when going by the name alone, but it's the
            // closest we can get to a descriptive error.
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to write entire log message",
            ))
        }
    })
}

/// The function that gets called when we're unable to print a message.
#[inline(never)]
#[cold]
fn log_failure(err: io::Error) {
    // We've just failed to log, no point in failing to log the fact that we
    // have failed to log... So we remove our panic hook and use the default
    // instead.
    #[cfg(feature = "log-panic")]
    drop(std::panic::take_hook());

    panic!("unexpected error logging message: {}", err)
}

// Functions to get standard out/error, which are stubbed in testing. Even
// though the return type of the functions are different we only need them both
// to implement `io::Write`.

#[cfg(test)]
use self::test_instruments::{stderr, stdout, LOG_OUTPUT};
#[cfg(not(test))]
use std::io::{stderr, stdout};

// The testing variant of the functions.

#[cfg(test)]
mod test_instruments {
    use std::io::{self, IoSlice, Write};
    use std::mem::replace;
    use std::sync::Mutex;

    use lazy_static::lazy_static;

    lazy_static! {
        /// Global log output.
        pub(crate) static ref LOG_OUTPUT: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
    }

    /// Simple wrapper around a `Vec<u8>` which adds itself to `LOG_OUTPUT` when
    /// dropped.
    pub(crate) struct LogOutput {
        inner: Vec<u8>,
    }

    impl Write for LogOutput {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.inner.write(buf)
        }

        fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            self.inner.write_vectored(bufs)
        }

        fn flush(&mut self) -> io::Result<()> {
            unreachable!()
        }
    }

    impl Drop for LogOutput {
        fn drop(&mut self) {
            let buf = replace(&mut self.inner, Vec::new());
            LOG_OUTPUT.lock().unwrap().push(buf);
        }
    }

    pub(crate) fn stdout() -> LogOutput {
        LogOutput { inner: Vec::new() }
    }

    pub(crate) fn stderr() -> LogOutput {
        LogOutput { inner: Vec::new() }
    }
}
