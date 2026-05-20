#!/bin/bash
#
# CoraOS ISO Build Script
# Builds a bootable Debian live ISO containing:
#   - Apache2 serving the dashboard on port 80
#   - coraos-backend API on port 8080 (proxied by Apache)
#   - cora-welcome console utility on tty1
#   - Custom Plymouth boot splash
#

set -e

INFO() { echo -e "\e[1;34m[INFO]\e[0m $1"; }
SUCCESS() { echo -e "\e[1;32m[SUCCESS]\e[0m $1"; }
ERROR() { echo -e "\e[1;31m[ERROR]\e[0m $1"; exit 1; }

if [ "$EUID" -ne 0 ]; then
  ERROR "This script must be run as root. Please run with sudo."
fi

WORKDIR=$(pwd)
CHROOT_DIR="${WORKDIR}/chroot"
IMAGE_DIR="${WORKDIR}/image"
ISO_NAME="coraos.iso"

cleanup() {
  INFO "Cleaning up temporary mounts..."
  if mountpoint -q "${CHROOT_DIR}/proc" 2>/dev/null; then umount -lf "${CHROOT_DIR}/proc"; fi
  if mountpoint -q "${CHROOT_DIR}/sys" 2>/dev/null; then umount -lf "${CHROOT_DIR}/sys"; fi
  if mountpoint -q "${CHROOT_DIR}/dev/pts" 2>/dev/null; then umount -lf "${CHROOT_DIR}/dev/pts"; fi
  if mountpoint -q "${CHROOT_DIR}/dev" 2>/dev/null; then umount -lf "${CHROOT_DIR}/dev"; fi
  SUCCESS "Cleanup finished."
}
trap cleanup EXIT

# ============================================================
# Host dependencies
# ============================================================
INFO "Installing required build dependencies on host..."
apt-get update
apt-get install -y debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin mtools python3-pillow

# ============================================================
# Boot logo
# ============================================================
INFO "Processing custom boot logo..."
if [ -f "logo/logo.ico" ]; then
  python3 -c "from PIL import Image; Image.open('logo/logo.ico').save('logo/logo.png')"
  python3 -c "from PIL import Image, ImageDraw; img = Image.new('RGBA', (12, 12), (0,0,0,0)); draw = ImageDraw.Draw(img); draw.ellipse((0, 0, 12, 12), fill='white'); img.save('plymouth/dot.png')"
  SUCCESS "Logo assets generated."
else
  ERROR "Could not find logo/logo.ico!"
fi

# ============================================================
# Build Rust binaries (if not already built by CI)
# ============================================================
if [ ! -f "cora-welcome/target/release/cora-welcome" ]; then
  INFO "Building cora-welcome..."
  if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
  fi
  cd cora-welcome && cargo build --release && cd ..
fi

if [ ! -f "backend/target/release/coraos-backend" ]; then
  INFO "Building coraos-backend..."
  if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
  fi
  cd backend && cargo build --release && cd ..
fi

SUCCESS "All binaries ready."

# ============================================================
# Bootstrap Debian
# ============================================================
INFO "Bootstrapping Debian system (stable/bookworm)..."
rm -rf "${CHROOT_DIR}" "${IMAGE_DIR}"
mkdir -p "${CHROOT_DIR}"
debootstrap --arch=amd64 stable "${CHROOT_DIR}" http://deb.debian.org/debian/
SUCCESS "Debian base bootstrapped."

# ============================================================
# Deploy binaries and assets into chroot
# ============================================================
INFO "Deploying CoraOS into chroot..."

# Console welcome utility
mkdir -p "${CHROOT_DIR}/usr/local/bin"
cp cora-welcome/target/release/cora-welcome "${CHROOT_DIR}/usr/local/bin/"
chmod +x "${CHROOT_DIR}/usr/local/bin/cora-welcome"

# Web management backend
cp backend/target/release/coraos-backend "${CHROOT_DIR}/usr/local/bin/"
chmod +x "${CHROOT_DIR}/usr/local/bin/coraos-backend"

# Frontend static files (served by Apache)
mkdir -p "${CHROOT_DIR}/var/www/coraos"
cp -r frontend/dist/* "${CHROOT_DIR}/var/www/coraos/"

# Backend data directories
mkdir -p "${CHROOT_DIR}/opt/coraos/data"
mkdir -p "${CHROOT_DIR}/opt/coraos/backups"
mkdir -p "${CHROOT_DIR}/opt/coraos/logs"
mkdir -p "${CHROOT_DIR}/opt/coraos/frontend/dist"

# Symlink so backend can also find frontend if needed
ln -sf /var/www/coraos "${CHROOT_DIR}/opt/coraos/frontend/dist" 2>/dev/null || \
  cp -r frontend/dist/* "${CHROOT_DIR}/opt/coraos/frontend/dist/"

# systemd service for the backend API
cp config/coraos.service "${CHROOT_DIR}/etc/systemd/system/coraos.service"
sed -i 's|/opt/coraos/backend/coraos-backend|/usr/local/bin/coraos-backend|g' "${CHROOT_DIR}/etc/systemd/system/coraos.service"
sed -i 's|WorkingDirectory=.*|WorkingDirectory=/opt/coraos|g' "${CHROOT_DIR}/etc/systemd/system/coraos.service"
sed -i 's|ReadWritePaths=.*|ReadWritePaths=/opt/coraos/data /opt/coraos/backups /opt/coraos/logs|g' "${CHROOT_DIR}/etc/systemd/system/coraos.service"
sed -i '/EnvironmentFile/d' "${CHROOT_DIR}/etc/systemd/system/coraos.service"

# Plymouth boot theme
PLYMOUTH_THEME_DIR="${CHROOT_DIR}/usr/share/plymouth/themes/coraos"
mkdir -p "${PLYMOUTH_THEME_DIR}"
cp logo/logo.png "${PLYMOUTH_THEME_DIR}/logo.png"
cp plymouth/dot.png "${PLYMOUTH_THEME_DIR}/dot.png"
cp plymouth/coraos.plymouth "${PLYMOUTH_THEME_DIR}/coraos.plymouth"
cp plymouth/coraos.script "${PLYMOUTH_THEME_DIR}/coraos.script"

# ============================================================
# Apache2 virtual host configuration
# ============================================================
INFO "Creating Apache2 configuration..."
mkdir -p "${CHROOT_DIR}/etc/apache2/sites-available"
cat << 'APACHECONF' > "${CHROOT_DIR}/etc/apache2/sites-available/coraos.conf"
<VirtualHost *:80>
    ServerName coraos
    DocumentRoot /var/www/coraos

    # Serve frontend static files directly
    <Directory /var/www/coraos>
        Options -Indexes +FollowSymLinks
        AllowOverride None
        Require all granted
    </Directory>

    # Proxy API requests to the Rust backend
    ProxyPreserveHost On
    ProxyPass /api http://127.0.0.1:8080/api
    ProxyPassReverse /api http://127.0.0.1:8080/api

    # WebSocket proxy for real-time updates
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule ^/api/ws$ ws://127.0.0.1:8080/api/ws [P,L]

    # SPA fallback: serve index.html for any route not matching a file
    RewriteCond %{REQUEST_URI} !^/api
    RewriteCond %{DOCUMENT_ROOT}%{REQUEST_URI} !-f
    RewriteCond %{DOCUMENT_ROOT}%{REQUEST_URI} !-d
    RewriteRule . /index.html [L]

    ErrorLog ${APACHE_LOG_DIR}/coraos-error.log
    CustomLog ${APACHE_LOG_DIR}/coraos-access.log combined
</VirtualHost>
APACHECONF

# ============================================================
# Chroot configuration
# ============================================================
INFO "Mounting pseudo-filesystems into chroot..."
mount --bind /dev "${CHROOT_DIR}/dev"
mount --bind /dev/pts "${CHROOT_DIR}/dev/pts"
mount --bind /proc "${CHROOT_DIR}/proc"
mount --bind /sys "${CHROOT_DIR}/sys"

INFO "Configuring Debian OS in chroot..."
cat << 'EOF' > "${CHROOT_DIR}/tmp/chroot_setup.sh"
#!/bin/bash
set -e

echo "coraos" > /etc/hostname
echo "127.0.0.1   localhost coraos" > /etc/hosts

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
  linux-image-amd64 \
  live-boot \
  live-config \
  live-config-systemd \
  systemd \
  systemd-sysv \
  grub-common \
  grub-pc-bin \
  plymouth \
  plymouth-themes \
  sudo \
  network-manager \
  iproute2 \
  bash \
  coreutils \
  util-linux \
  console-setup \
  dbus \
  apache2

# Enable Apache modules needed for reverse proxy and rewrite
a2enmod proxy
a2enmod proxy_http
a2enmod proxy_wstunnel
a2enmod rewrite

# Disable default site, enable CoraOS site
a2dissite 000-default
a2ensite coraos

# Enable services
systemctl enable NetworkManager
systemctl enable apache2
systemctl enable coraos.service

# Create system user for the backend
useradd --system --no-create-home --shell /usr/sbin/nologin coraos 2>/dev/null || true
chown -R coraos:coraos /opt/coraos

# Create interactive user
useradd -m -s /bin/bash cora
echo "cora:cora" | chpasswd
usermod -aG sudo cora
echo "cora ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Auto-login on tty1 with cora-welcome
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat << 'GETTY_EOF' > /etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin cora --noclear %I $TERM
GETTY_EOF

# Launch cora-welcome on login
cat << 'PROFILE_EOF' >> /home/cora/.bash_profile
/usr/local/bin/cora-welcome
PROFILE_EOF
chown cora:cora /home/cora/.bash_profile

# Plymouth boot theme
plymouth-set-default-theme -R coraos

apt-get clean
rm -rf /var/lib/apt/lists/*
EOF

chmod +x "${CHROOT_DIR}/tmp/chroot_setup.sh"
chroot "${CHROOT_DIR}" /bin/bash /tmp/chroot_setup.sh
rm -f "${CHROOT_DIR}/tmp/chroot_setup.sh"
SUCCESS "Guest system configuration complete."

cleanup

# ============================================================
# Build ISO
# ============================================================
INFO "Preparing boot files..."
mkdir -p "${IMAGE_DIR}/live"
KERNEL_PATH=$(ls -1 "${CHROOT_DIR}/boot/vmlinuz-"* | head -n 1)
INITRD_PATH=$(ls -1 "${CHROOT_DIR}/boot/initrd.img-"* | head -n 1)

cp "${KERNEL_PATH}" "${IMAGE_DIR}/live/vmlinuz"
cp "${INITRD_PATH}" "${IMAGE_DIR}/live/initrd.img"
SUCCESS "Kernel and initramfs exported."

INFO "Creating SquashFS image..."
mksquashfs "${CHROOT_DIR}" "${IMAGE_DIR}/live/filesystem.squashfs" -comp xz -e boot
SUCCESS "SquashFS root filesystem generated."

INFO "Creating bootloader configuration..."
mkdir -p "${IMAGE_DIR}/boot/grub"
cat << 'EOF' > "${IMAGE_DIR}/boot/grub/grub.cfg"
set default="0"
set timeout=3

insmod all_video

menuentry "CoraOS Live (GNU/Linux Debian-based)" {
    linux /live/vmlinuz boot=live quiet splash plymouth.ignore-serial-consoles vt.global_cursor_default=0
    initrd /live/initrd.img
}
EOF
SUCCESS "GRUB configuration created."

INFO "Compiling bootable hybrid UEFI/BIOS ISO..."
grub-mkrescue -o "${ISO_NAME}" "${IMAGE_DIR}"

SUCCESS "=========================================================="
SUCCESS " CoraOS ISO generated: ${WORKDIR}/${ISO_NAME}"
SUCCESS " "
SUCCESS " Access the dashboard: http://<server-ip>"
SUCCESS " Console admin: auto-starts on tty1"
SUCCESS " "
SUCCESS " Apache2 serves frontend on port 80"
SUCCESS " Backend API runs on port 8080 (proxied via /api)"
SUCCESS "=========================================================="
