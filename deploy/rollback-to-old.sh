#!/usr/bin/env bash
# 一键回滚：把 wayhug.com 根站从新站(3005)切回旧站(3002)。
# 旧站点目录、旧 binary、旧 systemd 服务都保留未动，回滚只需恢复 Apache 配置并 reload。
set -euo pipefail

CONF=/etc/apache2/sites-available/wayhug.com-le-ssl.conf
LINK=/etc/apache2/sites-enabled/wayhug.com-le-ssl.conf
BACK=/opt/backup/wayhug-new/pre-switch/wayhug.com-le-ssl.conf

[ -f "$BACK" ] || { echo "缺少原始 vhost 备份: $BACK"; exit 1; }

echo "== 1. 恢复原始 vhost =="
sudo cp "$BACK" "$CONF"
[ -L "$LINK" ] || sudo ln -sfn ../sites-available/wayhug.com-le-ssl.conf "$LINK"
sudo apache2ctl configtest

echo "== 2. reload apache =="
sudo systemctl reload apache2
sleep 2

echo "== 3. 确认旧站仍在监听 3002、新站停止 =="
systemctl is-active wayhug-rust || echo "警告: wayhug-rust 未运行"
sudo systemctl stop wayhug-new 2>/dev/null || true
sudo systemctl disable wayhug-new 2>/dev/null || true

echo "== 4. 校验 =="
for p in / /qipengshang/; do
  code=$(curl -s -o /dev/null -w '%{http_code}' "https://wayhug.com$p")
  echo "  $p -> $code"
done
grep -n '127.0.0.1:300[25]' "$CONF"
