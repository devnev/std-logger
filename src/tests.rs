// Copyright 2017 Thomas de Zeeuw
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option. This file may not be
// used, copied, modified, or distributed except according to those terms.

use std::{str, panic};
use std::default::Default;
use std::sync::Mutex;

use super::*;

lazy_static! {
    /// A global lock since most tests need to run in serial.
    static ref SERIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Macro to crate a serial test, that lock the `SERIAL_TEST_MUTEX` while
/// testing.
macro_rules! serial_test {
    (fn $name:ident() $body:block) => {
        #[test]
        fn $name() {
            let guard = SERIAL_TEST_MUTEX.lock().unwrap();
            // Catch any panics to not poisen the lock.
            if let Err(err) = panic::catch_unwind(|| { $body }) {
                drop(guard);
                panic::resume_unwind(err);
            }
        }
    }
}

/// Changes the environment.
serial_test!{
    fn should_get_the_correct_log_level_from_env() {
        let tests = vec![
            ("LOG", "TRACE", LogLevelFilter::Trace),
            ("LOG", "ERROR", LogLevelFilter::Error),
            ("LOG_LEVEL", "ERROR", LogLevelFilter::Error),
            ("LOG_LEVEL", "DEBUG", LogLevelFilter::Debug),
            ("TRACE", "1", LogLevelFilter::Trace),
            ("DEBUG", "1", LogLevelFilter::Debug),
        ];

        for test in tests {
            env::set_var(test.0, test.1);

            let want = test.2;
            let got = get_max_level();
            assert_eq!(want, got);

            env::remove_var(test.0);
        }
    }
}

/// Changes the environment and the global log buffer.
serial_test!{
    fn log_output() {
        unsafe { log_setup(); }

        #[cfg(feature = "timestamp")]
        let timestamp = chrono::Utc::now();

        trace!("trace message");
        debug!("debug message");
        info!("info message");
        warn!("warn message");
        error!("error message");
        info!(target: REQUEST_TARGET, "request message");

        let want = vec![
            #[cfg(feature = "log-panic")]
            "[DEBUG] std_logger: enabled std-logger with log level: TRACE, with logging of panics",
            #[cfg(not(feature = "log-panic"))]
            "[DEBUG] std_logger: enabled std-logger with log level: TRACE, no logging of panics",
            "[TRACE] std_logger::tests: trace message",
            "[DEBUG] std_logger::tests: debug message",
            "[INFO] std_logger::tests: info message",
            "[WARN] std_logger::tests: warn message",
            "[ERROR] std_logger::tests: error message",
            "[REQUEST]: request message",
        ];
        let mut got = unsafe {
            (&*LOG_OUTPUT).iter()
        };

        let mut got_length = 0;
        let mut want_iter = want.iter();
        loop {
            match (want_iter.next(), got.next()) {
                (Some(want), Some(got)) if got.is_some() => {
                    let got = got.as_ref().unwrap();
                    let got = str::from_utf8(got).expect("unable to parse string").trim();

                    let mut want = (*want).to_owned();
                    #[cfg(feature = "timestamp")]
                    { want = add_timestamp(want, timestamp, got); }

                    // TODO: for some reason this failure doesn't shows itself in the
                    // output, hence this workaround.
                    println!("Comparing:");
                    println!("want: {}", want);
                    println!("got:  {}", got);
                    assert_eq!(got, want.as_str(), "message differ");

                    got_length += 1;
                },
                _ => break,
            }
        }

        if got_length != want.len() {
            panic!("the number of log messages got differs from the amount of messages wanted");
        }
    }
}

/// Changes the environment and the global log buffer.
#[cfg(feature = "log-panic")]
serial_test!{
    fn log_panics() {
        use std::path::MAIN_SEPARATOR;

        unsafe { log_setup(); }

        assert!(panic::catch_unwind(|| panic!("oops")).is_err());

        // Get the timetamp after causing the panic to (hopefully) reduce the
        // flakyness of this test.
        #[cfg(feature = "timestamp")]
        let timestamp = chrono::Utc::now();

        let output = unsafe { (&*LOG_OUTPUT)[1].as_ref() };
        if let Some(output) = output {
            let got = str::from_utf8(output).expect("unable to parse string").trim();
            let mut want = format!("[ERROR] panic: thread \'tests::log_panics\' \
                panicked at \'oops\': src{}tests.rs:129", MAIN_SEPARATOR);
            #[cfg(feature = "timestamp")]
            { want = add_timestamp(want, timestamp, got); }

            println!("Comparing:");
            println!("want: {}", want);
            println!("got:  {}", &got[0..want.len()]);
            assert!(got.starts_with(&want));
        } else {
            panic!("can't retrieve output");
        }
    }
}

/// This requires the `SERIAL_TEST_MUTEX` to be held!
unsafe fn log_setup() {
    use std::sync::atomic::Ordering;

    // Cleanup the old logs.
    if LOG_OUTPUT.as_mut().is_some() {
        LOG_OUTPUT_INDEX.store(1, Ordering::Relaxed);
        return;
    }

    let output = Box::new(Default::default());
    LOG_OUTPUT = Box::into_raw(output);

    env::set_var("LOG_LEVEL", "TRACE");
    init();
    env::remove_var("LOG_LEVEL");
}

#[cfg(feature = "timestamp")]
fn add_timestamp(message: String, timestamp: chrono::DateTime<chrono::Utc>, got: &str) -> String {
    use chrono::{Datelike, Timelike};

    // Add the timestamp to the expected string.
    let timestamp = format!("{:004}-{:02}-{:02}T{:02}:{:02}:{:02}.{}Z",
        timestamp.year(), timestamp.month(), timestamp.day(),
        timestamp.hour(), timestamp.minute(), timestamp.second(),
        &got[20..26]);
    format!("{} {}", timestamp, message)
}
