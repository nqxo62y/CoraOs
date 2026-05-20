#!/bin/bash
# CoraOS Installation Script
# Installs the CoraOS server management platform on a Debian system.

set -e

INFO() { echo -e "\e[1;34m[INFO]\e[0m $1"; }
SUCCESS() { echo -e "\e[1;32m[OK]\e[0m $1"; }
ERROR() { echo -e "\e[1;31m[ERROR]\e[0m $1"; exit 1; }

if [ "$EUID" -ne 0 ]; then
    ERROR "This script must be run as root."
fi

INSTALL_DIR="/opt/coraos"

INFO "Creating CoraOS system user..."
if ! id -u coraos &>/dev/null; then
    useradd --system --no-create-home --shell /usr/sbin/nologin coraos
fi

INFO "Creating installation directories..."
mkdir -p "${INSTALL_DIR}/backend"
mkdir -p "${INSTALL_DIR}/frontend/dist"
mkdir -p "${INSTALL_DIR}/data"
mkdir -p "${INSTALL_DIR}/backups"
mkdir -p "${INSTALL_DIR}/logs"

INFO "Copying binary and frontend files..."
cp backend/target/release/coraos-backend "${INSTALL_DIR}/backend/"
cp -r frontend/dist/* "${INSTALL_DIR}/frontend/dist/"

INFO "Setting up environment configuration..."
if [ ! -f "${INSTALL_DIR}/.env" ]; then
    cp .env.example "${INSTALL_DIR}/.env"
    # Generate a random JWT secret
    JWT_SECRET=$(openssl rand -hex 64)
    sed -i "s/change_this_to_a_random_64_char_hex_string/${JWT_SECRET}/" "${INSTALL_DIR}/.env"
    SUCCESS "Generated random JWT secret"
fi

INFO "Setting file permissions..."
chown -R coraos:coraos "${INSTALL_DIR}"
chmod 750 "${INSTALL_DIR}/backend/coraos-backend"
chmod 600 "${INSTALL_DIR}/.env"

INFO "Installing systemd service..."
cp config/coraos.service /etc/systemd/system/coraos.service
systemctl daemon-reload
systemctl enable coraos.service

INFO "Starting CoraOS service..."
systemctl start coraos.service

SUCCESS "============================================"
SUCCESS " CoraOS installed successfully!"
SUCCESS " Access: http://$(hostname -I | awk '{print $1}'):8080"
SUCCESS " Default login: admin / admin123"
SUCCESS " CHANGE THE DEFAULT PASSWORD IMMEDIATELY!"
SUCCESS "============================================"
