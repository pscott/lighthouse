use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Defines the encoding for the API.
#[derive(Clone, Serialize, Deserialize, Copy)]
pub enum ApiEncodingFormat {
    JSON,
    YAML,
    SSZ,
}

impl ApiEncodingFormat {
    pub fn get_content_type(&self) -> &str {
        match self {
            ApiEncodingFormat::JSON => "application/json",
            ApiEncodingFormat::YAML => "application/yaml",
            ApiEncodingFormat::SSZ => "application/ssz",
        }
    }
}

impl From<&str> for ApiEncodingFormat {
    fn from(f: &str) -> ApiEncodingFormat {
        match f {
            "application/yaml" => ApiEncodingFormat::YAML,
            "application/ssz" => ApiEncodingFormat::SSZ,
            _ => ApiEncodingFormat::JSON,
        }
    }
}

/// HTTP REST API Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Enable the REST API server.
    pub enabled: bool,
    /// The IPv4 address the REST API HTTP server will listen on.
    pub listen_address: Ipv4Addr,
    /// The port the REST API HTTP server will listen on.
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            listen_address: Ipv4Addr::new(127, 0, 0, 1),
            port: 5052,
        }
    }
}

impl Config {
    pub fn apply_cli_args(&mut self, args: &ArgMatches) -> Result<(), &'static str> {
        if args.is_present("no-api") {
            self.enabled = false;
        }

        if let Some(rpc_address) = args.value_of("api-address") {
            self.listen_address = rpc_address
                .parse::<Ipv4Addr>()
                .map_err(|_| "api-address is not a valid IPv4 address.")?;
        }

        if let Some(rpc_port) = args.value_of("api-port") {
            self.port = rpc_port
                .parse::<u16>()
                .map_err(|_| "api-port is not a valid u16.")?;
        }

        Ok(())
    }
}
