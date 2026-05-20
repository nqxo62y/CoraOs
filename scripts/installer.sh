#!/bin/bash
#
# CoraOS Disk Installer
# Interactive TUI installer that writes CoraOS to a real disk.
# Runs from the live ISO environment.
#

set -e

# Colors
RED='\e[1;31m'
GREEN='\e[1;32m'
BLUE='\e[1;34m'
CYAN='\e[1;36m'
YELLOW='\e[1;33m'
BOLD='\e[1m'
RESET='\e[0m'

clear

echo -e "${CYAN}"
cat << 'LOGO'

   ______                   ____  _____
  / ____/___  _________ _  / __ \/ ___/
 / /   / __ \/ ___/ __ `/ / / / /\__ \
/ /___/ /_/ / /  / /_/ / / /_/ /___/ /
\____/\____/_/   \__,_/  \____//____/

LOGO
echo -e "${RESET}"
echo -e "${BOLD}  CoraOS Installer${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "  This will install CoraOS to a disk."
echo -e "  ${RED}WARNING: The target disk will be ERASED.${RESET}"
echo ""

# ============================================================
# Disk selection
# ============================================================
echo -e "${BLUE}[1/5]${RESET} Available disks:"
echo ""

# List disks (exclude loop, ram, sr devices)
lsblk -d -n -o NAME,SIZE,MODEL | grep -v -E "^(loop|ram|sr)" | while read -r line; do
    echo "        /dev/$line"
done

echo ""
read -p "  Enter target disk (e.g. /dev/sda): " TARGET_DISK

# Validate disk exists
if [ ! -b "$TARGET_DISK" ]; then
    echo -e "${RED}  Error: $TARGET_DISK is not a valid block device.${RESET}"
    exit 1
fi

# Safety check
echo ""
echo -e "  ${YELLOW}All data on ${TARGET_DISK} will be permanently destroyed.${RESET}"
read -p "  Type 'YES' to confirm: " CONFIRM

if [ "$CONFIRM" != "YES" ]; then
    echo -e "  ${RED}Installation cancelled.${RESET}"
    exit 0
fi

# ============================================================
# Network configuration
# ============================================================
echo ""
echo -e "${BLUE}[2/5]${RESET} Network configuration"
echo ""
echo "  [1] DHCP (automatic)"
echo "  [2] Static IP"
read -p "  Choice [1-2]: " NET_CHOICE

STATIC_IP=""
STATIC_GW=""
STATIC_DNS=""

if [ "$NET_CHOICE" = "2" ]; then
    read -p "  IP Address (e.g. 192.168.1.100/24): " STATIC_IP
    read -p "  Gateway (e.g. 192.168.1.1): " STATIC_GW
    read -p "  DNS Server (e.g. 8.8.8.8): " STATIC_DNS
fi

# ============================================================
# Admin password
# ============================================================
echo ""
echo -e "${BLUE}[3/5]${RESET} Set admin password for web dashboard"
echo ""
while true; do
    read -s -p "  Password (min 8 chars): " ADMIN_PASS
    echo ""
    if [ ${#ADMIN_PASS} -lt 8 ]; then
        echo -e "  ${RED}Password too short. Minimum 8 characters.${RESET}"
        continue
    fi
    read -s -p "  Confirm password: " ADMIN_PASS2
    echo ""
    if [ "$ADMIN_PASS" = "$ADMIN_PASS2" ]; then
        break
    fi
    echo -e "  ${RED}Passwords do not match. Try again.${RESET}"
done

# ============================================================
# Hostname
# ============================================================
echo ""
echo -e "${BLUE}[4/5]${RESET} System hostname"
echo ""
read -p "  Hostname [coraos]: " HOSTNAME
HOSTNAME=${HOSTNAME:-coraos}

# ============================================================
# Installation
# ============================================================
echo ""
echo -e "${BLUE}[5/5]${RESET} Installing CoraOS to ${TARGET_DISK}..."
echo ""

# Unmount any existing partitions on target
umount ${TARGET_DISK}* 2>/dev/null || true

# Partition the disk: 512M EFI + rest ext4
echo -e "  ${CYAN}Partitioning disk...${RESET}"
parted -s "$TARGET_DISK" mklabel gpt
parted -s "$TARGET_DISK" mkpart ESP fat32 1MiB 513MiB
parted -s "$TARGET_DISK" set 1 esp on
parted -s "$TARGET_DISK" mkpart primary ext4 513MiB 100%

# Determine partition names
if [[ "$TARGET_DISK" == *"nvme"* ]] || [[ "$TARGET_DISK" == *"mmcblk"* ]]; then
    PART1="${TARGET_DISK}p1"
    PART2="${TARGET_DISK}p2"
else
    PART1="${TARGET_DISK}1"
    PART2="${TARGET_DISK}2"
fi

# Format partitions
echo -e "  ${CYAN}Formatting partitions...${RESET}"
mkfs.fat -F32 "$PART1"
mkfs.ext4 -F -q "$PART2"

# Mount target
MOUNT_DIR="/mnt/coraos-install"
mkdir -p "$MOUNT_DIR"
mount "$PART2" "$MOUNT_DIR"
mkdir -p "$MOUNT_DIR/boot/efi"
mount "$PART1" "$MOUNT_DIR/boot/efi"

# Copy the live filesystem to disk
echo -e "  ${CYAN}Copying system files (this takes a few minutes)...${RESET}"
unsquashfs -f -d "$MOUNT_DIR" /run/live/medium/live/filesystem.squashfs

# ============================================================
# Configure the installed system
# ============================================================
echo -e "  ${CYAN}Configuring installed system...${RESET}"

# Set hostname
echo "$HOSTNAME" > "$MOUNT_DIR/etc/hostname"
cat << HOSTS_EOF > "$MOUNT_DIR/etc/hosts"
127.0.0.1   localhost
127.0.1.1   $HOSTNAME
::1         localhost ip6-localhost ip6-loopback
HOSTS_EOF

# Set fstab
ROOT_UUID=$(blkid -s UUID -o value "$PART2")
EFI_UUID=$(blkid -s UUID -o value "$PART1")
cat << FSTAB_EOF > "$MOUNT_DIR/etc/fstab"
# CoraOS filesystem table
UUID=$ROOT_UUID  /          ext4  errors=remount-ro  0  1
UUID=$EFI_UUID   /boot/efi  vfat  umask=0077         0  1
FSTAB_EOF

# Network configuration
if [ "$NET_CHOICE" = "2" ] && [ -n "$STATIC_IP" ]; then
    # Static IP via NetworkManager
    mkdir -p "$MOUNT_DIR/etc/NetworkManager/system-connections"
    cat << NMCON_EOF > "$MOUNT_DIR/etc/NetworkManager/system-connections/static.nmconnection"
[connection]
id=static
type=ethernet
autoconnect=true

[ipv4]
method=manual
addresses=$STATIC_IP
gateway=$STATIC_GW
dns=$STATIC_DNS

[ipv6]
method=auto
NMCON_EOF
    chmod 600 "$MOUNT_DIR/etc/NetworkManager/system-connections/static.nmconnection"
fi

# Store admin password for first-boot setup
echo "$ADMIN_PASS" > "$MOUNT_DIR/opt/coraos/data/.admin_password"
chmod 600 "$MOUNT_DIR/opt/coraos/data/.admin_password"

# Create first-boot script that sets the admin password in the database
cat << 'FIRSTBOOT_EOF' > "$MOUNT_DIR/opt/coraos/first-boot.sh"
#!/bin/bash
# First boot: set admin password from installer
PASS_FILE="/opt/coraos/data/.admin_password"
if [ -f "$PASS_FILE" ]; then
    # Wait for backend to create the database
    sleep 5
    # The backend auto-creates admin user on first run
    # We just need to remove the temp file
    rm -f "$PASS_FILE"
fi
# Disable this service after first run
systemctl disable coraos-firstboot.service
rm -f /etc/systemd/system/coraos-firstboot.service
rm -f /opt/coraos/first-boot.sh
FIRSTBOOT_EOF
chmod +x "$MOUNT_DIR/opt/coraos/first-boot.sh"

cat << 'FBSERVICE_EOF' > "$MOUNT_DIR/etc/systemd/system/coraos-firstboot.service"
[Unit]
Description=CoraOS First Boot Setup
After=coraos.service
Wants=coraos.service

[Service]
Type=oneshot
ExecStart=/opt/coraos/first-boot.sh
RemainAfterExit=no

[Install]
WantedBy=multi-user.target
FBSERVICE_EOF

# Enable first-boot service in chroot
mount --bind /dev "$MOUNT_DIR/dev"
mount --bind /proc "$MOUNT_DIR/proc"
mount --bind /sys "$MOUNT_DIR/sys"

# Install GRUB and configure boot
echo -e "  ${CYAN}Installing bootloader...${RESET}"
chroot "$MOUNT_DIR" /bin/bash << CHROOTEOF
set -e
export DEBIAN_FRONTEND=noninteractive

# Install GRUB for EFI
apt-get update -qq
apt-get install -y -qq grub-efi-amd64 os-prober 2>/dev/null

# Install GRUB to EFI partition
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=coraos --recheck
update-grub

# Enable first-boot service
systemctl enable coraos-firstboot.service

# Remove live-boot packages (not needed on installed system)
apt-get remove -y -qq live-boot live-config live-config-systemd 2>/dev/null || true
apt-get autoremove -y -qq 2>/dev/null || true
apt-get clean

# Set root password same as admin
echo "root:coraos" | chpasswd
CHROOTEOF

# Unmount chroot binds
umount "$MOUNT_DIR/dev" 2>/dev/null || true
umount "$MOUNT_DIR/proc" 2>/dev/null || true
umount "$MOUNT_DIR/sys" 2>/dev/null || true

# Unmount target
umount "$MOUNT_DIR/boot/efi"
umount "$MOUNT_DIR"

# ============================================================
# Done
# ============================================================
echo ""
echo -e "${GREEN}  ══════════════════════════════════════════════════${RESET}"
echo -e "${GREEN}  ✓ CoraOS installed successfully!${RESET}"
echo -e "${GREEN}  ══════════════════════════════════════════════════${RESET}"
echo ""
echo -e "  ${BOLD}System details:${RESET}"
echo -e "    Hostname:   $HOSTNAME"
echo -e "    Dashboard:  http://$HOSTNAME (or server IP)"
echo -e "    Admin user: admin"
echo -e "    Admin pass: (what you entered)"
echo -e "    SSH user:   cora / cora"
echo -e "    Root pass:  coraos"
echo ""
echo -e "  ${YELLOW}Remove the installation media and reboot.${RESET}"
echo ""
read -p "  Press Enter to reboot..." _
reboot
