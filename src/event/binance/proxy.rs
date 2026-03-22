use std::net::SocketAddr;

#[derive(Clone, Debug, Default)]
pub struct ProxyConfig {
    pub socks5: Option<Socks5Config>,
}

#[derive(Clone, Debug)]
pub struct Socks5Config {
    pub addr: SocketAddr,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    /// 从环境变量构建
    /// 支持 SOCKS5_PROXY=socks5://user:pass@host:port
    /// 或 SOCKS5_PROXY=socks5://host:port
    pub fn from_env() -> Option<Self> {
        let uri = std::env::var("SOCKS5_PROXY").ok()?;

        let uri = uri.trim_start_matches("socks5://");
        let (creds, host_port) = uri.split_once('@').unwrap_or(("", uri));

        let (username, password) = if !creds.is_empty() {
            let (u, p) = creds.split_once(':')?;
            (Some(u.to_string()), Some(p.to_string()))
        } else {
            (None, None)
        };

        let addr: SocketAddr = host_port.parse().ok()?;

        Some(Self {
            socks5: Some(Socks5Config {
                addr,
                username,
                password,
            }),
        })
    }

    /// 转换为 reqwest::Proxy URL 字符串
    pub fn to_reqwest_proxy_url(&self) -> Option<String> {
        let socks5 = self.socks5.as_ref()?;
        let addr = socks5.addr.to_string();

        let url = if let (Some(u), Some(p)) = (&socks5.username, &socks5.password) {
            format!("socks5://{u}:{p}@{addr}")
        } else {
            format!("socks5://{addr}")
        };

        Some(url)
    }
}
