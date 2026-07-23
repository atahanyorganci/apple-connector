use std::{
    io::{Error as IoError, ErrorKind},
    net::IpAddr,
    path::PathBuf,
};

use clap::Parser;

const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 3000;

#[derive(Debug, Parser)]
#[command(name = "apple-connector", version, about)]
pub struct Cli {
    /// IP address to bind the HTTP server to.
    #[arg(long, default_value = DEFAULT_ADDRESS, value_parser = parse_address)]
    pub address: IpAddr,

    /// TCP port to bind the HTTP server to.
    #[arg(long, default_value_t = DEFAULT_PORT, value_parser = parse_port)]
    pub port: u16,

    /// Path to the read-only Messages `chat.db` database.
    #[arg(long, value_name = "PATH")]
    pub messages_database: Option<PathBuf>,

    /// Path to the read-only Reminders SQLite store. Auto-discovered when omitted.
    #[arg(long, value_name = "PATH")]
    pub reminders_database: Option<PathBuf>,

    /// Directory to scan for Reminders store files when auto-discovering.
    #[arg(long, value_name = "PATH")]
    pub reminders_stores_dir: Option<PathBuf>,

    /// Path to the Messages attachment directory. Defaults to `Attachments` next to `chat.db`.
    #[arg(long, value_name = "PATH")]
    pub attachment_root: Option<PathBuf>,

    /// Path to the Reminders attachment support directory.
    #[arg(long, value_name = "PATH")]
    pub reminders_attachment_root: Option<PathBuf>,
}

impl Cli {
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    pub fn messages_database_path(&self) -> Result<PathBuf, IoError> {
        if let Some(path) = &self.messages_database {
            return Ok(path.clone());
        }
        if let Ok(path) = std::env::var("APPLE_CONNECTOR_MESSAGES_DATABASE") {
            return Ok(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("APPLE_CONNECTOR_DATABASE") {
            eprintln!(
                "warning: APPLE_CONNECTOR_DATABASE is deprecated; use APPLE_CONNECTOR_MESSAGES_DATABASE"
            );
            return Ok(PathBuf::from(path));
        }
        default_messages_database_path()
    }

    pub fn reminders_stores_dir_path(&self) -> Result<PathBuf, IoError> {
        match &self.reminders_stores_dir {
            Some(path) => Ok(path.clone()),
            None => crate::reminders::discovery::default_reminders_stores_dir(),
        }
    }

    pub fn attachment_root_path(&self) -> Result<PathBuf, IoError> {
        if let Some(path) = &self.attachment_root {
            return Ok(path.clone());
        }

        Ok(crate::messages::attachment_path::default_attachment_root(
            &self.messages_database_path()?,
        ))
    }

    pub fn warns_about_public_binding(&self) -> bool {
        matches!(self.address, IpAddr::V4(addr) if addr.is_unspecified())
    }
}

pub fn default_messages_database_path() -> Result<PathBuf, IoError> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| IoError::new(ErrorKind::NotFound, "HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Messages/chat.db"))
}

#[deprecated(note = "use default_messages_database_path instead")]
pub fn default_database_path() -> Result<PathBuf, IoError> {
    default_messages_database_path()
}

fn parse_address(value: &str) -> Result<IpAddr, String> {
    let address = value
        .parse::<IpAddr>()
        .map_err(|error| format!("invalid IP address `{value}`: {error}"))?;
    validate_address(address)?;
    Ok(address)
}

fn validate_address(address: IpAddr) -> Result<(), String> {
    match address {
        IpAddr::V4(v4) if v4.is_loopback() || v4.is_unspecified() => Ok(()),
        IpAddr::V6(v6) if v6.is_loopback() => Ok(()),
        _ => Err(format!(
            "address {address} is not allowed; use a loopback address or 0.0.0.0"
        )),
    }
}

fn parse_port(value: &str) -> Result<u16, String> {
    let port = value
        .parse::<u16>()
        .map_err(|error| format!("invalid port `{value}`: {error}"))?;
    if port == 0 {
        return Err("port must be greater than 0".to_owned());
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        path::PathBuf,
    };

    use clap::Parser;

    use super::{Cli, default_messages_database_path};

    #[test]
    fn defaults_to_loopback_port_and_home_database() {
        let cli = Cli::try_parse_from(["apple-connector"]).expect("parse defaults");

        assert_eq!(cli.address, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(cli.port, 3000);
        assert_eq!(
            cli.messages_database_path().expect("database path"),
            default_messages_database_path().expect("default database path")
        );
    }

    #[test]
    fn address_port_and_database_override_independently() {
        let cli = Cli::try_parse_from([
            "apple-connector",
            "--address",
            "127.0.0.2",
            "--port",
            "8080",
            "--messages-database",
            "/tmp/chat.db",
        ])
        .expect("parse overrides");

        assert_eq!(cli.address, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        assert_eq!(cli.port, 8080);
        assert_eq!(
            cli.messages_database_path().expect("database path"),
            PathBuf::from("/tmp/chat.db")
        );
    }

    #[test]
    fn accepts_ipv6_loopback_and_all_interfaces_ipv4() {
        let cli_v6 = Cli::try_parse_from(["apple-connector", "--address", "::1"])
            .expect("parse ipv6 loopback");
        assert!(matches!(cli_v6.address, IpAddr::V6(_)));

        let cli_v4 = Cli::try_parse_from(["apple-connector", "--address", "0.0.0.0"])
            .expect("parse all interfaces");
        assert_eq!(cli_v4.address, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert!(cli_v4.warns_about_public_binding());
    }

    #[test]
    fn rejects_invalid_address_port_and_malformed_args() {
        let invalid_ip =
            Cli::try_parse_from(["apple-connector", "--address", "not-an-ip"]).unwrap_err();
        assert!(invalid_ip.to_string().contains("invalid IP address"));

        let public_ip =
            Cli::try_parse_from(["apple-connector", "--address", "192.168.1.1"]).unwrap_err();
        assert!(public_ip.to_string().contains("not allowed"));

        let zero_port = Cli::try_parse_from(["apple-connector", "--port", "0"]).unwrap_err();
        assert!(zero_port.to_string().contains("greater than 0"));

        let unknown_flag = Cli::try_parse_from(["apple-connector", "--unknown"]).unwrap_err();
        assert!(unknown_flag.to_string().contains("unexpected"));
    }
}
