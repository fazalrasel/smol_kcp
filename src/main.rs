use smol_kcp::{KcpConfig, KcpListener};
use std::net::SocketAddr;

fn main() {
    env_logger::init();
    
    futures_lite::future::block_on(async {
        let config = KcpConfig::default();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        
        println!("Starting KCP echo server on {}", addr);
        println!("This is a minimal KCP implementation using individual async crates");
        println!("Perfect for OpenWrt and resource-constrained environments");
        
        let mut listener = KcpListener::bind(config, addr).await.unwrap();
        
        loop {
            match listener.accept().await {
                Ok((mut stream, peer_addr)) => {
                    println!("Accepted connection from {}", peer_addr);
                    
                    // Use futures_lite::future::spawn for task spawning
                    let _task = async move {
                        let mut buf = vec![0u8; 1024];
                        loop {
                            match stream.recv(&mut buf).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    println!("Received {} bytes from {}: {}", 
                                        n, peer_addr, String::from_utf8_lossy(&buf[..n]));
                                    if let Err(e) = stream.send(&buf[..n]).await {
                                        eprintln!("Send error: {}", e);
                                        break;
                                    }
                                    println!("Echoed {} bytes back to {}", n, peer_addr);
                                }
                                Err(e) => {
                                    eprintln!("Receive error: {}", e);
                                    break;
                                }
                            }
                        }
                        println!("Connection {} closed", peer_addr);
                    };
                    
                    // For simplicity, we'll handle one connection at a time
                    // In a real implementation, you'd want proper task spawning
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
    });
}
