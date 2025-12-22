use smol_kcp::{KcpConfig, KcpListener, KcpStream};
use std::net::SocketAddr;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    smol::block_on(async {
        let config = KcpConfig::default();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        
        // Start server
        let server_task = smol::spawn(async move {
            let mut listener = KcpListener::bind(config, addr).await.unwrap();
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
        });
        
        // Give server time to start
        smol::Timer::after(Duration::from_millis(100)).await;
        
        // Start client
        let client_task = smol::spawn(async move {
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
        });
        
        // Wait for both tasks
        let _ = futures_lite::future::zip(server_task, client_task).await;
        
        println!("Test completed");
    });
}