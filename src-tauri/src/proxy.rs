use crate::db::Database;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub proxy_type: String,   // "http" | "socks5"
    pub host: String,
    pub port: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Build the full proxy URL, e.g. "http://user:pass@127.0.0.1:7890"
    pub fn to_url(&self) -> String {
        let scheme = &self.proxy_type;
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                format!("{}://{}:{}@{}:{}", scheme, user, pass, self.host, self.port)
            }
            _ => {
                format!("{}://{}:{}", scheme, self.host, self.port)
            }
        }
    }

    /// Load proxy config from app_settings. Returns None if disabled or not configured.
    pub fn load(db: &Database) -> Option<Self> {
        let enabled = db.get_setting("proxy_enabled").ok()??;
        if enabled != "true" {
            return None;
        }

        let proxy_type = db.get_setting("proxy_type").ok()??;
        let host = db.get_setting("proxy_host").ok()??;
        let port = db.get_setting("proxy_port").ok()??;

        if host.is_empty() || port.is_empty() {
            return None;
        }

        let username = db.get_setting("proxy_username").ok().flatten()
            .filter(|s| !s.is_empty());
        let password = db.get_setting("proxy_password").ok().flatten()
            .filter(|s| !s.is_empty());

        Some(ProxyConfig {
            enabled: true,
            proxy_type,
            host,
            port,
            username,
            password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proxy(proxy_type: &str, host: &str, port: &str, username: Option<&str>, password: Option<&str>) -> ProxyConfig {
        ProxyConfig {
            enabled: true,
            proxy_type: proxy_type.to_string(),
            host: host.to_string(),
            port: port.to_string(),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_to_url_http_no_auth() {
        let proxy = make_proxy("http", "127.0.0.1", "7890", None, None);
        assert_eq!(proxy.to_url(), "http://127.0.0.1:7890");
    }

    #[test]
    fn test_to_url_http_with_auth() {
        let proxy = make_proxy("http", "proxy.example.com", "8080", Some("user"), Some("pass"));
        assert_eq!(proxy.to_url(), "http://user:pass@proxy.example.com:8080");
    }

    #[test]
    fn test_to_url_socks5() {
        let proxy = make_proxy("socks5", "127.0.0.1", "1080", None, None);
        assert_eq!(proxy.to_url(), "socks5://127.0.0.1:1080");
    }

    #[test]
    fn test_to_url_socks5_with_auth() {
        let proxy = make_proxy("socks5", "10.0.0.1", "1080", Some("admin"), Some("secret"));
        assert_eq!(proxy.to_url(), "socks5://admin:secret@10.0.0.1:1080");
    }

    #[test]
    fn test_load_returns_none_when_disabled() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("proxy_enabled", "false").unwrap();
        db.set_setting("proxy_type", "http").unwrap();
        db.set_setting("proxy_host", "127.0.0.1").unwrap();
        db.set_setting("proxy_port", "7890").unwrap();
        let result = ProxyConfig::load(&db);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_returns_none_when_not_configured() {
        let db = Database::new_in_memory().unwrap();
        // proxy_enabled not set at all
        let result = ProxyConfig::load(&db);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_returns_config_when_enabled() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("proxy_enabled", "true").unwrap();
        db.set_setting("proxy_type", "http").unwrap();
        db.set_setting("proxy_host", "127.0.0.1").unwrap();
        db.set_setting("proxy_port", "7890").unwrap();

        let result = ProxyConfig::load(&db);
        assert!(result.is_some());
        let config = result.unwrap();
        assert!(config.enabled);
        assert_eq!(config.proxy_type, "http");
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, "7890");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
    }

    #[test]
    fn test_load_with_auth() {
        let db = Database::new_in_memory().unwrap();
        db.set_setting("proxy_enabled", "true").unwrap();
        db.set_setting("proxy_type", "socks5").unwrap();
        db.set_setting("proxy_host", "10.0.0.1").unwrap();
        db.set_setting("proxy_port", "1080").unwrap();
        db.set_setting("proxy_username", "admin").unwrap();
        db.set_setting("proxy_password", "secret").unwrap();

        let result = ProxyConfig::load(&db);
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.proxy_type, "socks5");
        assert_eq!(config.username, Some("admin".to_string()));
        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.to_url(), "socks5://admin:secret@10.0.0.1:1080");
    }
}
