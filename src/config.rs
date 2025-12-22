use std::{io::Write, time::Duration};
use kcp::Kcp;

/// KCP NoDelay configuration
#[derive(Debug, Clone, Copy)]
pub struct KcpNoDelayConfig {
    /// Enable nodelay
    pub nodelay: bool,
    /// Internal update interval (ms)
    pub interval: i32,
    /// ACK number to enable fast resend
    pub resend: i32,
    /// Disable congestion control
    pub nc: bool,
}

impl Default for KcpNoDelayConfig {
    fn default() -> Self {
        Self {
            nodelay: false,
            interval: 100,
            resend: 0,
            nc: false,
        }
    }
}

impl KcpNoDelayConfig {
    /// Fastest configuration for low latency
    pub const fn fastest() -> Self {
        Self {
            nodelay: true,
            interval: 10,
            resend: 2,
            nc: true,
        }
    }

    /// Normal configuration for balanced performance
    pub const fn normal() -> Self {
        Self {
            nodelay: false,
            interval: 40,
            resend: 0,
            nc: false,
        }
    }

    /// Optimized configuration for interactive applications
    /// - Low latency for interactive response
    /// - Aggressive retransmission for reliability
    /// - Disabled congestion control for consistent performance
    pub const fn optimized() -> Self {
        Self {
            nodelay: true,
            interval: 20,  // 20ms update interval - balance between CPU and latency
            resend: 2,     // Fast resend after 2 duplicate ACKs
            nc: true,      // Disable congestion control for consistent performance
        }
    }

    /// Configuration for high-latency networks (satellite, cellular)
    pub const fn high_latency() -> Self {
        Self {
            nodelay: true,
            interval: 30,  // Slightly higher interval for high-latency links
            resend: 1,     // More conservative resend
            nc: false,     // Enable congestion control for lossy links
        }
    }
}

/// KCP configuration
#[derive(Debug, Clone, Copy)]
pub struct KcpConfig {
    /// Maximum Transmission Unit
    pub mtu: usize,
    /// NoDelay configuration
    pub nodelay: KcpNoDelayConfig,
    /// Send and receive window size (send, recv)
    pub wnd_size: (u16, u16),
    /// Session expire duration
    pub session_expire: Option<Duration>,
    /// Stream mode
    pub stream: bool,
}

impl Default for KcpConfig {
    fn default() -> Self {
        Self {
            mtu: 1400,
            nodelay: KcpNoDelayConfig::normal(),
            wnd_size: (256, 256),
            session_expire: Some(Duration::from_secs(90)),
            stream: false,
        }
    }
}

impl KcpConfig {
    /// Apply configuration to KCP instance
    pub fn apply_config<W: Write>(&self, kcp: &mut Kcp<W>) {
        kcp.set_mtu(self.mtu).expect("invalid MTU");
        kcp.set_nodelay(
            self.nodelay.nodelay,
            self.nodelay.interval,
            self.nodelay.resend,
            self.nodelay.nc,
        );
        kcp.set_wndsize(self.wnd_size.0, self.wnd_size.1);
    }

    /// Optimized configuration for local networks (LAN/WiFi)
    /// - Optimized for low latency and high throughput
    /// - Large windows for bulk data transfer
    /// - Stream mode for continuous data flow
    pub fn lan() -> Self {
        Self {
            mtu: 1400,
            nodelay: KcpNoDelayConfig::optimized(),
            wnd_size: (512, 512), // Larger windows for high throughput
            session_expire: Some(Duration::from_secs(300)), // 5 min timeout
            stream: true, // Stream mode for continuous data flow
        }
    }

    /// Optimized configuration for WAN/Internet connections
    /// - Balanced latency and reliability
    /// - Conservative window sizes for variable bandwidth
    pub fn wan() -> Self {
        Self {
            mtu: 1200, // Smaller MTU to avoid fragmentation
            nodelay: KcpNoDelayConfig::optimized(),
            wnd_size: (256, 256), // Conservative windows
            session_expire: Some(Duration::from_secs(180)), // 3 min timeout
            stream: true,
        }
    }

    /// Configuration for high-latency/lossy networks (satellite, cellular)
    pub fn high_latency() -> Self {
        Self {
            mtu: 1000, // Even smaller MTU for problematic links
            nodelay: KcpNoDelayConfig::high_latency(),
            wnd_size: (128, 256), // Smaller send window, larger receive
            session_expire: Some(Duration::from_secs(600)), // 10 min timeout
            stream: true,
        }
    }

    /// Configuration for low-bandwidth connections
    pub fn low_bandwidth() -> Self {
        Self {
            mtu: 800, // Small MTU for efficiency
            nodelay: KcpNoDelayConfig {
                nodelay: true,
                interval: 50, // Higher interval to reduce overhead
                resend: 1,
                nc: false, // Enable congestion control
            },
            wnd_size: (64, 128), // Small windows
            session_expire: Some(Duration::from_secs(300)),
            stream: true,
        }
    }
}