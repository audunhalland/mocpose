#![cfg(feature = "mock-std")]

use std::io::{BufRead, BufReader, Write};

use unimock::*;

#[test]
fn test_display() {
    assert_eq!(
        "u",
        Unimock::new(
            mock::core::fmt::DisplayMock::fmt
                .next_call(matching!())
                .mutates(|f, ()| write!(f, "u"))
        )
        .to_string()
    );
}

#[test]
#[should_panic = "a Display implementation returned an error unexpectedly: Error"]
fn test_display_error() {
    Unimock::new(
        mock::core::fmt::DisplayMock::fmt
            .next_call(matching!())
            .returns(Err(core::fmt::Error)),
    )
    .to_string();
}

#[test]
fn test_debug() {
    let unimock = Unimock::new(
        mock::core::fmt::DebugMock::fmt
            .next_call(matching!())
            .mutates(|f, ()| write!(f, "u")),
    );

    assert_eq!("u", format!("{unimock:?}"));
}

#[test]
fn test_read() {
    let mut reader = BufReader::new(Unimock::new((
        mock::std::io::ReadMock::read
            .next_call(matching!())
            .mutates(|mut f, ()| f.write(b"ok")),
        mock::std::io::ReadMock::read
            .next_call(matching!())
            .mutates(|mut f, ()| f.write(b"\n")),
    )));

    let mut line = String::new();
    let len = reader.read_line(&mut line).unwrap();
    assert_eq!(len, 3);
    assert_eq!("ok\n", line);
}

#[allow(clippy::write_literal)]
#[test]
fn test_write() {
    let mut unimock = Unimock::new((
        mock::std::io::WriteMock::write_all
            .next_call(matching!(eq!(b"hello ")))
            .returns(Ok(())),
        mock::std::io::WriteMock::write_all
            .next_call(matching!(eq!(b"world")))
            .returns(Ok(())),
    ));

    use std::io::Write;
    write!(&mut unimock, "hello {}", "world").unwrap();
}

#[allow(clippy::write_literal)]
#[test]
#[should_panic = "Write::write_all([119, 111, 114, 108, 100]): Ordered call (2) out of range"]
fn test_write_fail() {
    let mut unimock = Unimock::new(
        mock::std::io::WriteMock::write_all
            .next_call(matching!(eq!(b"hello ")))
            .returns(Ok(())),
    );

    use std::io::Write;
    write!(&mut unimock, "hello {}", "world").unwrap();
}

#[test]
fn test_fmt_io_multiplex_default_impl_implicit() {
    let unimock = Unimock::new((
        mock::core::fmt::DisplayMock::fmt
            .next_call(matching!())
            .mutates(|f, ()| write!(f, "hello {}", "unimock")),
        // NOTE: write! calls `write_all` which should get re-routed to `write`:
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"hello ")))
            .returns(Ok(6)),
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"unimock")))
            .returns(Ok("uni".len())),
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"mock")))
            .returns(Ok("mock".len())),
    ));
    write!(&mut unimock.clone(), "{unimock}").unwrap();
}

#[test]
fn test_fmt_io_multiplex_default_impl_explicit() {
    let unimock = Unimock::new((
        mock::core::fmt::DisplayMock::fmt
            .next_call(matching!())
            .mutates(|f, ()| write!(f, "hello {}", "unimock")),
        mock::std::io::WriteMock::write_all
            .next_call(matching!(eq!(b"hello ")))
            .calls_default_impl(),
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"hello ")))
            .returns(Ok(6)),
        mock::std::io::WriteMock::write_all
            .next_call(matching!(eq!(b"unimock")))
            .calls_default_impl(),
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"unimock")))
            .returns(Ok("uni".len())),
        mock::std::io::WriteMock::write
            .next_call(matching!(eq!(b"mock")))
            .returns(Ok("mock".len())),
    ));
    write!(&mut unimock.clone(), "{unimock}").unwrap();
}
