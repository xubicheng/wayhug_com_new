use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::util::now_utc;

/// 邮件发送。SMTP 已配置时真正投递；未配置时只把全文写入 mail_outbox（status=logged），
/// 并在标准日志里打印，绝不伪装成"已发送"。
pub struct Mailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Mailbox,
    db: SqlitePool,
    pub smtp_enabled: bool,
}

impl Mailer {
    pub fn new(cfg: &Config, db: SqlitePool) -> Self {
        let transport = if cfg.smtp.enabled {
            let creds = Credentials::new(cfg.smtp.user.clone(), cfg.smtp.pass.clone());
            AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp.host)
                .ok()
                .map(|b| b.port(cfg.smtp.port).credentials(creds).build())
        } else {
            None
        };
        let from = cfg
            .smtp
            .from
            .parse::<Mailbox>()
            .unwrap_or_else(|_| "wayhug.com <noreply@wayhug.com>".parse().unwrap());
        Self {
            transport,
            from,
            db,
            smtp_enabled: cfg.smtp.enabled,
        }
    }

    pub async fn send(&self, kind: &str, to: &str, subject: &str, body: &str) {
        let mut status = "logged";
        let mut error: Option<String> = None;

        if let Some(transport) = &self.transport {
            let built = Message::builder()
                .from(self.from.clone())
                .to(to.parse::<Mailbox>().unwrap_or_else(|_| {
                    "unknown@invalid".parse::<Mailbox>().unwrap()
                }))
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string());
            match built {
                Ok(msg) => match transport.send(msg).await {
                    Ok(_) => status = "sent",
                    Err(e) => {
                        status = "failed";
                        error = Some(e.to_string());
                    }
                },
                Err(e) => {
                    status = "failed";
                    error = Some(e.to_string());
                }
            }
        }

        if status == "logged" {
            tracing::info!(kind, to, subject, "\n--- mail (SMTP 未接入，已落盘) ---\n{}\n---", body);
        }

        let _ = sqlx::query(
            "INSERT INTO mail_outbox (to_email, subject, body, kind, status, error, created_at, sent_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(to)
        .bind(subject)
        .bind(body)
        .bind(kind)
        .bind(status)
        .bind(error)
        .bind(now_utc())
        .bind(if status == "sent" { Some(now_utc()) } else { None })
        .execute(&self.db)
        .await;
    }
}
