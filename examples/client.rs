use smol_kcp::{KcpConfig, KcpStream};
use std::net::SocketAddr;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    smol::block_on(async {
        let config = KcpConfig::default();
        let server_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        
        println!("Connecting to KCP server at {}", server_addr);
        
        match KcpStream::connect(&config, server_addr).await {
            Ok(mut stream) => {
                println!("Connected successfully!");
                
                let test_data = b"Hello, KCP World!";
                
                match stream.send(test_data).await {
                    Ok(n) => println!("Sent {} bytes", n),
                    Err(e) => {
                        eprintln!("Send error: {}", e);
                        return;
                    }
                }
                
                // Add a small delay to allow server to process
                smol::Timer::after(Duration::from_millis(500)).await;
                
                let mut buf = vec![0u8; 1024];
                
                // Simple receive with multiple attempts
                for attempt in 1..=5 {
                    println!("Receive attempt {}", attempt);
                    match stream.recv(&mut buf).await {
                        Ok(n) => {
                            println!("Received {} bytes: {}", n, String::from_utf8_lossy(&buf[..n]));
                            break;
                        }
                        Err(e) => {
                            eprintln!("Receive error on attempt {}: {}", attempt, e);
                            smol::Timer::after(Duration::from_millis(200)).await;
                        }
                    }
                }
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
        
        println!("Client finished");
    });
}