use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Clone)]
pub struct NoteCard {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub published_at: Option<String>,
    pub content_lang: String,
    pub has_lang: i64,
    pub author_name: Option<String>,
    pub guest_author_id: Option<i64>,
}

#[derive(FromRow, Clone)]
pub struct NoteFull {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub published_at: Option<String>,
    pub content_lang: String,
    pub has_lang: i64,
    pub tags: String,
    pub license: Option<String>,
    pub author_name: Option<String>,
    pub author_bio: Option<String>,
    pub guest_author_id: Option<i64>,
}

#[derive(FromRow, Clone, Serialize)]
pub struct ProductRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub revenue_note: String,
    pub price_cents: i64,
    pub currency: String,
    pub stock: String,
    pub has_lang: i64,
}

#[derive(FromRow, Clone)]
pub struct SubscriberRow {
    pub id: i64,
    pub email: String,
    pub lang: String,
    pub source: String,
    pub status: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub unsubscribed_at: Option<String>,
}

#[derive(FromRow, Clone)]
pub struct GuestAuthorRow {
    pub id: i64,
    pub name: String,
    pub bio: String,
    pub contact_email: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(FromRow, Clone)]
pub struct SubmissionRow {
    pub id: i64,
    pub guest_author_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub lang: String,
    pub title: String,
    pub body: String,
    pub license: String,
    pub status: String,
    pub created_at: String,
    pub note_id: Option<i64>,
}

#[derive(FromRow, Clone)]
pub struct SessionRow {
    pub id: i64,
    pub csrf: String,
    pub expires_at: String,
}

#[derive(FromRow, Clone)]
pub struct AuditRow {
    pub at: String,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub detail: Option<String>,
    pub ip: Option<String>,
}

#[derive(FromRow, Clone)]
pub struct MailRow {
    pub id: i64,
    pub to_email: String,
    pub subject: String,
    pub kind: String,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(FromRow, Clone)]
pub struct AdminNoteRow {
    pub id: i64,
    pub slug: String,
    pub status: String,
    pub published_at: Option<String>,
    pub updated_at: String,
    pub zh_title: Option<String>,
    pub en_title: Option<String>,
    pub guest_author_id: Option<i64>,
}

#[derive(FromRow, Clone)]
pub struct SubmissionRow2 {
    pub id: i64,
    pub receipt: String,
    pub occurred_at: String,
    pub weekday: Option<i64>,
    pub place: String,
    pub weather: String,
    pub temp_c: String,
    pub book_title: String,
    pub isbn: String,
    pub title: String,
    pub body: String,
    pub question: String,
    pub color: String,
    pub flag: String,
    pub links: String,
    pub lang: String,
    pub status: String,
    pub ip_hash: String,
    pub ip_region: String,
    pub created_at: String,
    pub reviewed_at: Option<String>,
    pub reviewer: Option<String>,
    pub reject_reason: Option<String>,
    pub review_note: Option<String>,
    pub note_id: Option<i64>,
    pub original_json: String,
    pub edited_at: Option<String>,
    pub edited_by: Option<String>,
}

impl SubmissionRow2 {
    pub fn words(&self) -> usize {
        self.title.chars().count() + self.body.chars().count() + self.question.chars().count()
    }

    pub fn status_label(&self) -> &'static str {
        match self.status.as_str() {
            "accepted" => "已采纳",
            "rejected" => "已退回",
            _ => "待审",
        }
    }
}

#[derive(FromRow, Clone)]
pub struct ReaderMeta {
    pub note_id: i64,
    pub submission_id: i64,
    pub occurred_at: String,
    pub weekday: Option<i64>,
    pub place: String,
    pub weather: String,
    pub temp_c: String,
    pub book_title: String,
    pub isbn: String,
    pub question: String,
    pub color: String,
    pub flag: String,
    pub links: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct PublicNote {
    pub slug: String,
    pub lang: String,
    pub title: String,
    pub summary: String,
    pub published_at: String,
}
