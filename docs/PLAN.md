# 新版 wayhug.com · 建设方案

> 版本：v1（2026-11-09）　署名：王元会　代码仓库：`github.com/xubicheng/wayhug_com_new`

## 1. 一句话定位

wayhug.com 是一个带激励性质的公益性读书平台：站长持续写中英双语读书笔记，读者用邮箱与站长建立长期联系，站长以自有知识产权的文创产品获得一部分收入，并邀请已建立联系的读者刊登原创文章。

页面上的表达原则：

- 不写"免费"，也不写"商业"。关于页直接说明：**平台不收订阅费、不放广告；文创产品有价格，扣除制作与寄送成本后的结余用于服务器、域名与赠书。**
- 不承诺未核实的数字（例如捐赠金额），只承诺"每年一月公布上一年度简账"。
- 不写营销话术，不用"赋能/闭环/增长"一类词。

## 2. 信息架构（IA）

```
wayhug.com
├── /zh/  中文（默认）
├── /en/  英文
│   ├── 首页            /{lang}/
│   ├── 读书笔记         /{lang}/notes
│   │   └── 笔记详情     /{lang}/notes/{slug}
│   ├── 读者文章         /{lang}/readers
│   │   └── 文章详情     /{lang}/readers/{slug}
│   ├── 关于与公益说明   /{lang}/about
│   ├── 文创产品         /{lang}/products
│   ├── 订阅             /{lang}/subscribe
│   │   ├── 确认         /{lang}/subscribe/confirm?token=
│   │   ├── 完成         /{lang}/subscribe/done
│   │   └── 管理         /{lang}/subscribe/manage?token=
│   └── 受邀投稿         /{lang}/contribute?invite=
├── /admin/  站长后台（单人，无语言前缀）
└── /healthz /robots.txt /sitemap.xml /static/*
```

导航只保留五项：读书笔记、读者文章、文创、关于、订阅。语言切换放在页头右上角，`中文 / English` 两个纯文本链接，切到同一路径的另一种语言，不做下拉、不用国旗。

## 3. 页面清单与内容职责

| 页面 | 主要内容 | 关键动作 |
|---|---|---|
| 首页 | 定位一句话、最新 4 篇笔记、一段"为什么留邮箱"、最新读者文章、文创入口 | 订阅 |
| 笔记列表 | 按发布时间倒序，标题 + 一句摘要 + 日期 + 阅读时长；无封面大图 | 阅读 |
| 笔记详情 | 正文（Markdown 渲染）、标签、署名、上一篇/下一篇；缺译文时显式提示 | 订阅（文末轻量表单） |
| 读者文章列表 | 受邀作者署名、作者简介、授权声明 | 阅读 |
| 读者文章详情 | 同上 + 文末"本文由作者授权刊登，版权归作者" | — |
| 关于 / 公益说明 | 定位、站长自述、公益与营收的边界、年度简账承诺、数据与隐私承诺 | 订阅 |
| 文创产品 | 名称、说明、价格（含币种）、库存状态、收益用途说明 | 邮件联系站长 |
| 订阅 | 表单（邮箱 + 语言偏好 + 同意说明） | 双重确认 |
| 确认 / 完成 / 管理 | 确认结果、退订、删除数据 | 退订 / 删除 |
| 受邀投稿 | 邀请码校验、标题、正文、授权声明勾选 | 提交待审 |
| 后台 | 见第 8 节 | 全部管理动作 |

## 4. 数据模型（SQLite）

语言是一等字段：内容主体与译文分表，新增语言只加数据行，不改表结构。

```sql
notes(id, slug UNIQUE, lang, status, published_at, created_at, updated_at,
      guest_author_id, license, tags)
note_i18n(note_id, lang, title, summary, body, PRIMARY KEY(note_id, lang))

subscribers(id, email UNIQUE, lang, source, status, created_at, confirmed_at,
            unsubscribed_at, confirm_token_hash, confirm_expires_at,
            manage_token_hash, last_mail_at, ip_hash)
-- status: pending | confirmed | unsubscribed；email 仅后台可见

guest_authors(id, name, bio, contact_email, status, created_at)
invites(id, guest_author_id, code_hash, expires_at, used_at, created_at)
submissions(id, guest_author_id, name, email, lang, title, body, license,
            status, created_at, reviewed_at, note_id)

products(id, slug UNIQUE, price_cents, currency, stock, url, created_at, updated_at)
product_i18n(product_id, lang, name, description, revenue_note,
             PRIMARY KEY(product_id, lang))
-- stock: in_stock | preorder | sold_out

sessions(id, token_hash UNIQUE, csrf, created_at, expires_at, last_seen_at, ip)
audit_log(id, at, actor, action, target, detail, ip)
mail_outbox(id, to_email, subject, body, kind, status, error, created_at, sent_at)
-- status: pending | sent | failed | logged（SMTP 未接入时只落盘并标注）
settings(k, v)
```

存储最小化：订阅者只存 `email / 语言 / 来源 / 时间 / 确认状态`，另有哈希后的令牌与 IP 哈希；不存姓名、不存 UA、不存行为轨迹。公开页面与公开 JSON 永不含 email 字段。

## 5. 路由表

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/` | 302 到 `/zh/`（按 Accept-Language 可到 `/en/`） |
| GET | `/{lang}/` | 首页 |
| GET | `/{lang}/notes` | 列表（`?page=`） |
| GET | `/{lang}/notes/{slug}` | 详情（缺译文→回退 + 提示） |
| GET | `/{lang}/readers`、`/{lang}/readers/{slug}` | 读者文章 |
| GET | `/{lang}/about`、`/{lang}/products` | 关于 / 文创 |
| GET/POST | `/{lang}/subscribe` | 表单 / 提交 |
| GET | `/{lang}/subscribe/confirm?token=` | 双重确认 |
| GET | `/{lang}/subscribe/done` | 完成页 |
| GET | `/{lang}/subscribe/manage?token=` | 自助管理 |
| POST | `/{lang}/subscribe/unsubscribe`、`/delete` | 退订 / 删除 |
| GET/POST | `/{lang}/contribute?invite=` | 受邀投稿 |
| GET/POST | `/admin/login` | 登录 |
| POST | `/admin/logout` | 退出 |
| GET | `/admin` | 概览 |
| GET/POST | `/admin/notes`、`/admin/notes/new`、`/admin/notes/{id}` | 笔记与双语版本 |
| GET | `/admin/subscribers` | 订阅者 |
| GET/POST | `/admin/authors`、`/admin/authors/{id}/invite` | 受邀作者与邀请码 |
| GET/POST | `/admin/submissions`、`/admin/submissions/{id}/approve` | 投稿审核 |
| GET/POST | `/admin/products`、`/admin/products/{id}` | 文创（价格/库存） |
| GET | `/admin/audit`、`/admin/outbox` | 审计日志 / 邮件发件箱 |
| GET | `/api/notes` | 公开只读 JSON（不含 email） |
| GET | `/healthz`、`/robots.txt`、`/sitemap.xml` | 运维与索引 |

## 6. 双语内容模型

- 语言枚举 `Lang { Zh, En }`，URL 前缀即语言，`/zh/` 与 `/en/` 互切保持同一路径。
- 每条笔记有 `notes.lang`（原文语言）与若干 `note_i18n` 译文行。
- 取文逻辑：先取目标语言译文；缺失则回退原文，并在正文顶部显示"本文暂无英文版本，以下为中文原文 / This note has no English version yet; the original Chinese text is shown below."
- 界面文案集中在 `i18n.rs` 的 `Strings` 结构体（zh/en 两个常量），新增语言 = 加一个常量 + 一个枚举值，路由与模板不改。
- 日期按 `Asia/Shanghai` 显示；英文界面用 `9 Nov 2026` 格式。

## 7. 订阅（double opt-in）时序

```
读者提交 email+lang
  → 校验格式/长度，规范化小写
  → 写入或更新 subscribers(status=pending, confirm_token_hash, expires=+48h)
  → 发确认邮件（含确认链接 + 退订/删除说明）
  → 页面统一提示"请查收确认邮件"（不暴露该邮箱是否已存在）
读者点击确认链接
  → 校验 token 哈希 + 未过期 → status=confirmed, confirmed_at
  → 跳转 /{lang}/subscribe/done
确认邮件内附 manage 链接（长期有效）
  → 管理页可退订（status=unsubscribed）或删除（email 字段清空并标记）
重复提交
  → 已 confirmed：重发 manage 链接，不新增记录
  → 仍 pending：换新 token 重发，不新增记录
  → 已 unsubscribed：重新走 pending→确认
```

防滥用：同 IP 订阅提交 5 次/小时；表单含蜜罐字段；所有公开 POST 校验 CSRF 双提交令牌。

邮件实现：优先用服务器已有的 QQ SMTP 凭据（环境变量注入，仅服务器 `.env`，不入库不入仓）。若未配置，则邮件写入 `mail_outbox`（status=logged）并打印到标准日志，后台"发件箱"可查看全文——此时报告与页面均明确写"邮件服务未接入"。

## 8. 后台功能

单人使用，无注册。登录：`ADMIN_USER` + `ADMIN_PASSWORD`（环境变量）；登录后 12 小时会话，Cookie `HttpOnly + SameSite=Strict + Secure`；同一 IP 15 分钟内错 5 次锁定。

| 模块 | 能力 |
|---|---|
| 概览 | 笔记/订阅/投稿/产品计数，最近审计 10 条 |
| 笔记 | 新建、编辑、发布/撤下、删除；中文与英文标题/摘要/正文分别编辑；标签；读者文章可挂作者与授权声明 |
| 订阅者 | 列表（email 仅此处可见）、状态、来源、时间；删除（GDPR 式可删除） |
| 受邀作者 | 新建作者、简介、私密邮箱、状态；生成一次性邀请码（72 小时） |
| 投稿 | 待审列表、查看全文、通过（自动建笔记并署名）/驳回 |
| 文创 | 新建/编辑名称、说明、价格（分）、币种、库存状态、收益用途 |
| 审计 | 登录、登出、增删改、审核、删除订阅者等全部记录 |
| 发件箱 | 邮件主题、收件人（仅后台）、状态、错误 |

CSRF：会话内随机 token，所有后台 POST 校验；跨站请求直接 403。

## 9. 视觉与排版规范

- 基调：纸张感、扁平、无阴影无渐变；大量留白；细线分隔（1px `#e4e0d7`）。
- 色彩：纸白 `#fbfaf7`、墨 `#20272e`、次级字 `#5f6b73`、品牌 `#5d8aa8`（沿用品牌主色）、暖褐点缀 `#a8794a`（只用于引文与强调）。
- 字体：正文衬线（Songti SC / Noto Serif CJK SC / Georgia），界面无衬线（系统栈）；不加载外部字体。
- 字号/行高：正文移动端 17px / 1.75，桌面 18px / 1.8；行宽上限 68 个字符（约 44rem）；段落间距 1.2em；标题层级 1.9/1.45/1.15rem。
- 触控：所有可点元素 ≥44px；焦点可见（2px 品牌色描边）。
- 响应式：单列，断点 720px；375px 宽无横向滚动。
- 无 JS 可完整阅读与订阅；`enhance.js`（<2KB）仅做"提交按钮防重复点击、复制邀请链接"。
- 可访问性：语义化 landmark、skip link、表单 label、`lang` 属性、对比度 ≥ 4.5:1、图片有 alt。

## 10. 安全与隐私

- 不存明文令牌（confirm/manage/session 均存 SHA-256 哈希）。
- 所有输出经 maud 自动转义；Markdown 中的原始 HTML 事件被转义，不允许注入。
- 安全响应头：CSP（`default-src 'self'`）、`X-Content-Type-Options`、`Referrer-Policy`、`Permissions-Policy`、HSTS（TLS 由 Apache 终止）。
- 公开 HTML/JSON 不含 email；订阅者 email 仅后台可见；审计记录操作者与 IP。
- 密钥全部来自 `/opt/wayhug-new/.env`（root 只读），仓库与前端不含任何密钥。

## 11. 部署与回滚

服务器：`129.211.170.72`（Ubuntu 22.04），Apache 反代。

- 新站：`/opt/wayhug-new/`（二进制 + data + static），systemd `wayhug-new.service`，监听 `127.0.0.1:3005`。
- 旧站：`wayhug-rust.service`（3002）继续运行，不下线、不改代码。
- **切换前**：对 `/var/www/wayhug.com`、`/opt/wayhug.com`（排除 target）、旧 vhost 配置做完整快照 + SHA-256 清单，存于 `/opt/backup/wayhug-new/pre-switch/` 并回报路径。
- 切换：只改 `/etc/apache2/sites-enabled/wayhug.com-le-ssl.conf` 中 `ProxyPass /` 的目标端口（3002 → 3005），保留 `/qipengshang`、`/hypen01.html`、`/gate/`、`/stats-dashboard.html`、IndexNow key 等原有直出规则。
- 回滚：一条命令把端口改回 3002 并 `systemctl reload apache2`；旧目录、旧 vhost 备份保留。
- 备份：源码 + SQLite（`VACUUM INTO` 一致性快照）+ `.env` + vhost + 证书引用 + 哈希清单，打包到 `/opt/backup/wayhug-new/` 并拉回本地。

## 12. 验收清单（逐项实测）

1. `/zh/`、`/en/` 200，切换后停留在同一页面；
2. 笔记详情 375px 宽可读，正文 ≥16px、行高 ≥1.6；
3. 订阅：提交 → 确认邮件（或明确说明未接入）→ 确认 → 后台可见；重复提交不新增记录；退订可用；
4. 未登录访问后台 → 302/401；CSRF 实测被拦截；
5. `/api/notes` 不含 email 字段；
6. 关闭 JS，首页内容完整；
7. 备份文件哈希可复核；
8. Lighthouse（移动端）性能/可访问性实测值写入报告。
