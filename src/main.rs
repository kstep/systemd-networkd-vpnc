use serde_derive::Deserialize;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

const NETWORKCTL: &str = "networkctl";
const DEFAULT_TUNDEV: &str = "tun0";
const SYSTEMD_NETWORKD_CONFIG_DIR: &str = "/etc/systemd/network/";

struct Changed(bool);

impl Changed {
    fn yes() -> Changed {
        Changed(true)
    }
    fn no() -> Changed {
        Changed(false)
    }
}

struct Networkctl {
    bin: PathBuf,
}

impl Networkctl {
    fn new() -> Networkctl {
        Networkctl {
            bin: find_bin_file(NETWORKCTL).expect("`networkctl` not found"),
        }
    }

    fn reload(&self) -> Result<(), std::io::Error> {
        std::process::Command::new(&self.bin)
            .arg("reload")
            .status()
            .map(|_| ())
    }
}

#[derive(Debug, Deserialize)]
struct Route {
    addr: String,
    mask: String,
    masklen: u8,
    #[serde(default = "Route::default_protocol")]
    protocol: u8,
    #[serde(default = "Route::default_port")]
    sport: u16,
    #[serde(default = "Route::default_port")]
    dport: u16,
}

impl Route {
    fn default_protocol() -> u8 {
        0
    }
    fn default_port() -> u16 {
        0
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Config {
    reason: Reason,
    vpngateway: String,
    #[serde(default = "Config::default_tundev")]
    tundev: String,

    #[serde(rename = "internal_ip4_address")]
    address: String,
    #[serde(rename = "internal_ip4_mtu")]
    mtu: Option<u32>,
    #[serde(rename = "internal_ip4_netmask")]
    netmask: Option<String>,
    #[serde(
        rename = "internal_ip4_netmasklen",
        default = "Config::default_netmasklen"
    )]
    netmasklen: u8,
    #[serde(rename = "internal_ip4_netaddr")]
    netaddr: Option<String>,
    #[serde(rename = "internal_ip4_dns")]
    dns: Option<String>,
    #[serde(rename = "internal_ip4_nbns")]
    nbns: Option<String>,

    #[serde(rename = "cisco_def_domain")]
    def_domain: String,
    #[serde(rename = "cisco_banner")]
    banner: Option<String>,
    #[serde(rename = "cisco_split_inc", default = "Config::default_split_routes")]
    split_routes_inc: usize,
}

impl Config {
    fn default_netmasklen() -> u8 {
        32
    }
    fn default_split_routes() -> usize {
        0
    }
    fn default_tundev() -> String {
        String::from(DEFAULT_TUNDEV)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Reason {
    Connect,
    Disconnect,
    PreInit,
    AttemptReconnect,
    Reconnect,
}

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Env(envy::Error),
}

impl From<envy::Error> for Error {
    fn from(err: envy::Error) -> Self {
        Error::Env(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

fn main() -> Result<(), Error> {
    if Process::new()?.run()?.0 {
        Networkctl::new().reload()?;
    }
    Ok(())
}

struct Process {
    config: Config,
    split_routes_inc: Vec<Route>,
    network_file: PathBuf,
}

impl Process {
    fn new() -> Result<Process, envy::Error> {
        let config = envy::from_env::<Config>()?;
        Ok(Process {
            network_file: PathBuf::from(SYSTEMD_NETWORKD_CONFIG_DIR)
                .join(&config.tundev)
                .with_extension("network"),
            split_routes_inc: (0..config.split_routes_inc)
                .map(|n| envy::prefixed(format!("CISCO_SPLIT_INC_{}_", n)).from_env::<Route>())
                .collect::<Result<Vec<_>, _>>()?,
            config,
        })
    }

    fn run(&self) -> Result<Changed, std::io::Error> {
        use Reason::*;
        match self.config.reason {
            PreInit => self.pre_init(),
            Connect => self.connect(),
            Disconnect => self.disconnect(),
            AttemptReconnect => self.attempt_reconnect(),
            Reconnect => self.reconnect(),
        }
    }

    fn reconnect(&self) -> Result<Changed, std::io::Error> {
        Ok(Changed::no())
    }

    fn pre_init(&self) -> Result<Changed, std::io::Error> {
        Ok(Changed::no())
    }

    fn attempt_reconnect(&self) -> Result<Changed, std::io::Error> {
        Ok(Changed::no())
    }

    fn disconnect(&self) -> Result<Changed, std::io::Error> {
        std::fs::remove_file(&self.network_file)?;
        Ok(Changed::yes())
    }

    fn connect(&self) -> Result<Changed, std::io::Error> {
        if let Some(ref banner) = self.config.banner {
            println!("Connect Banner:\n{}", banner);
        }

        if let Some(config_dir) = self.network_file.parent() {
            std::fs::create_dir_all(config_dir)?;
        }

        let mut file = std::fs::File::create(&self.network_file)?;

        writeln!(
            file,
            r#"
[Link]
MTUBytes={0}

[Address]
Address={1}/32

[Route]
Destination={1}/32
Gateway={1}
"#,
            self.config.mtu.unwrap_or(1412),
            self.config.address,
        )?;

        if self.config.netmask.is_some() {
            writeln!(
                file,
                r#"
[Route]
Destination={}/{}
Scope=link
"#,
                self.config.netaddr.as_deref().unwrap(),
                self.config.netmasklen
            )?;
        }

        let mut default_route = false;
        if self.config.split_routes_inc > 0 {
            for route in &self.split_routes_inc {
                if route.addr == "0.0.0.0" {
                    default_route = true;
                } else {
                    writeln!(
                        file,
                        r#"
[Route]
Scope=link
Destination={}/{}
"#,
                        route.addr, route.masklen
                    )?;
                }
            }
        } else {
            default_route = !self.config.address.is_empty();
        }

        writeln!(
            file,
            r#"
[Match]
Name={}

[Network]
Description=Cisco VPN to {}
DHCP=no
IPv6AcceptRA=no
"#,
            self.config.tundev, self.config.vpngateway
        )?;

        if default_route {
            writeln!(file, "DefaultRouteOnDevice=yes")?;
        }

        if let Some(ref dns) = self.config.dns {
            writeln!(file, "Domains={}", self.config.def_domain)?;

            for ns in dns.split_ascii_whitespace() {
                writeln!(file, "DNS={}", ns)?;
            }
        }
        Ok(Changed::yes())
    }
}

fn find_bin_file(file: &str) -> Option<PathBuf> {
    std::env::var("PATH").ok().and_then(|path| {
        path.split(':')
            .map(|p| Path::new(p).join(file))
            .find(|p| p.exists())
            .to_owned()
    })
}
