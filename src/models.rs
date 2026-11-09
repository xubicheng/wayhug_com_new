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

#[derive(Serialize)]
pub struct PublicNote {
    pub slug: String,
    pub lang: String,
    pub title: String,
    pub summary: String,
    pub published_at: String,
}
