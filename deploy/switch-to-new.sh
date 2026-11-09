#!/usr/bin/env bash
# 把 wayhug.com 根站从旧 Rust 站(3002)切到新站(3005)。
# 只改 Apache vhost 里的 ProxyPass/ProxyPassReverse 端口，其余（/qipengshang、
# /gate/、IndexNow key、证书、日志）保持不变。切换前已备份到
# /opt/backup/wayhug-new/pre-switch/（含 SHA256SUMS.txt）。
set -euo pipefail

CONF=/etc/apache2/sites-enabled/wayhug.com-le-ssl.conf
BACK=/opt/backup/wayhug-new/pre-switch/wayhug.com-le-ssl.conf

[ -f "$BACK" ] || { echo "缺少切换前备份: $BACK"; exit 1; }

echo "== 1. 确认新站存活 =="
curl -fsS -o /dev/null http://127.0.0.1:3005/zh/ || { echo "3005 未就绪，放弃切换"; exit 1; }

echo "== 2. 改 vhost 端口 3002 -> 3005 =="
sudo sed -i 's#http://127.0.0.1:3002/#http://127.0.0.1:3005/#g' "$CONF"
sudo apache2ctl configtest

echo "== 3. reload =="
sudo systemctl reload apache2
sleep 2

echo "== 4. 校验 =="
for p in /zh/ /en/ /; do
  code=$(curl -s -o /dev/null -w '%{http_code}' "https://wayhug.com$p")
  echo "  $p -> $code"
done
grep -n '127.0.0.1:300[25]' "$CONF"
