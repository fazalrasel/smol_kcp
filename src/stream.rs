use std::{
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    task::{Context, Poll},
    pin::Pin,
};

use futures_lite::io::{AsyncRead, AsyncWrite};
use kcp::{Error as KcpError, KcpResult};
use log::trace;
use smol::lock::Mutex;

use crate::{config::KcpConfig, socket::KcpSocket};

/// KCP stream for client connections
pub struct KcpStream {
    pub(crate) socket: Arc<Mutex<KcpSocket>>,
    pub(crate) recv_buffer: Vec<u8>,
    pub(crate) recv_buffer_pos: usize,
    pub(crate) recv_buffer_cap: usize,
}

impl KcpStream {
    /// Connect to a KCP server
    pub async fn connect(config: &KcpConfig, addr: SocketAddr) -> KcpResult<Self> {
        let udp_addr = match addr.ip() {
            IpAddr::V4(_) => "0.0.0.0:0",
            IpAddr::V6(_) => "[::]:0",
        };

        let udp = smol::net::UdpSocket::bind(udp_addr).await?;
        udp.connect(addr).await?;
        let udp = Arc::new(udp);

        let mut conv = rand::random::<u32>();
        while conv == 0 {
            conv = rand::random();
        }

        let socket = KcpSocket::new(config, conv, udp, addr, config.stream)?;
        
        Ok(Self {
            socket: Arc::new(Mutex::new(socket)),
            recv_buffer: Vec::new(),
            recv_buffer_pos: 0,
            recv_buffer_cap: 0,
        })
    }

    /// Create a stream from an existing socket (used by listener)
    pub(crate) fn from_socket(socket: Arc<Mutex<KcpSocket>>) -> Self {
        Self {
            socket,
            recv_buffer: Vec::new(),
            recv_buffer_pos: 0,
            recv_buffer_cap: 0,
        }
    }

    /// Send data
    pub async fn send(&mut self, buf: &[u8]) -> KcpResult<usize> {
        let mut socket = self.socket.lock().await;
        let result = socket.send(buf)?;
        socket.flush()?;
        Ok(result)
    }

    /// Receive data
    pub async fn recv(&mut self, buf: &mut [u8]) -> KcpResult<usize> {
        loop {
            // First, try to consume from internal buffer
            if self.recv_buffer_pos < self.recv_buffer_cap {
                let remaining = self.recv_buffer_cap - self.recv_buffer_pos;
                let copy_length = remaining.min(buf.len());

                buf[..copy_length].copy_from_slice(
                    &self.recv_buffer[self.recv_buffer_pos..self.recv_buffer_pos + copy_length]
                );
                self.recv_buffer_pos += copy_length;
                return Ok(copy_length);
            }

            // Try to receive from KCP
            let mut socket = self.socket.lock().await;
            
            // Check if we can read directly into user buffer
            let peek_size = socket.peek_size().unwrap_or(0);
            
            if peek_size > 0 && peek_size <= buf.len() {
                match socket.recv(buf) {
                    Ok(n) => {
                        trace!("recv directly {} bytes", n);
                        return Ok(n);
                    }
                    Err(KcpError::UserBufTooSmall) => {}
                    Err(err) => return Err(err),
                }
            }

            // Need to use internal buffer
            if peek_size > 0 {
                if self.recv_buffer.len() < peek_size {
                    self.recv_buffer.resize(peek_size, 0);
                }

                match socket.recv(&mut self.recv_buffer) {
                    Ok(0) => return Ok(0),
                    Ok(n) => {
                        trace!("recv buffered {} bytes", n);
                        self.recv_buffer_pos = 0;
                        self.recv_buffer_cap = n;
                        continue;
                    }
                    Err(err) => return Err(err),
                }
            }

            // No data available, need to wait for input
            drop(socket);
            
            // Simple polling approach - in a real implementation you'd want proper async waiting
            smol::Timer::after(std::time::Duration::from_millis(1)).await;
        }
    }

    /// Get local address
    pub async fn local_addr(&self) -> io::Result<SocketAddr> {
        let socket = self.socket.lock().await;
        socket.udp_socket().local_addr()
    }

    /// Get peer address
    pub async fn peer_addr(&self) -> SocketAddr {
        let socket = self.socket.lock().await;
        socket.peer_addr()
    }
}

impl AsyncRead for KcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // Simple implementation - in production you'd want proper async polling
        match futures_lite::future::block_on(self.recv(buf)) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(KcpError::IoError(err)) => Poll::Ready(Err(err)),
            Err(err) => Poll::Ready(Err(io::Error::other(err))),
        }
    }
}

impl AsyncWrite for KcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match futures_lite::future::block_on(self.send(buf)) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(KcpError::IoError(err)) => Poll::Ready(Err(err)),
            Err(err) => Poll::Ready(Err(io::Error::other(err))),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // UDP doesn't need explicit flushing
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}