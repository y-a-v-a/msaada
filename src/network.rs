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
        // Try up to 100 ports above the start port
        for port in start_port..(start_port + 100) {
            if Self::is_port_available(host, port) {
                return Some(port);
            }
        }
        None
    }

    /// Get the network IP address for external access
    pub fn get_network_address() -> Option<IpAddr> {
        match local_ip() {
            Ok(ip) => Some(ip),
            Err(_) => None,
        }
    }

    /// Create server addresses for display
    pub fn create_server_addresses(
        host: &str, 
        port: u16, 
        use_https: bool, 
        previous_port: Option<u16>
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
        allow_port_switching: bool
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
        match Self::find_available_port(host, requested_port + 1) {
            Some(available_port) => Ok(available_port),
            None => Err(format!(
                "Port {} is occupied and no alternative ports are available in the range {}{}.",
                requested_port, requested_port + 1, requested_port + 100
            )),
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
        assert!(available.is_some());
        
        let port = available.unwrap();
        assert!(port >= 40000);
        assert!(port < 40100);
    }

    #[test]
    fn test_create_server_addresses() {
        let addresses = NetworkUtils::create_server_addresses(
            "0.0.0.0", 
            3000, 
            false, 
            Some(8080)
        );
        
        assert_eq!(addresses.local, "http://localhost:3000");
        assert_eq!(addresses.previous_port, Some(8080));
        // Network address depends on system, so we just check it exists
        assert!(addresses.network.is_some());
    }

    #[test]
    fn test_create_server_addresses_https() {
        let addresses = NetworkUtils::create_server_addresses(
            "127.0.0.1", 
            8443, 
            true, 
            None
        );
        
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
}