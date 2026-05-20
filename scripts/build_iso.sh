#!/bin/bash
#
# CoraOS ISO Build Script
# Builds a bootable Debian ISO with:
#   - Live mode (try without installing)
#   - Install mode (real disk installer with setup wizard)
#   - Apache2 + Rust backend + dashboard
#   - Console admin utility
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

# Installer scripts
cp scripts/installer.sh "${CHROOT_DIR}/usr/local/bin/coraos-installer"
chmod +x "${CHROOT_DIR}/usr/local/bin/coraos-installer"

cp scripts/coraos-installer-gui "${CHROOT_DIR}/usr/local/bin/coraos-installer-gui"
chmod +x "${CHROOT_DIR}/usr/local/bin/coraos-installer-gui"

mkdir -p "${CHROOT_DIR}/usr/local/share/coraos"
cp scripts/installer-gui.py "${CHROOT_DIR}/usr/local/share/coraos/installer-gui.py"
chmod +x "${CHROOT_DIR}/usr/local/share/coraos/installer-gui.py"

# Frontend static files (served by Apache)
mkdir -p "${CHROOT_DIR}/var/www/coraos"
cp -r frontend/dist/* "${CHROOT_DIR}/var/www/coraos/"

# Backend working directory
mkdir -p "${CHROOT_DIR}/opt/coraos/data"
mkdir -p "${CHROOT_DIR}/opt/coraos/backups"
mkdir -p "${CHROOT_DIR}/opt/coraos/logs"
mkdir -p "${CHROOT_DIR}/opt/coraos/frontend/dist"
cp -r frontend/dist/* "${CHROOT_DIR}/opt/coraos/frontend/dist/"

# systemd service for the backend API
cat << 'SVCEOF' > "${CHROOT_DIR}/etc/systemd/system/coraos.service"
[Unit]
Description=CoraOS Server Management Platform
After=network.target apache2.service
Wants=network-online.target

[Service]
Type=simple
User=coraos
Group=coraos
WorkingDirectory=/opt/coraos
ExecStart=/usr/local/bin/coraos-backend
Restart=always
RestartSec=5
Environment=RUST_LOG=coraos_backend=info,tower_http=info

[Install]
WantedBy=multi-user.target
SVCEOF

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

    <Directory /var/www/coraos>
        Options -Indexes +FollowSymLinks
        AllowOverride None
        Require all granted
    </Directory>

    # Proxy API to Rust backend
    ProxyPreserveHost On
    ProxyPass /api http://127.0.0.1:8080/api
    ProxyPassReverse /api http://127.0.0.1:8080/api

    # WebSocket proxy
    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule ^/api/ws$ ws://127.0.0.1:8080/api/ws [P,L]

    # SPA fallback
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
  grub-efi-amd64-bin \
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
  apache2 \
  parted \
  dosfstools \
  e2fsprogs \
  squashfs-tools \
  os-prober \
  python3 \
  python3-gi \
  gir1.2-gtk-3.0 \
  xorg \
  xinit \
  openbox \
  dbus-x11

# Enable Apache modules
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

# Sudoers rules for the backend to manage system services
cat > /etc/sudoers.d/coraos << 'SUDOEOF'
coraos ALL=(root) NOPASSWD: /bin/systemctl, /usr/bin/systemctl
coraos ALL=(root) NOPASSWD: /usr/bin/apt-get, /usr/bin/apt
coraos ALL=(root) NOPASSWD: /bin/journalctl, /usr/bin/journalctl
coraos ALL=(root) NOPASSWD: /bin/tar, /usr/bin/tar
SUDOEOF
chmod 440 /etc/sudoers.d/coraos

# Create interactive user
useradd -m -s /bin/bash cora
echo "cora:cora" | chpasswd
usermod -aG sudo cora
echo "cora ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Auto-login on tty1
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat << 'GETTY_EOF' > /etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin cora --noclear %I linux
Type=idle
GETTY_EOF

# Launch cora-welcome on login (use .profile for compatibility)
cat << 'PROFILE_EOF' > /home/cora/.profile
# Launch CoraOS console menu
if [ "$(tty)" = "/dev/tty1" ]; then
    /usr/local/bin/cora-welcome
fi
PROFILE_EOF
chown cora:cora /home/cora/.profile

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
set timeout=10

insmod all_video

menuentry "CoraOS - Install to Disk" {
    linux /live/vmlinuz boot=live quiet splash plymouth.ignore-serial-consoles vt.global_cursor_default=0 systemd.unit=multi-user.target coraos.install=1
    initrd /live/initrd.img
}

menuentry "CoraOS - Live Mode (try without installing)" {
    linux /live/vmlinuz boot=live quiet splash plymouth.ignore-serial-consoles vt.global_cursor_default=0
    initrd /live/initrd.img
}

menuentry "CoraOS - Live Mode (safe graphics)" {
    linux /live/vmlinuz boot=live quiet splash nomodeset plymouth.ignore-serial-consoles vt.global_cursor_default=0
    initrd /live/initrd.img
}
EOF
SUCCESS "GRUB configuration created."

# Add a hook so the installer auto-launches when booted with coraos.install=1
mkdir -p "${CHROOT_DIR}/etc/systemd/system"
cat << 'INSTALLSVC' > "${CHROOT_DIR}/etc/systemd/system/coraos-autoinstall.service"
[Unit]
Description=CoraOS GUI Installer
After=multi-user.target
ConditionKernelCommandLine=coraos.install=1

[Service]
Type=simple
ExecStart=/usr/local/bin/coraos-installer-gui
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
Environment=DISPLAY=:0

[Install]
WantedBy=multi-user.target
INSTALLSVC

# Enable the auto-install service in the squashfs
# We need to re-mount and add the symlink
mount --bind /dev "${CHROOT_DIR}/dev"
mount --bind /proc "${CHROOT_DIR}/proc"
mount --bind /sys "${CHROOT_DIR}/sys"
chroot "${CHROOT_DIR}" systemctl enable coraos-autoinstall.service
umount "${CHROOT_DIR}/dev" 2>/dev/null || true
umount "${CHROOT_DIR}/proc" 2>/dev/null || true
umount "${CHROOT_DIR}/sys" 2>/dev/null || true

# Rebuild squashfs with the installer service enabled
INFO "Rebuilding SquashFS with installer service..."
rm -f "${IMAGE_DIR}/live/filesystem.squashfs"
mksquashfs "${CHROOT_DIR}" "${IMAGE_DIR}/live/filesystem.squashfs" -comp xz -e boot
SUCCESS "Final SquashFS generated."

INFO "Compiling bootable hybrid UEFI/BIOS ISO..."
grub-mkrescue -o "${ISO_NAME}" "${IMAGE_DIR}"

SUCCESS "═══════════════════════════════════════════════════════════"
SUCCESS " CoraOS ISO generated: ${WORKDIR}/${ISO_NAME}"
SUCCESS ""
SUCCESS " Boot menu options:"
SUCCESS "   1. Install to Disk  - Full setup wizard"
SUCCESS "   2. Live Mode        - Try without installing"
SUCCESS "   3. Safe Graphics    - For compatibility"
SUCCESS ""
SUCCESS " After installation:"
SUCCESS "   Dashboard: http://<server-ip>"
SUCCESS "   Console:   auto-login on tty1"
SUCCESS "   SSH:       cora / cora"
SUCCESS "═══════════════════════════════════════════════════════════"
