use smol_kcp::{KcpConfig, KcpListener, KcpStream};
use std::net::SocketAddr;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    futures_lite::future::block_on(async {
        let config = KcpConfig::default();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        
        println!("Starting simple KCP test...");
        
        // For embedded systems, we'll run a simpler sequential test
        // Create server and client futures without spawning tasks
        let server_config = config.clone();
        
        let server_future = async move {
            let mut listener = KcpListener::bind(server_config, addr).await.unwrap();
            println!("Server listening on {}", addr);
            
            let (mut stream, peer_addr) = listener.accept().await.unwrap();
            println!("Server: accepted connection from {}", peer_addr);
            
            let mut buf = vec![0u8; 1024];
            match stream.recv(&mut buf).await {
                Ok(n) => {
                    println!("Server: received {} bytes: {}", n, String::from_utf8_lossy(&buf[..n]));
                    // Echo back
                    stream.send(&buf[..n]).await.unwrap();
                    println!("Server: echoed {} bytes", n);
                }
                Err(e) => eprintln!("Server receive error: {}", e),
            }
        };
        
        let client_future = async move {
            // Give server time to start
            async_io::Timer::after(Duration::from_millis(100)).await;
            
            let mut stream = KcpStream::connect(&config, addr).await.unwrap();
            println!("Client: connected to server");
            
            let test_data = b"Hello from client!";
            stream.send(test_data).await.unwrap();
            println!("Client: sent {} bytes", test_data.len());
            
            // Try to receive echo
            let mut buf = vec![0u8; 1024];
            match stream.recv(&mut buf).await {
                Ok(n) => {
                    println!("Client: received {} bytes: {}", n, String::from_utf8_lossy(&buf[..n]));
                }
                Err(e) => eprintln!("Client receive error: {}", e),
            }
        };
        
        // Run both futures concurrently using futures_lite
        futures_lite::future::zip(server_future, client_future).await;
        
        println!("Test completed");
    });
}