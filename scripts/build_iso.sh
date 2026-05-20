#!/bin/bash
#
# CoraOS ISO Build Script
#
# Produces a bootable Debian live ISO with:
#   - Calamares graphical installer
#   - CoraOS web management platform (Apache + Rust backend)
#   - Console admin utility on tty1
#   - Plymouth boot splash
#

set -e

INFO()    { echo -e "\e[1;34m[INFO]\e[0m $1"; }
SUCCESS() { echo -e "\e[1;32m[OK]\e[0m $1"; }
ERROR()   { echo -e "\e[1;31m[ERROR]\e[0m $1"; exit 1; }

[[ "$EUID" -eq 0 ]] || ERROR "Run this script as root."

WORKDIR=$(pwd)
CHROOT="${WORKDIR}/chroot"
IMAGE="${WORKDIR}/image"
ISO_NAME="coraos.iso"

cleanup() {
    for mp in proc sys dev/pts dev; do
        mountpoint -q "${CHROOT}/${mp}" 2>/dev/null && umount -lf "${CHROOT}/${mp}"
    done
}
trap cleanup EXIT

# ─── Host Dependencies ────────────────────────────────────────────────────────

INFO "Installing host build tools..."
apt-get update -qq
apt-get install -y -qq debootstrap squashfs-tools xorriso \
    grub-pc-bin grub-efi-amd64-bin mtools python3-pillow

# ─── Logo Assets ──────────────────────────────────────────────────────────────

INFO "Generating logo assets..."
[[ -f "logo/logo.ico" ]] || ERROR "logo/logo.ico not found"
python3 -c "from PIL import Image; Image.open('logo/logo.ico').save('logo/logo.png')"
python3 -c "
from PIL import Image, ImageDraw
img = Image.new('RGBA', (12, 12), (0,0,0,0))
draw = ImageDraw.Draw(img)
draw.ellipse((0, 0, 12, 12), fill='white')
img.save('plymouth/dot.png')
"
SUCCESS "Logo assets ready."

# ─── Rust Binaries ────────────────────────────────────────────────────────────

build_rust() {
    local dir="$1"
    local bin="$2"
    if [[ ! -f "${dir}/target/release/${bin}" ]]; then
        INFO "Building ${bin}..."
        command -v cargo &>/dev/null || {
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        }
        (cd "$dir" && cargo build --release)
    fi
}

build_rust "cora-welcome" "cora-welcome"
build_rust "backend" "coraos-backend"
SUCCESS "Binaries compiled."

# ─── Bootstrap Debian ─────────────────────────────────────────────────────────

INFO "Bootstrapping Debian stable..."
rm -rf "${CHROOT}" "${IMAGE}"
mkdir -p "${CHROOT}"
debootstrap --arch=amd64 stable "${CHROOT}" http://deb.debian.org/debian/
SUCCESS "Base system ready."

# ─── Deploy CoraOS Files ──────────────────────────────────────────────────────

INFO "Deploying CoraOS files..."

# Binaries
install -m 755 cora-welcome/target/release/cora-welcome "${CHROOT}/usr/local/bin/"
install -m 755 backend/target/release/coraos-backend    "${CHROOT}/usr/local/bin/"

# Web frontend (served by Apache)
mkdir -p "${CHROOT}/var/www/coraos"
cp -r frontend/dist/* "${CHROOT}/var/www/coraos/"

# Backend working directory
mkdir -p "${CHROOT}/opt/coraos"/{data,backups,logs,frontend/dist}
cp -r frontend/dist/* "${CHROOT}/opt/coraos/frontend/dist/"

# Plymouth theme
mkdir -p "${CHROOT}/usr/share/plymouth/themes/coraos"
cp logo/logo.png plymouth/dot.png plymouth/coraos.plymouth plymouth/coraos.script \
   "${CHROOT}/usr/share/plymouth/themes/coraos/"

# ─── Systemd Services ─────────────────────────────────────────────────────────

# Backend API service
cat > "${CHROOT}/etc/systemd/system/coraos.service" << 'EOF'
[Unit]
Description=CoraOS Backend
After=network.target apache2.service

[Service]
Type=simple
User=coraos
Group=coraos
WorkingDirectory=/opt/coraos
ExecStart=/usr/local/bin/coraos-backend
Restart=always
RestartSec=5
Environment=RUST_LOG=coraos_backend=info

[Install]
WantedBy=multi-user.target
EOF

# Console menu on tty1
cat > "${CHROOT}/etc/systemd/system/coraos-console.service" << 'EOF'
[Unit]
Description=CoraOS Console
After=multi-user.target
Conflicts=getty@tty1.service

[Service]
ExecStart=/usr/local/bin/cora-welcome
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
User=cora
Environment=TERM=linux HOME=/home/cora
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

# ─── Apache Virtual Host ──────────────────────────────────────────────────────

mkdir -p "${CHROOT}/etc/apache2/sites-available"
cat > "${CHROOT}/etc/apache2/sites-available/coraos.conf" << 'EOF'
<VirtualHost *:80>
    ServerName coraos
    DocumentRoot /var/www/coraos

    <Directory /var/www/coraos>
        Options -Indexes +FollowSymLinks
        AllowOverride None
        Require all granted
    </Directory>

    ProxyPreserveHost On
    ProxyPass /api http://127.0.0.1:8080/api
    ProxyPassReverse /api http://127.0.0.1:8080/api

    RewriteEngine On
    RewriteCond %{HTTP:Upgrade} websocket [NC]
    RewriteCond %{HTTP:Connection} upgrade [NC]
    RewriteRule ^/api/ws$ ws://127.0.0.1:8080/api/ws [P,L]

    RewriteCond %{REQUEST_URI} !^/api
    RewriteCond %{DOCUMENT_ROOT}%{REQUEST_URI} !-f
    RewriteCond %{DOCUMENT_ROOT}%{REQUEST_URI} !-d
    RewriteRule . /index.html [L]
</VirtualHost>
EOF

# ─── Chroot Setup ─────────────────────────────────────────────────────────────

INFO "Mounting chroot filesystems..."
mount --bind /dev     "${CHROOT}/dev"
mount --bind /dev/pts "${CHROOT}/dev/pts"
mount --bind /proc    "${CHROOT}/proc"
mount --bind /sys     "${CHROOT}/sys"

INFO "Installing packages inside chroot..."
cat > "${CHROOT}/tmp/setup.sh" << 'SETUP'
#!/bin/bash
set -e
echo "coraos" > /etc/hostname
echo "127.0.0.1 localhost coraos" > /etc/hosts
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
    linux-image-amd64 live-boot live-config live-config-systemd \
    systemd systemd-sysv grub-common grub-pc-bin grub-efi-amd64-bin \
    plymouth plymouth-themes sudo network-manager iproute2 \
    bash coreutils util-linux console-setup dbus dbus-x11 \
    apache2 parted dosfstools e2fsprogs squashfs-tools os-prober \
    calamares calamares-settings-debian \
    xorg xinit openbox

# Apache setup
a2enmod proxy proxy_http proxy_wstunnel rewrite
a2dissite 000-default
a2ensite coraos

# Enable services
systemctl enable NetworkManager apache2 coraos.service coraos-console.service
systemctl disable getty@tty1.service 2>/dev/null || true

# Backend system user
useradd --system --no-create-home --shell /usr/sbin/nologin coraos 2>/dev/null || true
chown -R coraos:coraos /opt/coraos

# Sudoers for backend
cat > /etc/sudoers.d/coraos << 'SUDOERS'
coraos ALL=(root) NOPASSWD: /bin/systemctl, /usr/bin/systemctl
coraos ALL=(root) NOPASSWD: /usr/bin/apt-get, /usr/bin/apt
coraos ALL=(root) NOPASSWD: /bin/journalctl, /usr/bin/journalctl
coraos ALL=(root) NOPASSWD: /bin/tar, /usr/bin/tar
coraos ALL=(root) NOPASSWD: /sbin/mount, /bin/mount, /sbin/umount, /bin/umount
coraos ALL=(root) NOPASSWD: /sbin/mdadm, /usr/sbin/mdadm
coraos ALL=(root) NOPASSWD: /usr/sbin/useradd, /usr/sbin/userdel, /usr/sbin/chpasswd
coraos ALL=(root) NOPASSWD: /usr/bin/tee
SUDOERS
chmod 440 /etc/sudoers.d/coraos

# Interactive user (live session)
useradd -m -s /bin/bash cora
echo "cora:cora" | chpasswd
usermod -aG sudo cora
echo "cora ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Plymouth
plymouth-set-default-theme -R coraos

# Cleanup
apt-get clean
rm -rf /var/lib/apt/lists/*
SETUP

chmod +x "${CHROOT}/tmp/setup.sh"
chroot "${CHROOT}" /bin/bash /tmp/setup.sh
rm -f "${CHROOT}/tmp/setup.sh"
SUCCESS "Chroot configuration complete."

cleanup

# ─── Build ISO ────────────────────────────────────────────────────────────────

INFO "Preparing boot files..."
mkdir -p "${IMAGE}/live"
cp "${CHROOT}"/boot/vmlinuz-*  "${IMAGE}/live/vmlinuz"
cp "${CHROOT}"/boot/initrd.img-* "${IMAGE}/live/initrd.img"

INFO "Creating SquashFS..."
mksquashfs "${CHROOT}" "${IMAGE}/live/filesystem.squashfs" -comp xz -e boot
SUCCESS "Filesystem image created."

INFO "Writing GRUB config..."
mkdir -p "${IMAGE}/boot/grub"
cat > "${IMAGE}/boot/grub/grub.cfg" << 'EOF'
set default="0"
set timeout=10
insmod all_video

menuentry "CoraOS - Install" {
    linux /live/vmlinuz boot=live quiet splash
    initrd /live/initrd.img
}

menuentry "CoraOS - Live Mode" {
    linux /live/vmlinuz boot=live quiet splash
    initrd /live/initrd.img
}

menuentry "CoraOS - Safe Graphics" {
    linux /live/vmlinuz boot=live quiet splash nomodeset
    initrd /live/initrd.img
}
EOF

INFO "Building ISO..."
grub-mkrescue -o "${ISO_NAME}" "${IMAGE}"

SUCCESS "Done: ${WORKDIR}/${ISO_NAME}"
SUCCESS "  Dashboard: http://<ip>"
SUCCESS "  Console:   tty1 (auto)"
SUCCESS "  Installer: Launch Calamares from desktop or run 'sudo calamares'"
