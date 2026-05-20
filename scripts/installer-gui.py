#!/usr/bin/env python3
"""
CoraOS Graphical Installer
A GTK3-based setup wizard for installing CoraOS to disk.
"""

import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, Gdk, GLib, Pango
import subprocess
import threading
import os
import re

# ============================================================
# Styling
# ============================================================
CSS = b"""
window {
    background-color: #0f1117;
}
.title-label {
    font-size: 28px;
    font-weight: bold;
    color: #e4e7ec;
}
.subtitle-label {
    font-size: 14px;
    color: #9ca3b4;
}
.step-label {
    font-size: 12px;
    color: #6b7280;
}
.heading {
    font-size: 20px;
    font-weight: bold;
    color: #e4e7ec;
}
.description {
    font-size: 13px;
    color: #9ca3b4;
}
.field-label {
    font-size: 13px;
    font-weight: 500;
    color: #9ca3b4;
}
entry {
    background-color: #242836;
    color: #e4e7ec;
    border: 1px solid #2d3244;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 14px;
}
entry:focus {
    border-color: #6366f1;
}
combobox {
    background-color: #242836;
    color: #e4e7ec;
    border: 1px solid #2d3244;
    border-radius: 6px;
}
.btn-primary {
    background-color: #6366f1;
    color: white;
    border-radius: 6px;
    padding: 10px 24px;
    font-size: 14px;
    font-weight: 500;
}
.btn-primary:hover {
    background-color: #818cf8;
}
.btn-secondary {
    background-color: #242836;
    color: #e4e7ec;
    border: 1px solid #2d3244;
    border-radius: 6px;
    padding: 10px 24px;
    font-size: 14px;
}
.btn-danger {
    background-color: #ef4444;
    color: white;
    border-radius: 6px;
    padding: 10px 24px;
    font-size: 14px;
    font-weight: 500;
}
.sidebar {
    background-color: #1a1d27;
    border-right: 1px solid #2d3244;
}
.sidebar-item {
    font-size: 13px;
    color: #6b7280;
    padding: 8px 16px;
}
.sidebar-item-active {
    font-size: 13px;
    color: #6366f1;
    font-weight: bold;
    padding: 8px 16px;
}
.sidebar-item-done {
    font-size: 13px;
    color: #22c55e;
    padding: 8px 16px;
}
.error-label {
    font-size: 12px;
    color: #ef4444;
}
.success-box {
    background-color: #1a1d27;
    border: 1px solid #22c55e;
    border-radius: 8px;
    padding: 20px;
}
progressbar trough {
    background-color: #242836;
    border-radius: 4px;
    min-height: 8px;
}
progressbar progress {
    background-color: #6366f1;
    border-radius: 4px;
    min-height: 8px;
}
"""

# ============================================================
# Helper functions
# ============================================================

def get_disks():
    """Get list of available disks."""
    result = subprocess.run(
        ['lsblk', '-d', '-n', '-o', 'NAME,SIZE,MODEL'],
        capture_output=True, text=True
    )
    disks = []
    for line in result.stdout.strip().split('\n'):
        if not line:
            continue
        parts = line.split(None, 2)
        name = parts[0] if parts else ''
        if name and not name.startswith(('loop', 'ram', 'sr')):
            size = parts[1] if len(parts) > 1 else '?'
            model = parts[2] if len(parts) > 2 else 'Unknown'
            disks.append({
                'device': f'/dev/{name}',
                'size': size,
                'model': model.strip()
            })
    return disks


def validate_username(username):
    """Validate username format."""
    return bool(re.match(r'^[a-z][a-z0-9_-]{1,31}$', username))


# ============================================================
# Installer Window
# ============================================================

class InstallerWindow(Gtk.Window):
    def __init__(self):
        super().__init__(title="CoraOS Setup")
        self.set_default_size(900, 600)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.set_resizable(False)

        # Apply CSS
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(CSS)
        Gtk.StyleContext.add_provider_for_screen(
            Gdk.Screen.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        # State
        self.current_step = 0
        self.config = {
            'disk': '',
            'fullname': '',
            'username': '',
            'password': '',
            'network': 'dhcp',
            'static_ip': '',
            'static_gw': '',
            'static_dns': '',
            'hostname': 'coraos',
        }

        # Steps
        self.steps = [
            'Welcome',
            'Disk',
            'Account',
            'Network',
            'Install',
        ]

        self.build_ui()
        self.show_step(0)

    def build_ui(self):
        """Build the main layout."""
        main_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        self.add(main_box)

        # Sidebar
        self.sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        self.sidebar.set_size_request(200, -1)
        self.sidebar.get_style_context().add_class('sidebar')
        self.sidebar.set_margin_top(30)
        self.sidebar.set_margin_bottom(30)

        # Logo in sidebar
        logo_label = Gtk.Label(label="CoraOS")
        logo_label.get_style_context().add_class('title-label')
        logo_label.set_margin_bottom(30)
        logo_label.set_margin_top(10)
        self.sidebar.pack_start(logo_label, False, False, 0)

        # Step labels
        self.step_labels = []
        for step_name in self.steps:
            lbl = Gtk.Label(label=f"  {step_name}")
            lbl.set_xalign(0)
            lbl.get_style_context().add_class('sidebar-item')
            self.sidebar.pack_start(lbl, False, False, 0)
            self.step_labels.append(lbl)

        main_box.pack_start(self.sidebar, False, False, 0)

        # Content area
        self.content_stack = Gtk.Stack()
        self.content_stack.set_transition_type(Gtk.StackTransitionType.SLIDE_LEFT)
        self.content_stack.set_transition_duration(200)

        # Build each page
        self.content_stack.add_named(self.build_welcome_page(), 'welcome')
        self.content_stack.add_named(self.build_disk_page(), 'disk')
        self.content_stack.add_named(self.build_account_page(), 'account')
        self.content_stack.add_named(self.build_network_page(), 'network')
        self.content_stack.add_named(self.build_install_page(), 'install')

        main_box.pack_start(self.content_stack, True, True, 0)

    def update_sidebar(self):
        """Update sidebar step indicators."""
        for i, lbl in enumerate(self.step_labels):
            ctx = lbl.get_style_context()
            ctx.remove_class('sidebar-item')
            ctx.remove_class('sidebar-item-active')
            ctx.remove_class('sidebar-item-done')
            if i < self.current_step:
                ctx.add_class('sidebar-item-done')
                lbl.set_text(f"  ✓ {self.steps[i]}")
            elif i == self.current_step:
                ctx.add_class('sidebar-item-active')
                lbl.set_text(f"  ● {self.steps[i]}")
            else:
                ctx.add_class('sidebar-item')
                lbl.set_text(f"  {self.steps[i]}")

    def show_step(self, step):
        """Navigate to a step."""
        self.current_step = step
        self.update_sidebar()
        pages = ['welcome', 'disk', 'account', 'network', 'install']
        self.content_stack.set_visible_child_name(pages[step])

    def make_page(self, heading, description):
        """Create a standard page layout."""
        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        page.set_margin_top(40)
        page.set_margin_bottom(40)
        page.set_margin_start(40)
        page.set_margin_end(40)

        h = Gtk.Label(label=heading)
        h.set_xalign(0)
        h.get_style_context().add_class('heading')
        page.pack_start(h, False, False, 0)

        d = Gtk.Label(label=description)
        d.set_xalign(0)
        d.set_line_wrap(True)
        d.get_style_context().add_class('description')
        page.pack_start(d, False, False, 0)

        sep = Gtk.Separator()
        page.pack_start(sep, False, False, 8)

        return page

    def make_nav_buttons(self, back_cb=None, next_cb=None, next_label="Next"):
        """Create navigation buttons."""
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        box.set_halign(Gtk.Align.END)

        if back_cb:
            btn_back = Gtk.Button(label="Back")
            btn_back.get_style_context().add_class('btn-secondary')
            btn_back.connect('clicked', back_cb)
            box.pack_start(btn_back, False, False, 0)

        btn_next = Gtk.Button(label=next_label)
        btn_next.get_style_context().add_class('btn-primary')
        if next_cb:
            btn_next.connect('clicked', next_cb)
        box.pack_start(btn_next, False, False, 0)

        return box

    # ============================================================
    # Page: Welcome
    # ============================================================
    def build_welcome_page(self):
        page = self.make_page(
            "Welcome to CoraOS",
            "This wizard will install CoraOS on your server.\n"
            "You'll set up your disk, create an account, and configure networking."
        )

        info = Gtk.Label(
            label="CoraOS is a Debian-based server management platform with a "
                  "web dashboard for monitoring and managing your system."
        )
        info.set_xalign(0)
        info.set_line_wrap(True)
        info.get_style_context().add_class('description')
        info.set_margin_top(20)
        page.pack_start(info, False, False, 0)

        warn = Gtk.Label(label="⚠  The target disk will be completely erased.")
        warn.set_xalign(0)
        warn.get_style_context().add_class('error-label')
        warn.set_margin_top(20)
        page.pack_start(warn, False, False, 0)

        spacer = Gtk.Box()
        page.pack_start(spacer, True, True, 0)

        nav = self.make_nav_buttons(next_cb=self.on_welcome_next, next_label="Start Setup")
        page.pack_end(nav, False, False, 0)

        return page

    def on_welcome_next(self, btn):
        self.show_step(1)

    # ============================================================
    # Page: Disk Selection
    # ============================================================
    def build_disk_page(self):
        page = self.make_page(
            "Select Installation Disk",
            "Choose the disk where CoraOS will be installed. All data on the selected disk will be erased."
        )

        # Disk list
        self.disk_store = Gtk.ListStore(str, str, str)  # device, size, model
        self.disk_tree = Gtk.TreeView(model=self.disk_store)
        self.disk_tree.set_headers_visible(True)

        col1 = Gtk.TreeViewColumn("Device", Gtk.CellRendererText(), text=0)
        col2 = Gtk.TreeViewColumn("Size", Gtk.CellRendererText(), text=1)
        col3 = Gtk.TreeViewColumn("Model", Gtk.CellRendererText(), text=2)
        col1.set_min_width(120)
        col2.set_min_width(80)
        self.disk_tree.append_column(col1)
        self.disk_tree.append_column(col2)
        self.disk_tree.append_column(col3)

        scroll = Gtk.ScrolledWindow()
        scroll.set_min_content_height(200)
        scroll.add(self.disk_tree)
        page.pack_start(scroll, True, True, 0)

        # Refresh button
        refresh_btn = Gtk.Button(label="Refresh Disks")
        refresh_btn.get_style_context().add_class('btn-secondary')
        refresh_btn.connect('clicked', self.refresh_disks)
        page.pack_start(refresh_btn, False, False, 0)

        self.disk_error = Gtk.Label()
        self.disk_error.get_style_context().add_class('error-label')
        self.disk_error.set_xalign(0)
        page.pack_start(self.disk_error, False, False, 0)

        nav = self.make_nav_buttons(
            back_cb=lambda b: self.show_step(0),
            next_cb=self.on_disk_next
        )
        page.pack_end(nav, False, False, 0)

        # Load disks
        GLib.idle_add(self.refresh_disks, None)

        return page

    def refresh_disks(self, btn):
        self.disk_store.clear()
        for disk in get_disks():
            self.disk_store.append([disk['device'], disk['size'], disk['model']])

    def on_disk_next(self, btn):
        sel = self.disk_tree.get_selection()
        model, tree_iter = sel.get_selected()
        if tree_iter is None:
            self.disk_error.set_text("Please select a disk.")
            return
        self.config['disk'] = model[tree_iter][0]
        self.disk_error.set_text("")
        self.show_step(2)

    # ============================================================
    # Page: Account Setup
    # ============================================================
    def build_account_page(self):
        page = self.make_page(
            "Create Your Account",
            "Set up your personal account. This will be used for console login, "
            "SSH access, and the web dashboard."
        )

        grid = Gtk.Grid(column_spacing=12, row_spacing=12)
        grid.set_margin_top(10)

        # Full name
        lbl = Gtk.Label(label="Your Name")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        grid.attach(lbl, 0, 0, 1, 1)
        self.entry_fullname = Gtk.Entry()
        self.entry_fullname.set_placeholder_text("e.g. John Smith")
        self.entry_fullname.set_hexpand(True)
        grid.attach(self.entry_fullname, 1, 0, 1, 1)

        # Username
        lbl = Gtk.Label(label="Username")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        grid.attach(lbl, 0, 1, 1, 1)
        self.entry_username = Gtk.Entry()
        self.entry_username.set_placeholder_text("lowercase, no spaces")
        grid.attach(self.entry_username, 1, 1, 1, 1)

        # Hostname
        lbl = Gtk.Label(label="Server Name")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        grid.attach(lbl, 0, 2, 1, 1)
        self.entry_hostname = Gtk.Entry()
        self.entry_hostname.set_text("coraos")
        self.entry_hostname.set_placeholder_text("hostname")
        grid.attach(self.entry_hostname, 1, 2, 1, 1)

        # Password
        lbl = Gtk.Label(label="Password")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        grid.attach(lbl, 0, 3, 1, 1)
        self.entry_password = Gtk.Entry()
        self.entry_password.set_visibility(False)
        self.entry_password.set_placeholder_text("minimum 8 characters")
        grid.attach(self.entry_password, 1, 3, 1, 1)

        # Confirm password
        lbl = Gtk.Label(label="Confirm")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        grid.attach(lbl, 0, 4, 1, 1)
        self.entry_password2 = Gtk.Entry()
        self.entry_password2.set_visibility(False)
        self.entry_password2.set_placeholder_text("repeat password")
        grid.attach(self.entry_password2, 1, 4, 1, 1)

        page.pack_start(grid, False, False, 0)

        self.account_error = Gtk.Label()
        self.account_error.get_style_context().add_class('error-label')
        self.account_error.set_xalign(0)
        self.account_error.set_margin_top(8)
        page.pack_start(self.account_error, False, False, 0)

        spacer = Gtk.Box()
        page.pack_start(spacer, True, True, 0)

        nav = self.make_nav_buttons(
            back_cb=lambda b: self.show_step(1),
            next_cb=self.on_account_next
        )
        page.pack_end(nav, False, False, 0)

        return page

    def on_account_next(self, btn):
        fullname = self.entry_fullname.get_text().strip()
        username = self.entry_username.get_text().strip().lower()
        hostname = self.entry_hostname.get_text().strip().lower()
        password = self.entry_password.get_text()
        password2 = self.entry_password2.get_text()

        if not fullname:
            self.account_error.set_text("Please enter your name.")
            return
        if not validate_username(username):
            self.account_error.set_text(
                "Invalid username. Use lowercase letters, numbers, - or _ (2-32 chars)."
            )
            return
        if len(password) < 8:
            self.account_error.set_text("Password must be at least 8 characters.")
            return
        if password != password2:
            self.account_error.set_text("Passwords do not match.")
            return

        hostname = re.sub(r'[^a-z0-9-]', '', hostname) or 'coraos'

        self.config['fullname'] = fullname
        self.config['username'] = username
        self.config['password'] = password
        self.config['hostname'] = hostname
        self.account_error.set_text("")
        self.show_step(3)

    # ============================================================
    # Page: Network
    # ============================================================
    def build_network_page(self):
        page = self.make_page(
            "Network Configuration",
            "Choose how this server connects to your network."
        )

        # DHCP radio
        self.radio_dhcp = Gtk.RadioButton.new_with_label(None, "DHCP — Automatic (recommended)")
        page.pack_start(self.radio_dhcp, False, False, 4)

        # Static radio
        self.radio_static = Gtk.RadioButton.new_with_label_from_widget(
            self.radio_dhcp, "Static IP — Manual configuration"
        )
        page.pack_start(self.radio_static, False, False, 4)

        # Static fields
        self.static_box = Gtk.Grid(column_spacing=12, row_spacing=10)
        self.static_box.set_margin_top(12)
        self.static_box.set_margin_start(24)

        lbl = Gtk.Label(label="IP Address")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        self.static_box.attach(lbl, 0, 0, 1, 1)
        self.entry_ip = Gtk.Entry()
        self.entry_ip.set_placeholder_text("192.168.1.100/24")
        self.entry_ip.set_hexpand(True)
        self.static_box.attach(self.entry_ip, 1, 0, 1, 1)

        lbl = Gtk.Label(label="Gateway")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        self.static_box.attach(lbl, 0, 1, 1, 1)
        self.entry_gw = Gtk.Entry()
        self.entry_gw.set_placeholder_text("192.168.1.1")
        self.static_box.attach(self.entry_gw, 1, 1, 1, 1)

        lbl = Gtk.Label(label="DNS Server")
        lbl.set_xalign(0)
        lbl.get_style_context().add_class('field-label')
        self.static_box.attach(lbl, 0, 2, 1, 1)
        self.entry_dns = Gtk.Entry()
        self.entry_dns.set_placeholder_text("8.8.8.8")
        self.static_box.attach(self.entry_dns, 1, 2, 1, 1)

        page.pack_start(self.static_box, False, False, 0)
        self.static_box.set_sensitive(False)

        self.radio_dhcp.connect('toggled', self.on_net_toggle)
        self.radio_static.connect('toggled', self.on_net_toggle)

        spacer = Gtk.Box()
        page.pack_start(spacer, True, True, 0)

        nav = self.make_nav_buttons(
            back_cb=lambda b: self.show_step(2),
            next_cb=self.on_network_next,
            next_label="Install"
        )
        page.pack_end(nav, False, False, 0)

        return page

    def on_net_toggle(self, radio):
        self.static_box.set_sensitive(self.radio_static.get_active())

    def on_network_next(self, btn):
        if self.radio_static.get_active():
            self.config['network'] = 'static'
            self.config['static_ip'] = self.entry_ip.get_text().strip()
            self.config['static_gw'] = self.entry_gw.get_text().strip()
            self.config['static_dns'] = self.entry_dns.get_text().strip()
        else:
            self.config['network'] = 'dhcp'

        # Confirm dialog
        dialog = Gtk.MessageDialog(
            transient_for=self,
            modal=True,
            message_type=Gtk.MessageType.WARNING,
            buttons=Gtk.ButtonsType.YES_NO,
            text=f"Erase {self.config['disk']} and install CoraOS?"
        )
        dialog.format_secondary_text(
            "This will permanently destroy all data on the selected disk."
        )
        response = dialog.run()
        dialog.destroy()

        if response == Gtk.ResponseType.YES:
            self.show_step(4)
            GLib.idle_add(self.start_installation)

    # ============================================================
    # Page: Installation Progress
    # ============================================================
    def build_install_page(self):
        page = self.make_page(
            "Installing CoraOS",
            "Please wait while CoraOS is being installed on your system."
        )

        self.progress_bar = Gtk.ProgressBar()
        self.progress_bar.set_margin_top(20)
        page.pack_start(self.progress_bar, False, False, 0)

        self.progress_label = Gtk.Label(label="Preparing...")
        self.progress_label.set_xalign(0)
        self.progress_label.get_style_context().add_class('description')
        self.progress_label.set_margin_top(8)
        page.pack_start(self.progress_label, False, False, 0)

        # Log output
        self.log_view = Gtk.TextView()
        self.log_view.set_editable(False)
        self.log_view.set_cursor_visible(False)
        self.log_view.modify_font(Pango.FontDescription('monospace 10'))
        scroll = Gtk.ScrolledWindow()
        scroll.set_min_content_height(200)
        scroll.add(self.log_view)
        scroll.set_margin_top(16)
        page.pack_start(scroll, True, True, 0)

        # Done area (hidden initially)
        self.done_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.done_box.set_margin_top(20)

        done_label = Gtk.Label(label="✓ CoraOS installed successfully!")
        done_label.get_style_context().add_class('heading')
        done_label.set_xalign(0)
        self.done_box.pack_start(done_label, False, False, 0)

        self.done_info = Gtk.Label()
        self.done_info.set_xalign(0)
        self.done_info.set_line_wrap(True)
        self.done_info.get_style_context().add_class('description')
        self.done_box.pack_start(self.done_info, False, False, 0)

        reboot_btn = Gtk.Button(label="Reboot Now")
        reboot_btn.get_style_context().add_class('btn-primary')
        reboot_btn.connect('clicked', lambda b: subprocess.run(['reboot']))
        self.done_box.pack_start(reboot_btn, False, False, 0)

        self.done_box.set_no_show_all(True)
        page.pack_start(self.done_box, False, False, 0)

        return page

    def log(self, text):
        """Append text to the log view (thread-safe)."""
        def _append():
            buf = self.log_view.get_buffer()
            buf.insert(buf.get_end_iter(), text + '\n')
            # Auto-scroll
            mark = buf.get_insert()
            self.log_view.scroll_to_mark(mark, 0, True, 0, 1)
        GLib.idle_add(_append)

    def set_progress(self, fraction, text):
        """Update progress bar (thread-safe)."""
        def _update():
            self.progress_bar.set_fraction(fraction)
            self.progress_label.set_text(text)
        GLib.idle_add(_update)

    def run_cmd(self, cmd, check=True):
        """Run a shell command and log output."""
        self.log(f"$ {cmd}")
        result = subprocess.run(
            cmd, shell=True,
            capture_output=True, text=True
        )
        if result.stdout.strip():
            for line in result.stdout.strip().split('\n')[:5]:
                self.log(f"  {line}")
        if result.returncode != 0 and result.stderr.strip():
            for line in result.stderr.strip().split('\n')[:3]:
                self.log(f"  [err] {line}")
        if check and result.returncode != 0:
            raise RuntimeError(f"Command failed: {cmd}")
        return result

    def start_installation(self):
        """Start the installation in a background thread."""
        thread = threading.Thread(target=self.do_install, daemon=True)
        thread.start()

    def do_install(self):
        """Perform the actual installation."""
        try:
            disk = self.config['disk']
            username = self.config['username']
            fullname = self.config['fullname']
            password = self.config['password']
            hostname = self.config['hostname']

            # Determine partition names
            if 'nvme' in disk or 'mmcblk' in disk:
                part1 = f"{disk}p1"
                part2 = f"{disk}p2"
            else:
                part1 = f"{disk}1"
                part2 = f"{disk}2"

            mount_dir = "/mnt/coraos-install"

            # Step 1: Partition
            self.set_progress(0.05, "Partitioning disk...")
            self.run_cmd(f"umount {disk}* 2>/dev/null || true", check=False)
            self.run_cmd(f"parted -s {disk} mklabel gpt")
            self.run_cmd(f"parted -s {disk} mkpart ESP fat32 1MiB 513MiB")
            self.run_cmd(f"parted -s {disk} set 1 esp on")
            self.run_cmd(f"parted -s {disk} mkpart primary ext4 513MiB 100%")
            self.run_cmd("sleep 1")

            # Step 2: Format
            self.set_progress(0.10, "Formatting partitions...")
            self.run_cmd(f"mkfs.fat -F32 {part1}")
            self.run_cmd(f"mkfs.ext4 -F -q {part2}")

            # Step 3: Mount
            self.set_progress(0.15, "Mounting target...")
            self.run_cmd(f"mkdir -p {mount_dir}")
            self.run_cmd(f"mount {part2} {mount_dir}")
            self.run_cmd(f"mkdir -p {mount_dir}/boot/efi")
            self.run_cmd(f"mount {part1} {mount_dir}/boot/efi")

            # Step 4: Copy filesystem
            self.set_progress(0.20, "Copying system files (this takes a few minutes)...")
            self.run_cmd(
                f"unsquashfs -f -d {mount_dir} /run/live/medium/live/filesystem.squashfs"
            )

            # Step 5: Configure
            self.set_progress(0.60, "Configuring system...")
            self.run_cmd(f'echo "{hostname}" > {mount_dir}/etc/hostname')
            hosts_content = (
                f"127.0.0.1   localhost\\n"
                f"127.0.1.1   {hostname}\\n"
                f"::1         localhost ip6-localhost ip6-loopback"
            )
            self.run_cmd(f'printf "{hosts_content}" > {mount_dir}/etc/hosts')

            # Fstab
            root_uuid = subprocess.run(
                f"blkid -s UUID -o value {part2}",
                shell=True, capture_output=True, text=True
            ).stdout.strip()
            efi_uuid = subprocess.run(
                f"blkid -s UUID -o value {part1}",
                shell=True, capture_output=True, text=True
            ).stdout.strip()
            fstab = (
                f"UUID={root_uuid}  /          ext4  errors=remount-ro  0  1\\n"
                f"UUID={efi_uuid}   /boot/efi  vfat  umask=0077         0  1"
            )
            self.run_cmd(f'printf "{fstab}" > {mount_dir}/etc/fstab')

            # Network
            if self.config['network'] == 'static':
                self.set_progress(0.65, "Configuring network...")
                nm_dir = f"{mount_dir}/etc/NetworkManager/system-connections"
                self.run_cmd(f"mkdir -p {nm_dir}")
                nm_conf = (
                    "[connection]\\nid=static\\ntype=ethernet\\nautoconnect=true\\n\\n"
                    f"[ipv4]\\nmethod=manual\\naddresses={self.config['static_ip']}\\n"
                    f"gateway={self.config['static_gw']}\\ndns={self.config['static_dns']}\\n\\n"
                    "[ipv6]\\nmethod=auto"
                )
                self.run_cmd(f'printf "{nm_conf}" > {nm_dir}/static.nmconnection')
                self.run_cmd(f"chmod 600 {nm_dir}/static.nmconnection")

            # Step 6: User account
            self.set_progress(0.70, "Creating user account...")
            self.run_cmd(f"mount --bind /dev {mount_dir}/dev")
            self.run_cmd(f"mount --bind /proc {mount_dir}/proc")
            self.run_cmd(f"mount --bind /sys {mount_dir}/sys")

            # Write user setup script
            user_script = f"""#!/bin/bash
set -e
userdel -r cora 2>/dev/null || true
useradd -m -s /bin/bash -c '{fullname}' '{username}'
echo '{username}:{password}' | chpasswd
usermod -aG sudo '{username}'
echo '{username} ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers
echo 'root:{password}' | chpasswd
mkdir -p /etc/systemd/system/getty@tty1.service.d
cat > /etc/systemd/system/getty@tty1.service.d/override.conf << 'GEOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin {username} --noclear %I $TERM
GEOF
echo '/usr/local/bin/cora-welcome' >> /home/{username}/.bash_profile
chown {username}:{username} /home/{username}/.bash_profile
"""
            script_path = f"{mount_dir}/tmp/setup_user.sh"
            with open(script_path, 'w') as f:
                f.write(user_script)
            os.chmod(script_path, 0o755)
            self.run_cmd(f"chroot {mount_dir} /bin/bash /tmp/setup_user.sh")
            self.run_cmd(f"rm -f {script_path}")

            # Step 7: Bootloader
            self.set_progress(0.80, "Installing bootloader...")
            grub_script = """#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive
# grub-efi-amd64 is already installed from the live image
# Just install GRUB to the EFI partition and generate config
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=coraos --recheck 2>/dev/null || true
update-grub 2>/dev/null || true
# Remove live-boot packages (not needed on installed system)
apt-get remove -y -qq live-boot live-config live-config-systemd 2>/dev/null || true
apt-get autoremove -y -qq 2>/dev/null || true
apt-get clean 2>/dev/null || true
"""
            script_path = f"{mount_dir}/tmp/setup_grub.sh"
            with open(script_path, 'w') as f:
                f.write(grub_script)
            os.chmod(script_path, 0o755)
            self.run_cmd(f"chroot {mount_dir} /bin/bash /tmp/setup_grub.sh", check=False)
            self.run_cmd(f"rm -f {script_path}")

            # Cleanup
            self.set_progress(0.95, "Finalizing...")
            self.run_cmd(f"umount {mount_dir}/dev 2>/dev/null || true", check=False)
            self.run_cmd(f"umount {mount_dir}/proc 2>/dev/null || true", check=False)
            self.run_cmd(f"umount {mount_dir}/sys 2>/dev/null || true", check=False)
            self.run_cmd(f"umount {mount_dir}/boot/efi", check=False)
            self.run_cmd(f"umount {mount_dir}", check=False)

            # Done
            self.set_progress(1.0, "Installation complete!")
            self.log("\n✓ CoraOS installed successfully!")

            def show_done():
                self.done_box.set_visible(True)
                self.done_box.show_all()
                self.done_info.set_text(
                    f"Dashboard: http://{hostname}\n"
                    f"Username: {username}\n"
                    f"Password: (what you entered)\n\n"
                    "Remove the installation media and reboot."
                )
            GLib.idle_add(show_done)

        except Exception as e:
            self.set_progress(0.0, f"Installation failed: {e}")
            self.log(f"\n✗ ERROR: {e}")


# ============================================================
# Main
# ============================================================

if __name__ == '__main__':
    win = InstallerWindow()
    win.connect('destroy', Gtk.main_quit)
    win.show_all()
    Gtk.main()
