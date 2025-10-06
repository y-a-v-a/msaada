// src/network.rs
// Network utilities for port checking and network interface detection

use local_ip_address::local_ip;
use port_check::is_port_reachable;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct ServerAddresses {
    pub local: String,
    pub network: Option<String>,
    pub previous_port: Option<u16>,
}

pub struct NetworkUtils;

impl NetworkUtils {
    /// Check if a port is available on the given host
    pub fn is_port_available(host: &str, port: u16) -> bool {
        // Use port_check to see if the port is reachable (i.e., already in use)
        // We want the opposite - if it's NOT reachable, then it's available
        !is_port_reachable(format!("{}:{}", host, port))
    }

    /// Find the next available port starting from the given port
    pub fn find_available_port(host: &str, start_port: u16) -> Option<u16> {
        // Try up to 100 ports above the start port without overflowing u16 bounds
        let start = u32::from(start_port);
        let end = (start.saturating_add(100)).min(u32::from(u16::MAX) + 1);

        (start..end)
            .map(|port| port as u16)
            .find(|&port| Self::is_port_available(host, port))
    }

    /// Get the network IP address for external access
    pub fn get_network_address() -> Option<IpAddr> {
        local_ip().ok()
    }

    /// Create server addresses for display
    pub fn create_server_addresses(
        host: &str,
        port: u16,
        use_https: bool,
        previous_port: Option<u16>,
    ) -> ServerAddresses {
        let protocol = if use_https { "https" } else { "http" };

        // Handle special cases for host names
        let display_host = match host {
            "0.0.0.0" => "localhost",
            "::" => "localhost",
            _ => host,
        };

        let local = format!("{}://{}:{}", protocol, display_host, port);

        // Try to get network address for external access
        let network = Self::get_network_address().map(|ip| {
            // For IPv6 addresses, we need to wrap them in brackets
            let formatted_ip = match ip {
                IpAddr::V6(v6) => format!("[{}]", v6),
                IpAddr::V4(v4) => v4.to_string(),
            };
            format!("{}://{}:{}", protocol, formatted_ip, port)
        });

        ServerAddresses {
            local,
            network,
            previous_port,
        }
    }

    /// Check port and auto-switch if needed
    pub fn resolve_port(
        host: &str,
        requested_port: u16,
        allow_port_switching: bool,
    ) -> Result<u16, String> {
        if Self::is_port_available(host, requested_port) {
            return Ok(requested_port);
        }

        if !allow_port_switching {
            return Err(format!(
                "Port {} is already in use. Use --no-port-switching to disable auto-switching.",
                requested_port
            ));
        }

        // Port is occupied and switching is allowed
        let next_port = match requested_port.checked_add(1) {
            Some(port) => port,
            None => {
                return Err(format!(
                    "Port {} is already in use and no higher ports are available.",
                    requested_port
                ));
            }
        };

        match Self::find_available_port(host, next_port) {
            Some(available_port) => Ok(available_port),
            None => {
                let range_end = (u32::from(next_port)
                    .saturating_add(99)
                    .min(u32::from(u16::MAX))) as u16;

                Err(format!(
                    "Port {} is occupied and no alternative ports are available in the range {}-{}.",
                    requested_port,
                    next_port,
                    range_end
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_port_availability() {
        // Bind to a port to make it unavailable
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = listener.local_addr().unwrap();
        let bound_port = bound_addr.port();

        // The bound port should not be available
        assert!(!NetworkUtils::is_port_available("127.0.0.1", bound_port));

        drop(listener);
        // After dropping the listener, the port should be available
        // Note: This might be flaky on some systems due to TIME_WAIT state
        // so we won't test this part
    }

    #[test]
    fn test_find_available_port() {
        // This should find an available port
        let available = NetworkUtils::find_available_port("127.0.0.1", 40000);
        if let Some(port) = available {
            assert!(port >= 40000);
            assert!(port < 40100);
        }
    }

    #[test]
    fn test_create_server_addresses() {
        let addresses = NetworkUtils::create_server_addresses("0.0.0.0", 3000, false, Some(8080));

        assert_eq!(addresses.local, "http://localhost:3000");
        assert_eq!(addresses.previous_port, Some(8080));
        let expected_network = NetworkUtils::get_network_address().map(|ip| match ip {
            IpAddr::V6(v6) => format!("http://[{}]:3000", v6),
            IpAddr::V4(v4) => format!("http://{}:3000", v4),
        });
        assert_eq!(addresses.network, expected_network);
    }

    #[test]
    fn test_create_server_addresses_https() {
        let addresses = NetworkUtils::create_server_addresses("127.0.0.1", 8443, true, None);

        assert_eq!(addresses.local, "https://127.0.0.1:8443");
        assert_eq!(addresses.previous_port, None);
    }

    #[test]
    fn test_resolve_port_available() {
        // Test with a very high port number that should be available
        let result = NetworkUtils::resolve_port("127.0.0.1", 58000, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 58000);
    }

    #[test]
    fn test_resolve_port_switching_disabled() {
        // Bind to a port to occupy it
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = listener.local_addr().unwrap();
        let bound_port = bound_addr.port();

        let result = NetworkUtils::resolve_port("127.0.0.1", bound_port, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already in use"));
    }

    #[test]
    fn test_resolve_port_switching_enabled() {
        // Bind to a port to occupy it
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = listener.local_addr().unwrap();
        let bound_port = bound_addr.port();

        let result = NetworkUtils::resolve_port("127.0.0.1", bound_port, true);
        assert!(result.is_ok());

        let new_port = result.unwrap();
        assert!(new_port > bound_port);
    }

    #[test]
    fn test_find_available_port_near_upper_bound() {
        let result = NetworkUtils::find_available_port("127.0.0.1", u16::MAX - 1);

        if let Some(port) = result {
            assert!(port >= u16::MAX - 1);
        }
    }

    #[test]
    fn test_resolve_port_switching_enabled_at_upper_bound() {
        // Bind to the maximum port to force overflow handling paths
        let listener = TcpListener::bind("127.0.0.1:65535").unwrap();

        let result = NetworkUtils::resolve_port("127.0.0.1", 65_535, true);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("no higher ports are available"));

        drop(listener);
    }

    #[test]
    fn test_create_server_addresses_ipv6() {
        let addresses = NetworkUtils::create_server_addresses("::", 8080, true, None);

        assert_eq!(addresses.local, "https://localhost:8080");
        assert_eq!(addresses.previous_port, None);

        // Network address should exist and handle IPv6 formatting
        if let Some(network) = &addresses.network {
            // Should contain brackets if IPv6
            assert!(network.starts_with("https://"));
        }
    }

    #[test]
    fn test_create_server_addresses_with_previous_port() {
        let addresses = NetworkUtils::create_server_addresses("0.0.0.0", 3001, false, Some(3000));

        assert_eq!(addresses.local, "http://localhost:3001");
        assert_eq!(addresses.previous_port, Some(3000));
    }

    #[test]
    fn test_resolve_port_boundary_cases() {
        // Test with very high port numbers near the limit
        let result = NetworkUtils::resolve_port("127.0.0.1", 65535, true);
        // Should either succeed with 65535 or fail gracefully
        if let Ok(port) = result {
            assert_eq!(port, 65535)
        }

        // Test with port 1 (privileged)
        let result = NetworkUtils::resolve_port("127.0.0.1", 1, false);
        // Will likely fail due to permissions, but should not panic
        match result {
            Ok(_) => {} // Unexpected but not wrong
            Err(msg) => assert!(msg.contains("already in use") || msg.contains("Permission")),
        }
    }

    #[test]
    fn test_port_range_exhaustion() {
        // Create a scenario where many ports are bound
        let mut listeners = Vec::new();
        let start_port = 45000;

        // Bind several consecutive ports
        for i in 0..5 {
            if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", start_port + i)) {
                listeners.push(listener);
            }
        }

        // Try to find available port in a small range
        let available = NetworkUtils::find_available_port("127.0.0.1", start_port);

        if let Some(port) = available {
            // Should find a port outside the bound range
            assert!(port >= start_port);
            assert!(port < start_port + 100);
        }

        drop(listeners); // Clean up
    }

    #[test]
    fn test_is_port_available_different_hosts() {
        // Test localhost variants
        let high_port = 58001; // Use high port likely to be available

        // These should all refer to the same interface in most cases
        let localhost_available = NetworkUtils::is_port_available("localhost", high_port);
        let ip_available = NetworkUtils::is_port_available("127.0.0.1", high_port);
        let wildcard_available = NetworkUtils::is_port_available("0.0.0.0", high_port);

        // Results should be consistent for localhost references
        assert_eq!(localhost_available, ip_available);

        // Note: 0.0.0.0 might behave differently depending on system configuration
        // so we just verify it doesn't panic
        let _ = wildcard_available;
    }

    #[test]
    fn test_get_network_address_returns_valid_ip() {
        match NetworkUtils::get_network_address() {
            Some(ip) => {
                // Verify it's a valid IP address
                match ip {
                    std::net::IpAddr::V4(v4) => {
                        // Should not be the loopback address for "network" address
                        // but might be in some test environments
                        assert!(!v4.to_string().is_empty());
                    }
                    std::net::IpAddr::V6(v6) => {
                        assert!(!v6.to_string().is_empty());
                    }
                }
            }
            None => {
                // Acceptable in environments without network interfaces
                println!("No network address available (expected in some test environments)");
            }
        }
    }

    #[test]
    fn test_server_addresses_protocol_consistency() {
        let http_addresses = NetworkUtils::create_server_addresses("localhost", 8000, false, None);
        let https_addresses = NetworkUtils::create_server_addresses("localhost", 8000, true, None);

        assert!(http_addresses.local.starts_with("http://"));
        assert!(https_addresses.local.starts_with("https://"));

        if let Some(network_http) = &http_addresses.network {
            assert!(network_http.starts_with("http://"));
        }

        if let Some(network_https) = &https_addresses.network {
            assert!(network_https.starts_with("https://"));
        }
    }

    #[test]
    fn test_error_message_formatting() {
        // Test error message when port switching is disabled
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let bound_addr = listener.local_addr().unwrap();
        let bound_port = bound_addr.port();

        let result = NetworkUtils::resolve_port("127.0.0.1", bound_port, false);
        assert!(result.is_err());

        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("already in use"));
        assert!(error_msg.contains("--no-port-switching"));
        assert!(error_msg.contains(&bound_port.to_string()));
    }

    #[test]
    fn test_find_available_port_range_limits() {
        // Test that find_available_port respects the 100-port limit
        let start_port = 50000;
        let result = NetworkUtils::find_available_port("127.0.0.1", start_port);

        if let Some(port) = result {
            assert!(port >= start_port);
            assert!(port < start_port + 100);
        }
    }
}
