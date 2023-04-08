//! Mock APIs for `std` traits

/// Mock APIs for `std::io` traits
pub mod io {
    use std::io::{IoSlice, IoSliceMut, Result};

    use unimock_macros::unimock;

    #[unimock(prefix=crate, api=ReadMock, emulate=std::io::Read)]
    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
        fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {}
        fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {}
        fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {}
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {}
    }

    #[unimock(prefix=crate, api=WriteMock, emulate=std::io::Write)]
    pub trait Write {
        fn write(&mut self, buf: &[u8]) -> Result<usize>;
        fn flush(&mut self) -> Result<()>;
        fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {}
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {}

        // FIXME: This is not implemented (yet) because of self-lifetime in argument.
        // It just uses the default implementation which delegates to `Self::write`.
        // fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> std::io::Result<()> {}
    }
}