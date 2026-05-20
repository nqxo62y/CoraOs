#!/bin/bash
#
# CoraOS Disk Installer
# Interactive setup wizard — like a real OS installer.
# Steps: Language → Disk → User Account → Network → Hostname → Install
#

set -e

# Colors
RED='\e[1;31m'
GREEN='\e[1;32m'
BLUE='\e[1;34m'
CYAN='\e[1;36m'
YELLOW='\e[1;33m'
BOLD='\e[1m'
DIM='\e[2m'
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
echo -e "${BOLD}  Welcome to CoraOS Setup${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "  This wizard will guide you through installing"
echo -e "  CoraOS on your server."
echo ""
echo -e "  ${RED}WARNING: The target disk will be completely erased.${RESET}"
echo ""
read -p "  Press Enter to begin setup..." _

# ============================================================
# Step 1: Disk Selection
# ============================================================
clear
echo -e "${BOLD}  CoraOS Setup${RESET} ${DIM}— Step 1 of 5${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "${BLUE}  Select Installation Disk${RESET}"
echo ""
echo -e "  Available disks:"
echo ""

# List disks
DISKS=()
while IFS= read -r line; do
    DISKS+=("$line")
done < <(lsblk -d -n -o NAME,SIZE,MODEL | grep -v -E "^(loop|ram|sr)")

for i in "${!DISKS[@]}"; do
    echo "    [$((i+1))] /dev/${DISKS[$i]}"
done

echo ""
read -p "  Select disk number [1-${#DISKS[@]}]: " DISK_NUM

if [[ "$DISK_NUM" -lt 1 ]] || [[ "$DISK_NUM" -gt "${#DISKS[@]}" ]]; then
    echo -e "  ${RED}Invalid selection.${RESET}"
    exit 1
fi

TARGET_DISK="/dev/$(echo "${DISKS[$((DISK_NUM-1))]}" | awk '{print $1}')"

if [ ! -b "$TARGET_DISK" ]; then
    echo -e "${RED}  Error: $TARGET_DISK is not a valid block device.${RESET}"
    exit 1
fi

echo ""
echo -e "  ${YELLOW}All data on ${TARGET_DISK} will be permanently destroyed.${RESET}"
read -p "  Type 'YES' to confirm: " CONFIRM

if [ "$CONFIRM" != "YES" ]; then
    echo -e "  ${RED}Installation cancelled.${RESET}"
    exit 0
fi

# ============================================================
# Step 2: Create Your Account
# ============================================================
clear
echo -e "${BOLD}  CoraOS Setup${RESET} ${DIM}— Step 2 of 5${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "${BLUE}  Create Your Account${RESET}"
echo ""
echo -e "  This account is used for:"
echo -e "    • SSH / console login"
echo -e "    • Web dashboard admin access"
echo -e "    • System administration (sudo)"
echo ""

# Full name
read -p "  Your name (display name): " USER_FULLNAME
USER_FULLNAME=${USER_FULLNAME:-Administrator}

# Username
while true; do
    read -p "  Username (lowercase, no spaces): " USERNAME
    USERNAME=${USERNAME:-admin}
    # Validate: lowercase, alphanumeric, starts with letter
    if [[ "$USERNAME" =~ ^[a-z][a-z0-9_-]{1,31}$ ]]; then
        break
    fi
    echo -e "  ${RED}Invalid username. Use lowercase letters, numbers, - or _ (2-32 chars).${RESET}"
done

# Password
echo ""
while true; do
    read -s -p "  Password (min 8 characters): " USER_PASS
    echo ""
    if [ ${#USER_PASS} -lt 8 ]; then
        echo -e "  ${RED}Password too short.${RESET}"
        continue
    fi
    read -s -p "  Confirm password: " USER_PASS2
    echo ""
    if [ "$USER_PASS" = "$USER_PASS2" ]; then
        break
    fi
    echo -e "  ${RED}Passwords do not match.${RESET}"
done

echo ""
echo -e "  ${GREEN}✓${RESET} Account: ${BOLD}${USERNAME}${RESET} (${USER_FULLNAME})"

# ============================================================
# Step 3: Network Configuration
# ============================================================
clear
echo -e "${BOLD}  CoraOS Setup${RESET} ${DIM}— Step 3 of 5${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "${BLUE}  Network Configuration${RESET}"
echo ""
echo "  [1] DHCP — automatic (recommended)"
echo "  [2] Static IP — manual configuration"
echo ""
read -p "  Choice [1-2]: " NET_CHOICE

STATIC_IP=""
STATIC_GW=""
STATIC_DNS=""

if [ "$NET_CHOICE" = "2" ]; then
    echo ""
    read -p "  IP Address (e.g. 192.168.1.100/24): " STATIC_IP
    read -p "  Gateway    (e.g. 192.168.1.1):      " STATIC_GW
    read -p "  DNS Server (e.g. 8.8.8.8):          " STATIC_DNS
    echo ""
    echo -e "  ${GREEN}✓${RESET} Static: ${STATIC_IP} via ${STATIC_GW}"
else
    echo ""
    echo -e "  ${GREEN}✓${RESET} Using DHCP (automatic)"
fi

# ============================================================
# Step 4: Hostname
# ============================================================
clear
echo -e "${BOLD}  CoraOS Setup${RESET} ${DIM}— Step 4 of 5${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "${BLUE}  Server Name${RESET}"
echo ""
echo -e "  Choose a hostname for this server."
echo -e "  This is how it appears on your network."
echo ""
read -p "  Hostname [coraos]: " HOSTNAME
HOSTNAME=${HOSTNAME:-coraos}

# Validate hostname
HOSTNAME=$(echo "$HOSTNAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-')
HOSTNAME=${HOSTNAME:-coraos}

echo ""
echo -e "  ${GREEN}✓${RESET} Hostname: ${BOLD}${HOSTNAME}${RESET}"

# ============================================================
# Step 5: Confirm & Install
# ============================================================
clear
echo -e "${BOLD}  CoraOS Setup${RESET} ${DIM}— Step 5 of 5${RESET}"
echo -e "  ════════════════════════════════════════"
echo ""
echo -e "${BLUE}  Review & Install${RESET}"
echo ""
echo -e "  ${BOLD}Disk:${RESET}      ${TARGET_DISK}"
echo -e "  ${BOLD}Account:${RESET}   ${USERNAME} (${USER_FULLNAME})"
echo -e "  ${BOLD}Network:${RESET}   $([ "$NET_CHOICE" = "2" ] && echo "Static: $STATIC_IP" || echo "DHCP")"
echo -e "  ${BOLD}Hostname:${RESET}  ${HOSTNAME}"
echo ""
echo -e "  ${YELLOW}This will erase ${TARGET_DISK} and install CoraOS.${RESET}"
echo ""
read -p "  Start installation? [y/N]: " START_INSTALL

if [[ "$START_INSTALL" != "y" && "$START_INSTALL" != "Y" ]]; then
    echo -e "  ${RED}Installation cancelled.${RESET}"
    exit 0
fi

# ============================================================
# Installation Process
# ============================================================
echo ""
echo -e "  ${CYAN}Installing CoraOS...${RESET}"
echo ""

# Unmount any existing partitions on target
umount ${TARGET_DISK}* 2>/dev/null || true

# Partition the disk: 512M EFI + rest ext4
echo -e "  [1/7] Partitioning disk..."
parted -s "$TARGET_DISK" mklabel gpt
parted -s "$TARGET_DISK" mkpart ESP fat32 1MiB 513MiB
parted -s "$TARGET_DISK" set 1 esp on
parted -s "$TARGET_DISK" mkpart primary ext4 513MiB 100%
sleep 1

# Determine partition names
if [[ "$TARGET_DISK" == *"nvme"* ]] || [[ "$TARGET_DISK" == *"mmcblk"* ]]; then
    PART1="${TARGET_DISK}p1"
    PART2="${TARGET_DISK}p2"
else
    PART1="${TARGET_DISK}1"
    PART2="${TARGET_DISK}2"
fi

# Format partitions
echo -e "  [2/7] Formatting partitions..."
mkfs.fat -F32 "$PART1" > /dev/null 2>&1
mkfs.ext4 -F -q "$PART2" > /dev/null 2>&1

# Mount target
MOUNT_DIR="/mnt/coraos-install"
mkdir -p "$MOUNT_DIR"
mount "$PART2" "$MOUNT_DIR"
mkdir -p "$MOUNT_DIR/boot/efi"
mount "$PART1" "$MOUNT_DIR/boot/efi"

# Copy the live filesystem to disk
echo -e "  [3/7] Copying system files (this takes a few minutes)..."
unsquashfs -f -d "$MOUNT_DIR" /run/live/medium/live/filesystem.squashfs > /dev/null 2>&1

# ============================================================
# Configure the installed system
# ============================================================
echo -e "  [4/7] Configuring system..."

# Hostname
echo "$HOSTNAME" > "$MOUNT_DIR/etc/hostname"
cat << HOSTS_EOF > "$MOUNT_DIR/etc/hosts"
127.0.0.1   localhost
127.0.1.1   $HOSTNAME
::1         localhost ip6-localhost ip6-loopback
HOSTS_EOF

# Fstab
ROOT_UUID=$(blkid -s UUID -o value "$PART2")
EFI_UUID=$(blkid -s UUID -o value "$PART1")
cat << FSTAB_EOF > "$MOUNT_DIR/etc/fstab"
UUID=$ROOT_UUID  /          ext4  errors=remount-ro  0  1
UUID=$EFI_UUID   /boot/efi  vfat  umask=0077         0  1
FSTAB_EOF

# Network
if [ "$NET_CHOICE" = "2" ] && [ -n "$STATIC_IP" ]; then
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

# ============================================================
# User Account Setup
# ============================================================
echo -e "  [5/7] Creating user account..."

# Remove the default 'cora' user and create the real user
cat << USERACCOUNT_EOF > "$MOUNT_DIR/tmp/setup_user.sh"
#!/bin/bash
set -e

# Remove default live user if it exists
userdel -r cora 2>/dev/null || true

# Create the real user
useradd -m -s /bin/bash -c "$USER_FULLNAME" "$USERNAME"
echo "$USERNAME:$USER_PASS" | chpasswd
usermod -aG sudo "$USERNAME"
echo "$USERNAME ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Set root password
echo "root:$USER_PASS" | chpasswd

# Auto-login this user on tty1
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat << 'GETTY_EOF' > /etc/systemd/system/getty@tty1.service.d/override.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin $USERNAME --noclear %I \$TERM
GETTY_EOF

# Set up cora-welcome for this user
cat << 'PROFILE_EOF' >> /home/$USERNAME/.bash_profile
/usr/local/bin/cora-welcome
PROFILE_EOF
chown $USERNAME:$USERNAME /home/$USERNAME/.bash_profile
USERACCOUNT_EOF

# We need to expand variables before writing
sed -i "s|\$USER_FULLNAME|${USER_FULLNAME}|g" "$MOUNT_DIR/tmp/setup_user.sh"
sed -i "s|\$USERNAME|${USERNAME}|g" "$MOUNT_DIR/tmp/setup_user.sh"
sed -i "s|\$USER_PASS|${USER_PASS}|g" "$MOUNT_DIR/tmp/setup_user.sh"
chmod +x "$MOUNT_DIR/tmp/setup_user.sh"

# Store dashboard admin credentials for first-boot
mkdir -p "$MOUNT_DIR/opt/coraos/data"
cat << ADMINEOF > "$MOUNT_DIR/opt/coraos/data/.setup_account"
USERNAME=$USERNAME
DISPLAY_NAME=$USER_FULLNAME
ADMINEOF
chmod 600 "$MOUNT_DIR/opt/coraos/data/.setup_account"

# ============================================================
# Bootloader & Final Config
# ============================================================
echo -e "  [6/7] Installing bootloader..."

mount --bind /dev "$MOUNT_DIR/dev"
mount --bind /proc "$MOUNT_DIR/proc"
mount --bind /sys "$MOUNT_DIR/sys"

chroot "$MOUNT_DIR" /bin/bash << CHROOTEOF
set -e
export DEBIAN_FRONTEND=noninteractive

# Run user setup
bash /tmp/setup_user.sh
rm -f /tmp/setup_user.sh

# Install GRUB
apt-get update -qq 2>/dev/null
apt-get install -y -qq grub-efi-amd64 2>/dev/null

grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=coraos --recheck 2>/dev/null
update-grub 2>/dev/null

# Remove live-boot packages
apt-get remove -y -qq live-boot live-config live-config-systemd 2>/dev/null || true
apt-get autoremove -y -qq 2>/dev/null || true
apt-get clean
CHROOTEOF

umount "$MOUNT_DIR/dev" 2>/dev/null || true
umount "$MOUNT_DIR/proc" 2>/dev/null || true
umount "$MOUNT_DIR/sys" 2>/dev/null || true

echo -e "  [7/7] Finalizing..."

# Unmount target
umount "$MOUNT_DIR/boot/efi"
umount "$MOUNT_DIR"

# ============================================================
# Done
# ============================================================
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
echo -e "${GREEN}  ══════════════════════════════════════════════════════${RESET}"
echo -e "${GREEN}  ✓ CoraOS has been installed successfully!${RESET}"
echo -e "${GREEN}  ══════════════════════════════════════════════════════${RESET}"
echo ""
echo -e "  ${BOLD}Your system is ready.${RESET}"
echo ""
echo -e "  ┌─────────────────────────────────────────────┐"
echo -e "  │  Dashboard:  http://${HOSTNAME}              │"
echo -e "  │  Username:   ${USERNAME}                     │"
echo -e "  │  Password:   (what you set during setup)    │"
echo -e "  └─────────────────────────────────────────────┘"
echo ""
echo -e "  ${DIM}The web dashboard uses the same credentials${RESET}"
echo -e "  ${DIM}as your system account.${RESET}"
echo ""
echo -e "  ${YELLOW}Remove the installation media and press Enter to reboot.${RESET}"
echo ""
read -p "  Press Enter to reboot..." _
reboot
