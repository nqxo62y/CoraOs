#!/bin/bash
# =====================================================================
# CoraOS ISO Builder Script
# Debian-based Live OS with Custom Rust Dashboard and Plymouth Splash
# =====================================================================

set -e

# Visual formatting helper functions
INFO() { echo -e "\e[1;34m[INFO]\e[0m $1"; }
SUCCESS() { echo -e "\e[1;32m[SUCCESS]\e[0m $1"; }
ERROR() { echo -e "\e[1;31m[ERROR]\e[0m $1"; exit 1; }

# Prerequisites Check (Must run as root for debootstrap and mounting)
if [ "$EUID" -ne 0 ]; then
  ERROR "This script must be run as root. Please run with sudo."
fi

# Variables
WORKDIR=$(pwd)
CHROOT_DIR="${WORKDIR}/chroot"
IMAGE_DIR="${WORKDIR}/image"
ISO_NAME="coraos.iso"

# Cleanup trap to ensure mounts are unmounted even if script fails
cleanup() {
  INFO "Cleaning up temporary mounts..."
  if mountpoint -q "${CHROOT_DIR}/proc"; then umount -lf "${CHROOT_DIR}/proc"; fi
  if mountpoint -q "${CHROOT_DIR}/sys"; then umount -lf "${CHROOT_DIR}/sys"; fi
  if mountpoint -q "${CHROOT_DIR}/dev/pts"; then umount -lf "${CHROOT_DIR}/dev/pts"; fi
  if mountpoint -q "${CHROOT_DIR}/dev"; then umount -lf "${CHROOT_DIR}/dev"; fi
  SUCCESS "Cleanup finished."
}
trap cleanup EXIT

# 1. Install Host Dependencies
INFO "Installing required build dependencies on host..."
apt-get update
apt-get install -y debootstrap squashfs-tools xorriso grub-pc-bin grub-efi-amd64-bin mtools python3-pillow python3-pip

# 2. Extract and Convert Icon to PNG
INFO "Processing custom boot logo..."
if [ -f "logo/logo.ico" ]; then
  python3 -c "from PIL import Image; Image.open('logo/logo.ico').save('logo/logo.png')"
  SUCCESS "Converted logo/logo.ico to logo/logo.png."
else
  ERROR "Could not find logo/logo.ico inside the logo directory!"
fi

# 3. Compile custom Rust welcome utility
INFO "Building custom Rust welcome utility..."
if [ -d "cora-welcome" ]; then
  # Make sure Rust cargo is installed on host
  if ! command -v cargo &> /dev/null; then
    INFO "Cargo not found. Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
  fi
  
  cd cora-welcome
  cargo build --release
  cd ..
  SUCCESS "Rust welcome utility compiled successfully!"
else
  ERROR "cora-welcome directory not found!"
fi

# 4. Bootstrap a minimal Debian system
INFO "Bootstrapping Debian system (stable/bookworm)..."
rm -rf "${CHROOT_DIR}" "${IMAGE_DIR}"
mkdir -p "${CHROOT_DIR}"
debootstrap --arch=amd64 stable "${CHROOT_DIR}" http://deb.debian.org/debian/
SUCCESS "Debian base bootstrapped successfully."

# 5. Copy configuration assets and binaries into chroot
INFO "Deploying custom system config, theme files, and Rust welcome binary into the chroot..."

# Copy Rust welcome program
mkdir -p "${CHROOT_DIR}/usr/local/bin"
cp cora-welcome/target/release/cora-welcome "${CHROOT_DIR}/usr/local/bin/"
chmod +x "${CHROOT_DIR}/usr/local/bin/cora-welcome"

# Copy Plymouth custom theme files
PLYMOUTH_THEME_DIR="${CHROOT_DIR}/usr/share/plymouth/themes/coraos"
mkdir -p "${PLYMOUTH_THEME_DIR}"
cp logo/logo.png "${PLYMOUTH_THEME_DIR}/logo.png"
cp plymouth/coraos.plymouth "${PLYMOUTH_THEME_DIR}/coraos.plymouth"
cp plymouth/coraos.script "${PLYMOUTH_THEME_DIR}/coraos.script"

# 6. Bind mount filesystems to chroot for configuration
INFO "Mounting pseudo-filesystems into chroot..."
mount --bind /dev "${CHROOT_DIR}/dev"
mount --bind /dev/pts "${CHROOT_DIR}/dev/pts"
mount --bind /proc "${CHROOT_DIR}/proc"
mount --bind /sys "${CHROOT_DIR}/sys"

# 7. Run Chroot Setup Script
INFO "Configuring Debian OS in chroot..."
cat << 'EOF' > "${CHROOT_DIR}/tmp/chroot_setup.sh"
#!/bin/bash
set -e

# Set hostname
echo "coraos" > /etc/hostname
echo "127.0.0.1   localhost coraos" > /etc/hosts

# Configure non-interactive frontend for apt
export DEBIAN_FRONTEND=noninteractive

# Update and install system dependencies inside the guest OS
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
  dbus

# Configure networking
systemctl enable NetworkManager

# Create custom live user 'cora'
useradd -m -s /bin/bash cora
echo "cora:cora" | chpasswd
usermod -aG sudo cora
echo "cora ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Configure systemd to autologin 'cora' on tty1
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat << 'GETTY_EOF' > /etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin cora --noclear %I $TERM
GETTY_EOF

# Set custom welcome script to launch on login in .bashrc
cat << 'BASHRC_EOF' >> /home/cora/.bashrc

# Automatically run CoraOS Welcome Interface
if [ -f /usr/local/bin/cora-welcome ]; then
    /usr/local/bin/cora-welcome
    exit
fi
BASHRC_EOF
chown cora:cora /home/cora/.bashrc

# Set Plymouth default theme and rebuild Initramfs
INFO_GUEST() { echo -e "\e[1;35m[CHROOT]\e[0m $1"; }
INFO_GUEST "Configuring custom boot splash logo..."
plymouth-set-default-theme -R coraos

# Clean up apt caches to minimize ISO size
apt-get clean
rm -rf /var/lib/apt/lists/*
EOF

chmod +x "${CHROOT_DIR}/tmp/chroot_setup.sh"
chroot "${CHROOT_DIR}" /bin/bash /tmp/chroot_setup.sh
rm -f "${CHROOT_DIR}/tmp/chroot_setup.sh"
SUCCESS "Guest system configuration complete."

# 8. Unmount chroot
cleanup

# 9. Extract kernel and initrd outside the chroot for GRUB booting
INFO "Preparing boot files..."
mkdir -p "${IMAGE_DIR}/live"
KERNEL_PATH=$(ls -1 "${CHROOT_DIR}/boot/vmlinuz-"* | head -n 1)
INITRD_PATH=$(ls -1 "${CHROOT_DIR}/boot/initrd.img-"* | head -n 1)

cp "${KERNEL_PATH}" "${IMAGE_DIR}/live/vmlinuz"
cp "${INITRD_PATH}" "${IMAGE_DIR}/live/initrd.img"
SUCCESS "Kernel and Initramfs exported to live image folder."

# 10. Compress the root filesystem into SquashFS
INFO "Creating SquashFS image (this may take a couple of minutes)..."
mksquashfs "${CHROOT_DIR}" "${IMAGE_DIR}/live/filesystem.squashfs" -comp xz -e boot
SUCCESS "SquashFS root filesystem generated."

# 11. Create custom bootloader GRUB configuration
INFO "Creating bootloader configuration..."
mkdir -p "${IMAGE_DIR}/boot/grub"
cat << 'EOF' > "${IMAGE_DIR}/boot/grub/grub.cfg"
set default="0"
set timeout=3

# Load graphic elements
insmod all_video

menuentry "CoraOS Live (GNU/Linux Debian-based)" {
    linux /live/vmlinuz boot=live quiet splash plymouth.ignore-serial-consoles vt.global_cursor_default=0
    initrd /live/initrd.img
}
EOF
SUCCESS "GRUB configuration created."

# 12. Package the custom hybrid live ISO
INFO "Compiling the bootable hybrid UEFI/BIOS ISO..."
grub-mkrescue -o "${ISO_NAME}" "${IMAGE_DIR}"

SUCCESS "=========================================================="
SUCCESS " CoraOS ISO generated successfully at: ${WORKDIR}/${ISO_NAME}"
SUCCESS "=========================================================="
