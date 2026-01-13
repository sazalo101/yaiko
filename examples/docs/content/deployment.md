# Deployment Guide

Deploy a Yaiko application to a VPS with systemd and nginx.

## Prerequisites

- A VPS (Ubuntu 22.04 or Debian 12)
- Domain name pointing to your VPS IP
- SSH access to the server

## Step 1: Prepare the Server

SSH into your server:
```bash
ssh root@your-server-ip
```

Update and install dependencies:
```bash
apt update && apt upgrade -y
apt install -y nginx certbot python3-certbot-nginx
```

## Step 2: Install Rust on Server

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Step 3: Create Application User

```bash
useradd -m -s /bin/bash yaiko
mkdir -p /opt/yaiko
chown yaiko:yaiko /opt/yaiko
```

## Step 4: Deploy Your Application

On your local machine, build for release:
```bash
yaiko build --release
```

Copy files to server:
```bash
scp target/release/myapp root@your-server-ip:/opt/yaiko/
scp -r public root@your-server-ip:/opt/yaiko/
scp -r templates root@your-server-ip:/opt/yaiko/
scp .env.production root@your-server-ip:/opt/yaiko/.env
```

Or build on server:
```bash
# Clone your repo on server
cd /opt/yaiko
git clone https://github.com/you/myapp.git .
cargo build --release
cp target/release/myapp ./
```

Set permissions:
```bash
chown -R yaiko:yaiko /opt/yaiko
chmod +x /opt/yaiko/myapp
```

## Step 5: Create Systemd Service

Create `/etc/systemd/system/yaiko.service`:
```ini
[Unit]
Description=Yaiko Application
After=network.target

[Service]
Type=simple
User=yaiko
Group=yaiko
WorkingDirectory=/opt/yaiko
Environment="HOST=127.0.0.1"
Environment="PORT=3000"
Environment="RUST_LOG=info"
EnvironmentFile=/opt/yaiko/.env
ExecStart=/opt/yaiko/myapp
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
systemctl daemon-reload
systemctl enable yaiko
systemctl start yaiko
systemctl status yaiko
```

## Step 6: Configure Nginx

Create `/etc/nginx/sites-available/myapp`:
```nginx
server {
    listen 80;
    server_name myapp.com www.myapp.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        proxy_read_timeout 60s;
    }

    location /static/ {
        alias /opt/yaiko/public/;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }
}
```

Enable the site:
```bash
ln -s /etc/nginx/sites-available/myapp /etc/nginx/sites-enabled/
nginx -t
systemctl reload nginx
```

## Step 7: SSL with Certbot

```bash
certbot --nginx -d myapp.com -d www.myapp.com
```

Follow the prompts. Certbot will:
- Obtain SSL certificate
- Configure nginx
- Set up auto-renewal

Verify auto-renewal:
```bash
certbot renew --dry-run
```

## Step 8: Configure Firewall

```bash
ufw allow 22
ufw allow 80
ufw allow 443
ufw enable
```

## Step 9: Verify Deployment

```bash
# Check service status
systemctl status yaiko

# View logs
journalctl -u yaiko -f

# Test locally
curl http://127.0.0.1:3000/health

# Test via domain
curl https://myapp.com
```

## Updating the Application

```bash
# Stop service
systemctl stop yaiko

# Deploy new binary
scp target/release/myapp root@your-server-ip:/opt/yaiko/

# Start service
systemctl start yaiko
```

Or with a deploy script:
```bash
#!/bin/bash
# deploy.sh
set -e

SERVER="root@your-server-ip"
APP_DIR="/opt/yaiko"

echo "Building..."
yaiko build --release

echo "Deploying..."
ssh $SERVER "systemctl stop yaiko"
scp target/release/myapp $SERVER:$APP_DIR/
scp -r public $SERVER:$APP_DIR/
ssh $SERVER "chown -R yaiko:yaiko $APP_DIR && systemctl start yaiko"

echo "Done!"
```

## Troubleshooting

### Service won't start
```bash
journalctl -u yaiko -n 50
```

### Permission denied
```bash
chown -R yaiko:yaiko /opt/yaiko
chmod +x /opt/yaiko/myapp
```

### Nginx 502 Bad Gateway
```bash
# Check if app is running
systemctl status yaiko

# Check port
ss -tlnp | grep 3000
```

### Can't connect to database
```bash
# Check .env file
cat /opt/yaiko/.env

# Test database connection
psql $DATABASE_URL -c "SELECT 1"
```

## Production Checklist

- [ ] Set `RUST_LOG=warn` in production
- [ ] Configure proper `DATABASE_URL`
- [ ] Set strong `JWT_SECRET`
- [ ] Enable HTTPS only
- [ ] Set up monitoring (optional)
- [ ] Configure backups (optional)
- [ ] Set up log rotation
