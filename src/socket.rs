use std::{
    io::{self, Write},
    net::SocketAddr,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use kcp::{Kcp, KcpResult};
use log::trace;

use crate::config::KcpConfig;

/// KCP socket implementation
pub struct KcpSocket {
    kcp: Kcp<KcpOutput>,
    udp: Arc<smol::net::UdpSocket>,
    peer_addr: SocketAddr,
    last_update: Instant,
}

impl KcpSocket {
    pub fn new(
        config: &KcpConfig,
        conv: u32,
        udp: Arc<smol::net::UdpSocket>,
        peer_addr: SocketAddr,
        _stream: bool,
    ) -> KcpResult<Self> {
        let output = KcpOutput::new(udp.clone(), peer_addr);
        let mut kcp = Kcp::new(conv, output);
        
        config.apply_config(&mut kcp);
        // Note: set_stream method doesn't exist in kcp 0.5.3, stream mode is handled differently

        Ok(Self {
            kcp,
            udp,
            peer_addr,
            last_update: Instant::now(),
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn input(&mut self, data: &[u8]) -> KcpResult<bool> {
        self.last_update = Instant::now();
        // Update KCP before input
        let current = current_millis();
        self.kcp.update(current)?;
        match self.kcp.input(data) {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> KcpResult<usize> {
        self.last_update = Instant::now();
        // Update KCP before sending
        let current = current_millis();
        self.kcp.update(current)?;
        self.kcp.send(data)
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> KcpResult<usize> {
        // Update KCP before receiving
        let current = current_millis();
        self.kcp.update(current)?;
        self.kcp.recv(buf)
    }

    pub fn peek_size(&self) -> Option<usize> {
        self.kcp.peeksize().ok()
    }

    pub fn flush(&mut self) -> KcpResult<()> {
        // Update before flush
        let current = current_millis();
        self.kcp.update(current)?;
        self.kcp.flush()
    }

    pub fn udp_socket(&self) -> &Arc<smol::net::UdpSocket> {
        &self.udp
    }
}

/// KCP output implementation
struct KcpOutput {
    udp: Arc<smol::net::UdpSocket>,
    peer_addr: SocketAddr,
}

impl KcpOutput {
    fn new(udp: Arc<smol::net::UdpSocket>, peer_addr: SocketAddr) -> Self {
        Self { udp, peer_addr }
    }
}

impl Write for KcpOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Use blocking send_to - this is fine for KCP output
        // In a real async implementation, we'd need to handle this differently
        // but for simplicity and router compatibility, blocking is acceptable
        futures_lite::future::block_on(async {
            match self.udp.send_to(buf, self.peer_addr).await {
                Ok(n) => {
                    trace!("UDP sent {} bytes to {}", n, self.peer_addr);
                    Ok(n)
                }
                Err(e) => Err(e),
            }
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn current_millis() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32
}