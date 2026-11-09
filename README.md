# wayhug.com

> 一个带激励性质的公益性读书平台 · 王元会

中英双语的读书笔记、读者来信与少量文创。邮箱是这里唯一的联系方式：订阅需二次确认，可随时退订与删除。

## 技术栈

- Rust / axum 0.8 + maud 服务端渲染（无重型 SPA，首屏不依赖 JS）
- SQLite（sqlx）+ WAL
- 双语路由 `/zh/`、`/en/`，内容语言为一等字段，缺失译文显式回退
- 邮件双重确认；SMTP 未配置时邮件写入 `mail_outbox` 并标记 `logged`，不伪造发送

## 本地运行

```bash
cp .env.example .env      # 至少填写 ADMIN_PASSWORD（>= 8 位）
cargo run
# http://127.0.0.1:3000/zh/
```

## 目录

```
src/         i18n / config / db / models / ui / public / subscribe / admin / mail / md
migrations/  0001_init.sql（表结构）、0002_seed.sql（示例内容）
static/      site.css、enhance.js、favicon.svg
docs/PLAN.md 信息架构、数据模型、路由表、订阅时序、部署与回滚
```

## 许可

内容版权归作者所有。代码部分未声明开源许可，保留所有权利。
