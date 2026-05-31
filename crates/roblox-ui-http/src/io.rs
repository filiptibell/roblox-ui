use std::pin::Pin;
use std::task::{Context, Poll};

use futures_lite::io::{AsyncRead, AsyncWrite};
use hyper::rt::{Read as HyperRead, ReadBufCursor, Write as HyperWrite};

/**
    Adapter that turns any `futures` AsyncRead + AsyncWrite (such as an
    `async_net::TcpStream` or a `futures_rustls` TLS stream) into the
    `hyper::rt::{Read, Write}` traits that hyper 1.x requires.

    There is no `hyper-util` runtime adapter for the smol / async-* stack, so
    we provide our own. The inner stream must be `Unpin`, which all of the
    streams we use here are.
*/
pub struct FuturesIo<T> {
    inner: T,
}

impl<T> FuturesIo<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: AsyncRead + Unpin> HyperRead for FuturesIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        // hyper hands us a (possibly uninitialized) cursor. We read into a
        // temporary stack buffer sized to fit the cursor, then copy across.
        let mut tmp = [0u8; 8192];
        let dst = unsafe { buf.as_mut() };
        let len = dst.len().min(tmp.len());
        match Pin::new(&mut self.inner).poll_read(cx, &mut tmp[..len]) {
            Poll::Ready(Ok(n)) => {
                // SAFETY: we only write `n` initialized bytes and advance by `n`.
                unsafe {
                    std::ptr::copy_nonoverlapping(tmp.as_ptr(), dst.as_mut_ptr() as *mut u8, n);
                    buf.advance(n);
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: AsyncWrite + Unpin> HyperWrite for FuturesIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx)
    }
}
