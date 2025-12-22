use std::{
    collections::HashMap,
    io,
    net::SocketAddr,
    sync::Arc,
};

use async_io::Async;
use async_std::sync::Mutex;
use kcp::KcpResult;
use log::{debug, error, trace};

use crate::{config::KcpConfig, socket::KcpSocket, stream::KcpStream};

/// KCP listener for accepting connections
pub struct KcpListener {
    udp: Arc<Async<std::net::UdpSocket>>,
    config: KcpConfig,
    sessions: Arc<Mutex<HashMap<SocketAddr, Arc<Mutex<KcpSocket>>>>>,
}

impl KcpListener {
    /// Bind to an address
    pub async fn bind(config: KcpConfig, addr: SocketAddr) -> KcpResult<Self> {
        let udp = std::net::UdpSocket::bind(addr)?;
        let udp = Arc::new(Async::new(udp)?);

        Ok(Self {
            udp,
            config,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Accept a new connection
    pub async fn accept(&mut self) -> KcpResult<(KcpStream, SocketAddr)> {
        let mut buf = vec![0u8; 65536];

        loop {
            let (n, peer_addr) = self.udp.recv_from(&mut buf).await?;
            
            if n < kcp::KCP_OVERHEAD {
                error!("packet too short: {} bytes", n);
                continue;
            }

            let packet = &buf[..n];
            let mut conv = kcp::get_conv(packet);

            // Allocate conv if needed
            if conv == 0 {
                conv = {
                    let mut new_conv = rand::random::<u32>();
                    while new_conv == 0 {
                        new_conv = rand::random();
                    }
                    new_conv
                };
                debug!("allocated conv {} for peer {}", conv, peer_addr);
            }

            let mut sessions = self.sessions.lock().await;
            
            // Check if session exists
            if let Some(socket) = sessions.get(&peer_addr) {
                let mut socket = socket.lock().await;
                if let Err(e) = socket.input(packet) {
                    error!("input error: {}", e);
                }
                continue;
            }

            // Create new session
            let socket = KcpSocket::new(
                &self.config,
                conv,
                self.udp.clone(),
                peer_addr,
                self.config.stream,
            )?;

            let socket = Arc::new(Mutex::new(socket));
            
            // Input the first packet
            {
                let mut s = socket.lock().await;
                if let Err(e) = s.input(packet) {
                    error!("initial input error: {}", e);
                    continue;
                }
            }

            sessions.insert(peer_addr, socket.clone());
            drop(sessions);

            trace!("accepted new connection from {}", peer_addr);

            let stream = KcpStream::from_socket(socket);

            return Ok((stream, peer_addr));
        }
    }

    /// Get local address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.udp.get_ref().local_addr()
    }
}