use std::io;
use std::net::SocketAddr;

use mio::{TryRead, TryWrite};
use mio::tcp::TcpStream;
#[cfg(all(not(windows), feature="ssl"))]
use openssl::ssl::SslStream;
#[cfg(all(not(windows), feature="ssl"))]
use openssl::ssl::error::Error as SslError;

use result::{Result, Error, Kind};

use self::Stream::*;
pub enum Stream {
    Tcp(TcpStream),
    #[cfg(all(not(windows), feature="ssl"))]
    Tls {
        sock: SslStream<TcpStream>,
        negotiating: bool,
    }
}


impl Stream {

    pub fn tcp(stream: TcpStream) -> Stream {
        Tcp(stream)
    }

    #[cfg(all(not(windows), feature="ssl"))]
    pub fn tls(stream: SslStream<TcpStream>) -> Stream {
        Tls { sock: stream, negotiating: false }
    }

    #[cfg(all(not(windows), feature="ssl"))]
    pub fn is_tls(&self) -> bool {
        match *self {
            Tcp(_) => false,
            #[cfg(all(not(windows), feature="ssl"))]
            Tls {..} => true,
        }
    }

    pub fn evented(&self) -> &TcpStream {
        match *self {
            Tcp(ref sock) => sock,
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { ref sock, ..} => sock.get_ref(),
        }
    }

    pub fn is_negotiating(&self) -> bool {
        match *self {
            Tcp(_) => false,
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { sock: _, ref negotiating } => *negotiating,
        }

    }

    pub fn clear_negotiating(&mut self) -> Result<()> {
        match *self {
            Tcp(_) => Err(Error::new(Kind::Internal, "Attempted to clear negotiating flag on non ssl connection.")),
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { sock: _, ref mut negotiating } => Ok(*negotiating = false),
        }
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        match *self {
            Tcp(ref sock) => sock.peer_addr(),
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { ref sock, ..} => sock.get_ref().peer_addr(),
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        match *self {
            Tcp(ref sock) => sock.local_addr(),
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { ref sock, ..} => sock.get_ref().local_addr(),
        }
    }
}

impl TryRead for Stream {

    fn try_read(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>> {
        match *self {
            Tcp(ref mut sock) => sock.try_read(buf),
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { ref mut sock, ref mut negotiating } => {
                match sock.ssl_read(buf) {
                    Ok(cnt) => Ok(Some(cnt)),
                    Err(SslError::WantWrite(_)) => {
                        *negotiating = true;
                        Ok(None)
                    },
                    Err(SslError::WantRead(_)) => Ok(None),
                    Err(err) =>
                        Err(io::Error::new(io::ErrorKind::Other, err)),
                }
            }
        }
    }
}

impl TryWrite for Stream {

    fn try_write(&mut self, buf: &[u8]) -> io::Result<Option<usize>> {
        match *self {
            Tcp(ref mut sock) => sock.try_write(buf),
            #[cfg(all(not(windows), feature="ssl"))]
            Tls { ref mut sock, ref mut negotiating } => {

                *negotiating = false;

                match sock.ssl_write(buf) {
                    Ok(cnt) => Ok(Some(cnt)),
                    Err(SslError::WantRead(_)) => {
                        *negotiating = true;
                        Ok(None)
                    },
                    Err(SslError::WantWrite(_)) => Ok(None),
                    Err(err) =>
                        Err(io::Error::new(io::ErrorKind::Other, err)),
                }
            }
        }
    }
}
