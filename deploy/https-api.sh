#!/bin/sh
# Managed by portfolio deploy/https-api.sh
set -eu

public_ip=${1:?Public IPv4 address required}
site_port=${2:-8050}
api_port=${3:-8052}
case "$public_ip" in *[!0-9.]*|'') echo 'Expected a public IPv4 address' >&2; exit 1;; esac
for port in "$site_port" "$api_port"; do
    case "$port" in *[!0-9]*|'') echo 'Expected numeric listener ports' >&2; exit 1;; esac
    [ "$port" -gt 1024 ] && [ "$port" -le 65535 ] || exit 1
done
[ "$site_port" != "$api_port" ] || { echo 'Site and API ports must differ' >&2; exit 1; }
[ "$(id -u)" -eq 0 ] || { echo 'Run this script with sudo' >&2; exit 1; }
for tool in nginx certbot openssl systemctl ip ufw; do
    command -v "$tool" >/dev/null || { echo "Missing server tool: $tool" >&2; exit 1; }
done
[ -n "$(ip -4 -o address show to "$public_ip/32")" ] || {
    echo 'The public IPv4 address must be assigned to this server' >&2; exit 1;
}

marker='# Managed by portfolio deploy/https-api.sh'
config=/etc/nginx/sites-available/portfolio-api
enabled=/etc/nginx/sites-enabled/portfolio-api
service=/etc/systemd/system/portfolio-api-renew.service
timer=/etc/systemd/system/portfolio-api-renew.timer
for file in "$config" "$service" "$timer"; do
    if [ -e "$file" ]; then
        IFS= read -r first_line < "$file"
        [ "$first_line" = "$marker" ] || { echo "Refusing to replace unmanaged $file" >&2; exit 1; }
    fi
done
if [ -e "$enabled" ] || [ -L "$enabled" ]; then
    [ -L "$enabled" ] && [ "$(readlink "$enabled")" = "$config" ] || {
        echo "Refusing to replace unmanaged $enabled" >&2; exit 1;
    }
fi

work=$(mktemp -d /tmp/portfolio-https.XXXXXX)
had_config=false
had_enabled=false
[ ! -e "$config" ] || { cp "$config" "$work/previous.conf"; had_config=true; }
[ ! -L "$enabled" ] || had_enabled=true
completed=false
cleanup() {
    status=$?
    trap - 0
    if [ "$completed" = false ]; then
        if [ "$had_config" = true ]; then
            cp "$work/previous.conf" "$config"
        else
            rm -f "$config"
        fi
        [ "$had_enabled" = true ] || rm -f "$enabled"
        if nginx -t; then systemctl reload nginx || true; fi
    fi
    rm -rf "$work"
    exit "$status"
}
trap cleanup 0
trap 'exit 1' HUP INT TERM

webroot=/var/lib/portfolio-acme
cert_name="portfolio-api-$public_ip"
cert_dir="/etc/letsencrypt/live/$cert_name"
mkdir -p "$webroot/.well-known/acme-challenge"

write_config() {
    cat > "$work/portfolio-api.conf" <<EOF
$marker
server {
    listen 80;
    server_name $public_ip;
    location ^~ /.well-known/acme-challenge/ {
        root $webroot;
        default_type text/plain;
    }
    location / { return 301 https://$public_ip:$api_port\$request_uri; }
}
EOF
    if [ -s "$cert_dir/fullchain.pem" ] && [ -s "$cert_dir/privkey.pem" ]; then
        openssl x509 -in "$cert_dir/fullchain.pem" -checkip "$public_ip" -noout
        cat >> "$work/portfolio-api.conf" <<EOF
server {
    # A specific public interface lets TLS and the loopback Rust API share 8052.
    listen $public_ip:$api_port ssl;
    server_name $public_ip;
    ssl_certificate $cert_dir/fullchain.pem;
    ssl_certificate_key $cert_dir/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    client_max_body_size 96k;
    proxy_http_version 1.1;
    proxy_set_header Host \$host;
    proxy_set_header X-Real-IP \$remote_addr;
    proxy_set_header X-Forwarded-For \$remote_addr;
    proxy_set_header X-Forwarded-Proto \$scheme;
    location /api/ { proxy_pass http://127.0.0.1:$api_port; }
    location = /contact { proxy_pass http://127.0.0.1:$api_port; }
    location = /health { proxy_pass http://127.0.0.1:$api_port; }
    # Only the public renderer, assets and private visitor threads; never admin.
    location / { proxy_pass http://127.0.0.1:$site_port; }
}
EOF
    fi
    install -m 644 "$work/portfolio-api.conf" "$config"
    [ -L "$enabled" ] || ln -s "$config" "$enabled"
    nginx -t
    systemctl reload nginx
}

# HTTP-01 uses the IP Host header and leaves all existing domain vhosts alone.
write_config
ufw allow 80/tcp
certbot certonly --non-interactive --agree-tos --webroot -w "$webroot" \
    --ip-address "$public_ip" --required-profile shortlived \
    --cert-name "$cert_name" --keep-until-expiring \
    --deploy-hook 'systemctl reload nginx'
write_config
ufw allow "$api_port/tcp"

# IP certificates last six days. Give this certificate its own frequent timer;
# do not change the renewal cadence or configuration of other hosted apps.
certbot_path=$(command -v certbot)
cat > "$service" <<EOF
$marker
[Unit]
Description=Renew the portfolio API IP-address certificate
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=$certbot_path renew --cert-name $cert_name --quiet
EOF
cat > "$timer" <<EOF
$marker
[Unit]
Description=Check the portfolio API certificate every six hours

[Timer]
OnCalendar=*-*-* 00,06,12,18:00:00
RandomizedDelaySec=20min
Persistent=true

[Install]
WantedBy=timers.target
EOF
systemctl daemon-reload
systemctl enable --now portfolio-api-renew.timer
systemctl is-active --quiet portfolio-api-renew.timer
completed=true
printf 'HTTPS backend ready: https://%s:%s\n' "$public_ip" "$api_port"
