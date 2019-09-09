//! HTTP server settings.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs as _};

use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

/// HTTP server settings.
#[derive(Clone, Debug, Deserialize, Serialize, SmartDefault)]
#[serde(default)]
pub struct Client {
    /// Public URI of server. Address for exposed [Client API].
    ///
    /// This address will be returned from [Control API] in `sids` and to
    /// this address will connect [Jason] for start session.
    ///
    /// Defaults to `ws://0.0.0.0:8080`.
    ///
    /// [Client API]: http://tiny.cc/c80uaz
    /// [Jason]: https://github.com/instrumentisto/medea/tree/master/jason
    #[default("ws://0.0.0.0:8080".to_string())]
    pub public_url: String,

    /// IP address to bind HTTP server to. Defaults to `0.0.0.0`.
    #[default(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))]
    pub bind_ip: IpAddr,

    /// Port to bind HTTP server to. Defaults to `8080`.
    #[default(8080)]
    pub bind_port: u16,

    /// Path to directory with static control API specs.
    pub static_specs_path: Option<String>,
}

impl Client {
    /// Builds [`SocketAddr`] from `bind_ip` and `bind_port`.
    #[inline]
    pub fn bind_addr(&self) -> SocketAddr {
        (self.bind_ip, self.bind_port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    }
}

#[cfg(test)]
mod server_spec {
    use std::env;

    use serial_test_derive::serial;

    use crate::conf::Conf;

    use super::*;

    #[test]
    #[serial]
    fn overrides_defaults_and_gets_bind_addr() {
        let default_conf = Conf::default();

        env::set_var("MEDEA_CLIENT.BIND_IP", "5.5.5.5");
        env::set_var("MEDEA_CLIENT.BIND_PORT", "1234");

        let env_conf = Conf::parse().unwrap();

        env::remove_var("MEDEA_CLIENT.BIND_IP");
        env::remove_var("MEDEA_CLIENT.BIND_PORT");

        assert_ne!(default_conf.client.bind_ip, env_conf.client.bind_ip);
        assert_ne!(default_conf.client.bind_port, env_conf.client.bind_port);

        assert_eq!(env_conf.client.bind_ip, Ipv4Addr::new(5, 5, 5, 5));
        assert_eq!(env_conf.client.bind_port, 1234);
        assert_eq!(
            env_conf.client.bind_addr(),
            "5.5.5.5:1234".parse().unwrap(),
        );
    }
}
