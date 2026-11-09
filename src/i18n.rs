//! 双语界面文案。新增语言：加一个 `Lang` 变体 + 一个 `Strings` 常量即可，
//! 内容表（note_i18n / product_i18n）以 lang 字段扩展，无需改表结构。

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn from_seg(s: &str) -> Option<Lang> {
        match s {
            "zh" => Some(Lang::Zh),
            "en" => Some(Lang::En),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn html_lang(self) -> &'static str {
        match self {
            Lang::Zh => "zh-CN",
            Lang::En => "en",
        }
    }

    pub fn other(self) -> Lang {
        match self {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        }
    }

    pub fn s(self) -> &'static Strings {
        match self {
            Lang::Zh => &ZH,
            Lang::En => &EN,
        }
    }

    pub fn reading_time(self, minutes: i64) -> String {
        match self {
            Lang::Zh => format!("{} 分钟", minutes.max(1)),
            Lang::En => format!("{} min read", minutes.max(1)),
        }
    }

    /// 把 `YYYY-MM-DD` 或 `YYYY-MM-DD HH:MM:SS`（UTC）格式化为本地日期。
    pub fn date(self, raw: &str) -> String {
        let date = raw.get(0..10).unwrap_or(raw);
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return date.to_string();
        }
        let (y, m, d) = (parts[0], parts[1], parts[2]);
        match self {
            Lang::Zh => format!(
                "{}年{}月{}日",
                y,
                m.trim_start_matches('0'),
                d.trim_start_matches('0')
            ),
            Lang::En => {
                const MONTHS: [&str; 12] = [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ];
                let idx = m.parse::<usize>().unwrap_or(1).clamp(1, 12) - 1;
                format!("{} {} {}", d.trim_start_matches('0'), MONTHS[idx], y)
            }
        }
    }

    /// 语言切换链接：把路径首段换成另一种语言。
    pub fn switch_path(self, path: &str) -> String {
        let other = self.other().code();
        let mut segs = path.split('/');
        let _ = segs.next();
        let first = segs.next().unwrap_or("");
        if Lang::from_seg(first).is_some() {
            let rest: Vec<&str> = segs.collect();
            if rest.is_empty() {
                format!("/{}/", other)
            } else {
                format!("/{}/{}", other, rest.join("/"))
            }
        } else {
            format!("/{}/", other)
        }
    }
}

pub struct Strings {
    pub site_tagline: &'static str,
    pub site_desc: &'static str,
    pub nav_notes: &'static str,
    pub nav_readers: &'static str,
    pub nav_products: &'static str,
    pub nav_about: &'static str,
    pub nav_subscribe: &'static str,
    pub read_more: &'static str,
    pub back_home: &'static str,
    pub not_found_title: &'static str,
    pub not_found_body: &'static str,

    pub home_kicker: &'static str,
    pub home_intro: &'static str,
    pub home_notes_title: &'static str,
    pub home_notes_all: &'static str,
    pub home_why_title: &'static str,
    pub home_why_1: &'static str,
    pub home_why_2: &'static str,
    pub home_why_3: &'static str,
    pub home_readers_title: &'static str,
    pub home_readers_empty: &'static str,
    pub home_products_title: &'static str,
    pub home_products_note: &'static str,

    pub notes_title: &'static str,
    pub notes_intro: &'static str,
    pub notes_empty: &'static str,
    pub previous_note: &'static str,
    pub next_note: &'static str,
    pub tags_label: &'static str,
    pub fallback_notice: &'static str,

    pub readers_title: &'static str,
    pub readers_intro: &'static str,
    pub readers_empty: &'static str,
    pub readers_license: &'static str,

    pub about_title: &'static str,
    pub about_lead: &'static str,
    pub about_p1: &'static str,
    pub about_p2: &'static str,
    pub about_money_title: &'static str,
    pub about_money_1: &'static str,
    pub about_money_2: &'static str,
    pub about_money_3: &'static str,
    pub about_privacy_title: &'static str,
    pub about_privacy: &'static str,
    pub about_contact_title: &'static str,
    pub about_contact: &'static str,

    pub products_title: &'static str,
    pub products_intro: &'static str,
    pub products_empty: &'static str,
    pub stock_in_stock: &'static str,
    pub stock_preorder: &'static str,
    pub stock_sold_out: &'static str,
    pub revenue_use: &'static str,
    pub order_note: &'static str,

    pub subscribe_title: &'static str,
    pub subscribe_intro: &'static str,
    pub label_email: &'static str,
    pub consent_label: &'static str,
    pub consent_hint: &'static str,
    pub button_subscribe: &'static str,
    pub subscribe_privacy: &'static str,
    pub sent_title: &'static str,
    pub sent_body: &'static str,
    pub sent_hint: &'static str,
    pub confirmed_title: &'static str,
    pub confirmed_body: &'static str,
    pub invalid_token_title: &'static str,
    pub invalid_token_body: &'static str,
    pub manage_title: &'static str,
    pub manage_status_label: &'static str,
    pub status_confirmed: &'static str,
    pub status_pending: &'static str,
    pub status_unsubscribed: &'static str,
    pub manage_hint: &'static str,
    pub manage_unsubscribe: &'static str,
    pub manage_delete: &'static str,
    pub manage_delete_warn: &'static str,
    pub manage_unsubscribed_ok: &'static str,
    pub manage_deleted_ok: &'static str,
    pub manage_token_invalid: &'static str,

    pub contribute_title: &'static str,
    pub contribute_intro: &'static str,
    pub contribute_invalid: &'static str,
    pub field_invite: &'static str,
    pub field_title: &'static str,
    pub field_body: &'static str,
    pub field_license: &'static str,
    pub label_agree_license: &'static str,
    pub submit_contribution: &'static str,
    pub contribute_done_title: &'static str,
    pub contribute_done_body: &'static str,

    pub footer_note: &'static str,
    pub footer_rights: &'static str,
    pub footer_icp: &'static str,
    pub footer_psb: &'static str,
    pub footer_ledger: &'static str,
    pub lang_switch: &'static str,
}

pub static ZH: Strings = Strings {
    site_tagline: "慢一点的读书站",
    site_desc: "王元会的公益性读书平台：中英双语读书笔记、读者来信、文创与读者原创文章。",
    nav_notes: "读书笔记",
    nav_readers: "读者文章",
    nav_products: "文创",
    nav_about: "关于",
    nav_subscribe: "订阅",
    read_more: "继续读",
    back_home: "回到首页",
    not_found_title: "这一页不在了",
    not_found_body: "链接可能写错，或内容已经撤下。可以从读书笔记重新开始。",

    home_kicker: "公益性读书平台",
    home_intro: "这里存放我读书时留下来的笔记：一句话、一段引文、一次没想明白的疑问。中英双语，慢慢更新。不追热点，也不做推荐算法。",
    home_notes_title: "最近的笔记",
    home_notes_all: "全部笔记",
    home_why_title: "为什么留下邮箱",
    home_why_1: "有新笔记时，我会写一封简短的邮件，不群发促销。",
    home_why_2: "每年一月，寄一份上一年度的收支简账。",
    home_why_3: "邮箱只用于这两件事；你可以随时退订，也可以要求删除。",
    home_readers_title: "读者文章",
    home_readers_empty: "还没有读者文章。通过邮件与我建立联系的读者，会陆续收到邀请。",
    home_products_title: "文创",
    home_products_note: "有价格，也有成本；结余用于这个站点与赠书。",

    notes_title: "读书笔记",
    notes_intro: "按时间倒序。每篇都不长，读完大约几分钟。",
    notes_empty: "还没有发布笔记。",
    previous_note: "上一篇",
    next_note: "下一篇",
    tags_label: "标签",
    fallback_notice: "这篇还没有英文版本，以下为中文原文。",

    readers_title: "读者文章",
    readers_intro: "这些文章由已通过邮件与我建立联系的读者受邀撰写并授权刊登，版权归作者本人。",
    readers_empty: "还没有读者文章。",
    readers_license: "本文由作者授权刊登，版权归作者所有。",

    about_title: "关于这个站点",
    about_lead: "wayhug.com 是一个带激励性质的公益性读书平台，署名王元会。它不是作品集，而是一份持续的读书记录，以及由此长出来的读者关系。",
    about_p1: "我在这里发布自己的读书笔记，中文与英文各一份。翻译不是逐字对应的，英文版更像一次重写，写给不同语言里的读者。",
    about_p2: "站点不收订阅费，不放广告，也不卖读者数据。读者通过邮箱与我联系；对已经建立联系的读者，我会发出邀请，请他们刊登自己的原创文章，扩展这里看问题的角度。",
    about_money_title: "公益，与营收",
    about_money_1: "这个站点不是纯免费的：文创产品有明确标价。",
    about_money_2: "收入先覆盖制作、包装、寄送、服务器与域名等直接成本；结余用于购书与赠书，以及维持站点运行。",
    about_money_3: "每年一月，我会公布上一年度的收支简账，包含收入、成本与结余去向。数字不修饰。",
    about_privacy_title: "邮箱与数据",
    about_privacy: "订阅采用双重确认：提交后你会收到一封确认邮件，点击链接才算订阅成功。数据库只保存邮箱、语言偏好、来源、时间与确认状态。邮箱不会出现在任何公开页面，也不会提供给第三方。你可以随时退订或要求删除。",
    about_contact_title: "联系",
    about_contact: "邮件是最可靠的方式。读书笔记里的问题、翻译建议、投稿意向，都可以直接写给我。",

    products_title: "文创",
    products_intro: "我与纸、墨、装订打交道时做的小东西。数量不多，价格如实标注。",
    products_empty: "第一批文创还在准备中，可以先用邮件告诉我你想要什么。",
    stock_in_stock: "有货",
    stock_preorder: "预售",
    stock_sold_out: "暂时售罄",
    revenue_use: "收益用途",
    order_note: "购买请发邮件，说明物品与数量，我会回复付款与寄送方式。",

    subscribe_title: "订阅新笔记",
    subscribe_intro: "留下邮箱，有新笔记时收到一封短信。没有促销，没有转发要求。",
    label_email: "邮箱",
    consent_label: "我同意接收新笔记与年度简账邮件，并了解可以随时退订或删除。",
    consent_hint: "提交后会收到一封确认邮件，点击其中的链接才完成订阅。",
    button_subscribe: "订阅",
    subscribe_privacy: "只保存邮箱、语言、来源与时间；不追踪阅读行为，不出售数据。",
    sent_title: "请查收确认邮件",
    sent_body: "如果这个邮箱可以接收邮件，几分钟内会收到一封确认信。点击其中的链接即完成订阅。",
    sent_hint: "没有收到？请检查垃圾邮件目录，或稍后重试。确认链接 48 小时内有效。",
    confirmed_title: "订阅已确认",
    confirmed_body: "谢谢你愿意读。下一封邮件会是一篇新笔记，不会更频繁。",
    invalid_token_title: "链接无效或已过期",
    invalid_token_body: "确认链接 48 小时内有效。你可以回到订阅页重新提交。",
    manage_title: "管理订阅",
    manage_status_label: "当前状态",
    status_confirmed: "已确认",
    status_pending: "等待确认",
    status_unsubscribed: "已退订",
    manage_hint: "你可以在这里退订，或要求删除邮箱记录。",
    manage_unsubscribe: "退订",
    manage_delete: "删除我的邮箱",
    manage_delete_warn: "删除后记录不可恢复；如果以后想再订阅，需要重新确认。",
    manage_unsubscribed_ok: "已退订。不会再收到邮件。",
    manage_deleted_ok: "邮箱记录已删除。",
    manage_token_invalid: "管理链接无效。",

    contribute_title: "受邀投稿",
    contribute_intro: "这个入口只对收到邀请的读者开放。邀请码在邮件里，有效期 72 小时。",
    contribute_invalid: "邀请码无效或已过期。可以回信向我要一个新的。",
    field_invite: "邀请码",
    field_title: "标题",
    field_body: "正文（Markdown）",
    field_license: "授权声明",
    label_agree_license: "我确认这是本人原创，并授权 wayhug.com 刊登；版权仍归我所有。",
    submit_contribution: "提交待审",
    contribute_done_title: "已收到",
    contribute_done_body: "谢谢。我会在读完后的几天内回信，确认是否刊登以及排版细节。",

    footer_note: "公益性读书平台 · 读书笔记、读者文章与文创",
    footer_rights: "王元会",
    footer_icp: "苏ICP备17031763号-6",
    footer_psb: "苏公网安备32010502011359号",
    footer_ledger: "年度简账每年一月公布",
    lang_switch: "English",
};

pub static EN: Strings = Strings {
    site_tagline: "A slower reading site",
    site_desc: "Wang Yuanhui's public-benefit reading platform: bilingual reading notes, letters from readers, small press objects, and guest essays.",
    nav_notes: "Notes",
    nav_readers: "Readers",
    nav_products: "Objects",
    nav_about: "About",
    nav_subscribe: "Subscribe",
    read_more: "Read on",
    back_home: "Back to the homepage",
    not_found_title: "This page is gone",
    not_found_body: "The link may be wrong, or the page has been taken down. The reading notes are a good place to start again.",

    home_kicker: "A public-benefit reading platform",
    home_intro: "This is where I keep what is left over from reading: a sentence, a quotation, a question I have not resolved. Written in Chinese and English, updated slowly. No trends, no recommendation algorithm.",
    home_notes_title: "Recent notes",
    home_notes_all: "All notes",
    home_why_title: "Why leave an email",
    home_why_1: "When a new note is published, I send a short letter. No promotions.",
    home_why_2: "Every January, I send a plain account of the previous year's income and costs.",
    home_why_3: "The address is used for those two things only. You can unsubscribe or ask me to delete it at any time.",
    home_readers_title: "Reader essays",
    home_readers_empty: "No reader essays yet. Readers who have written to me will receive invitations over time.",
    home_products_title: "Objects",
    home_products_note: "They have prices and costs; what is left goes to this site and to donated books.",

    notes_title: "Reading notes",
    notes_intro: "Newest first. None of them is long; a few minutes each.",
    notes_empty: "No notes published yet.",
    previous_note: "Previous",
    next_note: "Next",
    tags_label: "Tags",
    fallback_notice: "This note has no English version yet; the original Chinese text is shown below.",

    readers_title: "Reader essays",
    readers_intro: "These essays are written and authorised by readers who have written to me. Copyright remains with each author.",
    readers_empty: "No reader essays yet.",
    readers_license: "Published with the author's permission. Copyright remains with the author.",

    about_title: "About this site",
    about_lead: "wayhug.com is a public-benefit reading platform with an incentive, signed Wang Yuanhui. It is not a portfolio but an ongoing record of reading, and the reader relationships that grow out of it.",
    about_p1: "I publish my reading notes here, in Chinese and in English. The English is not a literal translation; it is closer to a rewriting for readers in another language.",
    about_p2: "There is no subscription fee, no advertising, and reader data is not for sale. Readers reach me by email. To readers who have written, I send invitations to publish their own essays here, widening the range of views.",
    about_money_title: "Public benefit, and revenue",
    about_money_1: "This site is not purely free: the objects have stated prices.",
    about_money_2: "Revenue first covers making, packing, shipping, the server and the domain. What remains buys books, gives books away, and keeps the site running.",
    about_money_3: "Every January I publish a plain account of the previous year: income, costs, and where the balance went. The figures are not dressed up.",
    about_privacy_title: "Email and data",
    about_privacy: "Subscription uses double opt-in: after submitting, you receive a confirmation email, and only clicking its link completes the subscription. The database stores only the email address, language preference, source, time and confirmation status. The address never appears on a public page and is never given to third parties. You may unsubscribe or ask for deletion at any time.",
    about_contact_title: "Contact",
    about_contact: "Email is the most reliable way. Questions about a note, translation suggestions, or an offer to write are all welcome.",

    products_title: "Objects",
    products_intro: "Small things I make while working with paper, ink and binding. Not many, and the prices are stated plainly.",
    products_empty: "The first objects are still being prepared. You are welcome to email me what you would like to see.",
    stock_in_stock: "In stock",
    stock_preorder: "Pre-order",
    stock_sold_out: "Sold out",
    revenue_use: "Where the money goes",
    order_note: "To order, email me the item and quantity; I will reply with payment and shipping details.",

    subscribe_title: "Subscribe to new notes",
    subscribe_intro: "Leave an address and receive a short letter when a new note appears. No promotions, no forwarding requests.",
    label_email: "Email",
    consent_label: "I agree to receive new notes and the annual account, and understand I can unsubscribe or delete my address at any time.",
    consent_hint: "After submitting you will receive a confirmation email; the subscription starts when you click its link.",
    button_subscribe: "Subscribe",
    subscribe_privacy: "Only email, language, source and time are stored. No reading behaviour is tracked, and data is not sold.",
    sent_title: "Please check your inbox",
    sent_body: "If this address can receive mail, a confirmation message will arrive within a few minutes. Click the link inside to finish subscribing.",
    sent_hint: "Nothing yet? Check the spam folder, or try again shortly. Confirmation links are valid for 48 hours.",
    confirmed_title: "Subscription confirmed",
    confirmed_body: "Thank you for reading. The next letter will be a new note, and no more often than that.",
    invalid_token_title: "This link is invalid or expired",
    invalid_token_body: "Confirmation links are valid for 48 hours. You can submit the subscription form again.",
    manage_title: "Manage subscription",
    manage_status_label: "Status",
    status_confirmed: "Confirmed",
    status_pending: "Awaiting confirmation",
    status_unsubscribed: "Unsubscribed",
    manage_hint: "You can unsubscribe here, or ask for the address to be deleted.",
    manage_unsubscribe: "Unsubscribe",
    manage_delete: "Delete my address",
    manage_delete_warn: "Deletion cannot be undone. To subscribe again later, you would need to confirm once more.",
    manage_unsubscribed_ok: "Unsubscribed. No further emails will be sent.",
    manage_deleted_ok: "The address has been deleted.",
    manage_token_invalid: "This management link is invalid.",

    contribute_title: "Invited contribution",
    contribute_intro: "This form is open to invited readers only. The code is in your email and is valid for 72 hours.",
    contribute_invalid: "The invitation code is invalid or expired. Write back and I will send a new one.",
    field_invite: "Invitation code",
    field_title: "Title",
    field_body: "Text (Markdown)",
    field_license: "Permission",
    label_agree_license: "I confirm this is my original work and authorise wayhug.com to publish it; copyright remains mine.",
    submit_contribution: "Submit for review",
    contribute_done_title: "Received",
    contribute_done_body: "Thank you. I will write back within a few days of reading it, to confirm whether it will be published and to settle the details.",

    footer_note: "A public-benefit reading platform · notes, reader essays and objects",
    footer_rights: "Wang Yuanhui",
    footer_icp: "苏ICP备17031763号-6",
    footer_psb: "苏公网安备32010502011359号",
    footer_ledger: "The annual account is published each January",
    lang_switch: "中文",
};
