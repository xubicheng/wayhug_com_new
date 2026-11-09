use std::env;

#[derive(Clone)]
pub struct Smtp {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub from: String,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub site_url: String,
    pub contact_email: String,
    pub admin_user: String,
    pub admin_password: String,
    pub cookie_secure: bool,
    pub database_url: String,
    pub smtp: Smtp,
}

fn var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> anyhow::Result<Config> {
        let admin_password = env::var("ADMIN_PASSWORD").unwrap_or_default();
        if admin_password.len() < 8 {
            anyhow::bail!("ADMIN_PASSWORD 未设置或短于 8 位；请在 .env 中配置");
        }
        let smtp_host = var("SMTP_HOST", "");
        let smtp_user = var("SMTP_USER", "");
        let smtp_pass = var("SMTP_PASS", "");
        let mail_enabled = var("MAIL_ENABLED", "1") == "1"
            && !smtp_host.is_empty()
            && !smtp_user.is_empty()
            && !smtp_pass.is_empty();

        Ok(Config {
            bind: var("BIND", "127.0.0.1"),
            port: var("PORT", "3005").parse().unwrap_or(3005),
            site_url: var("SITE_URL", "https://wayhug.com"),
            contact_email: var("CONTACT_EMAIL", "xubicheng@qq.com"),
            admin_user: var("ADMIN_USER", "admin"),
            admin_password,
            cookie_secure: var("COOKIE_SECURE", "1") == "1",
            database_url: var("DATABASE_URL", "sqlite://data/wayhug.db"),
            smtp: Smtp {
                host: smtp_host,
                port: var("SMTP_PORT", "465").parse().unwrap_or(465),
                user: smtp_user,
                pass: smtp_pass,
                from: var("SMTP_FROM", "wayhug.com <xubicheng@qq.com>"),
                enabled: mail_enabled,
            },
        })
    }
}
