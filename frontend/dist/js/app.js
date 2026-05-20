/**
 * CoraOS Dashboard Application
 * Main application logic handling navigation, data loading, and UI updates.
 */

const App = {
    currentPage: 'overview',
    ws: null,
    refreshInterval: null,
    user: null,

    /**
     * Initialize the application.
     */
    init() {
        API.init();

        if (API.isAuthenticated()) {
            this.user = JSON.parse(localStorage.getItem('coraos_user') || '{}');
            this.showDashboard();
        } else {
            this.showLogin();
        }

        this.bindEvents();
        this.loadTheme();
    },

    /**
     * Bind all event listeners.
     */
    bindEvents() {
        // Login form
        document.getElementById('login-form').addEventListener('submit', (e) => {
            e.preventDefault();
            this.handleLogin();
        });

        // Logout
        document.getElementById('logout-btn').addEventListener('click', () => this.handleLogout());

        // Navigation
        document.querySelectorAll('.nav-item').forEach(item => {
            item.addEventListener('click', (e) => {
                e.preventDefault();
                this.navigateTo(item.dataset.page);
            });
        });

        // Sidebar toggle (mobile)
        document.getElementById('sidebar-toggle').addEventListener('click', () => {
            document.querySelector('.sidebar').classList.toggle('open');
        });

        // Theme toggle
        document.getElementById('theme-toggle').addEventListener('click', () => this.toggleTheme());
        document.getElementById('theme-select').addEventListener('change', (e) => {
            this.setTheme(e.target.value);
        });

        // Service search
        document.getElementById('service-search').addEventListener('input', (e) => {
            this.filterServices(e.target.value);
        });

        // Log controls
        document.getElementById('refresh-logs-btn').addEventListener('click', () => this.loadLogs());

        // Updates
        document.getElementById('check-updates-btn').addEventListener('click', () => this.checkUpdates());
        document.getElementById('apply-updates-btn').addEventListener('click', () => this.applyUpdates());

        // Backup
        document.getElementById('create-backup-btn').addEventListener('click', () => this.showCreateBackupModal());

        // Users
        document.getElementById('create-user-btn').addEventListener('click', () => this.showCreateUserModal());

        // Config
        document.getElementById('add-config-btn').addEventListener('click', () => this.showAddConfigModal());

        // Modal close
        document.getElementById('modal-close').addEventListener('click', () => this.closeModal());
        document.getElementById('modal-overlay').addEventListener('click', (e) => {
            if (e.target === e.currentTarget) this.closeModal();
        });
    },

    // ===== Authentication =====
    showLogin() {
        document.getElementById('login-page').classList.remove('hidden');
        document.getElementById('dashboard').classList.add('hidden');
    },

    showDashboard() {
        document.getElementById('login-page').classList.add('hidden');
        document.getElementById('dashboard').classList.remove('hidden');
        document.getElementById('current-user').textContent = this.user?.username || 'user';
        document.getElementById('current-role').textContent = this.user?.role || 'viewer';
        this.navigateTo('overview');
        this.connectWebSocket();
    },

    async handleLogin() {
        const username = document.getElementById('username').value.trim();
        const password = document.getElementById('password').value;
        const errorEl = document.getElementById('login-error');
        const btn = document.getElementById('login-btn');

        if (!username || !password) {
            errorEl.textContent = 'Please enter username and password';
            errorEl.classList.remove('hidden');
            return;
        }

        btn.disabled = true;
        btn.querySelector('.btn-text').classList.add('hidden');
        btn.querySelector('.btn-loading').classList.remove('hidden');
        errorEl.classList.add('hidden');

        try {
            const data = await API.login(username, password);
            this.user = data.user;
            this.showDashboard();
        } catch (err) {
            errorEl.textContent = err.message;
            errorEl.classList.remove('hidden');
        } finally {
            btn.disabled = false;
            btn.querySelector('.btn-text').classList.remove('hidden');
            btn.querySelector('.btn-loading').classList.add('hidden');
        }
    },

    async handleLogout() {
        await API.logout();
        this.disconnectWebSocket();
        this.showLogin();
    },

    // ===== Navigation =====
    navigateTo(page) {
        this.currentPage = page;

        // Update nav active state
        document.querySelectorAll('.nav-item').forEach(item => {
            item.classList.toggle('active', item.dataset.page === page);
        });

        // Show/hide page sections
        document.querySelectorAll('.page-section').forEach(section => {
            section.classList.toggle('active', section.id === `page-${page}`);
        });

        // Close mobile sidebar
        document.querySelector('.sidebar').classList.remove('open');

        // Load page data
        this.loadPageData(page);
    },

    loadPageData(page) {
        switch (page) {
            case 'overview': this.loadOverview(); break;
            case 'services': this.loadServices(); break;
            case 'processes': this.loadProcesses(); break;
            case 'logs': this.loadLogs(); break;
            case 'updates': break;
            case 'backups': this.loadBackups(); break;
            case 'users': this.loadUsers(); break;
            case 'config': this.loadConfig(); break;
        }
    },

    // ===== WebSocket =====
    connectWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/api/ws`;

        this.ws = new WebSocket(wsUrl);

        this.ws.onmessage = (event) => {
            try {
                const metrics = JSON.parse(event.data);
                this.updateMetricsDisplay(metrics);
            } catch (e) {
                console.error('WebSocket parse error:', e);
            }
        };

        this.ws.onclose = () => {
            // Reconnect after 5 seconds
            setTimeout(() => {
                if (API.isAuthenticated()) {
                    this.connectWebSocket();
                }
            }, 5000);
        };

        this.ws.onerror = () => {
            this.ws.close();
        };
    },

    disconnectWebSocket() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    },

    // ===== Overview / Metrics =====
    async loadOverview() {
        try {
            const metrics = await API.getMetrics();
            this.updateMetricsDisplay(metrics);
        } catch (err) {
            console.error('Failed to load metrics:', err);
        }
    },

    updateMetricsDisplay(m) {
        // CPU
        const cpuPct = m.cpu_usage_percent.toFixed(1);
        document.getElementById('cpu-usage').textContent = `${cpuPct}%`;
        const cpuBar = document.getElementById('cpu-bar');
        cpuBar.style.width = `${cpuPct}%`;
        cpuBar.className = 'metric-bar-fill' + (cpuPct > 90 ? ' danger' : cpuPct > 70 ? ' warning' : '');
        document.getElementById('cpu-detail').textContent = `${m.cpu_count} cores`;

        // Memory
        const memPct = m.memory_percent.toFixed(1);
        document.getElementById('mem-usage').textContent = `${memPct}%`;
        const memBar = document.getElementById('mem-bar');
        memBar.style.width = `${memPct}%`;
        memBar.className = 'metric-bar-fill' + (memPct > 90 ? ' danger' : memPct > 70 ? ' warning' : '');
        document.getElementById('mem-detail').textContent = `${m.memory_used_mb} / ${m.memory_total_mb} MB`;

        // Disk (use first disk or aggregate)
        if (m.disk_usage && m.disk_usage.length > 0) {
            const rootDisk = m.disk_usage.find(d => d.mount_point === '/') || m.disk_usage[0];
            const diskPct = rootDisk.usage_percent.toFixed(1);
            document.getElementById('disk-usage').textContent = `${diskPct}%`;
            const diskBar = document.getElementById('disk-bar');
            diskBar.style.width = `${diskPct}%`;
            diskBar.className = 'metric-bar-fill' + (diskPct > 90 ? ' danger' : diskPct > 70 ? ' warning' : '');
            document.getElementById('disk-detail').textContent =
                `${rootDisk.used_gb.toFixed(1)} / ${rootDisk.total_gb.toFixed(1)} GB`;
        }

        // Network
        if (m.network && m.network.length > 0) {
            const totalRx = m.network.reduce((sum, n) => sum + n.received_bytes, 0);
            const totalTx = m.network.reduce((sum, n) => sum + n.transmitted_bytes, 0);
            document.getElementById('net-usage').textContent = this.formatBytes(totalRx + totalTx);
            document.getElementById('net-detail').textContent =
                `RX: ${this.formatBytes(totalRx)} / TX: ${this.formatBytes(totalTx)}`;
        }

        // System info
        document.getElementById('sys-hostname').textContent = m.hostname;
        document.getElementById('sys-os').textContent = m.os_name;
        document.getElementById('sys-kernel').textContent = m.kernel_version;
        document.getElementById('sys-uptime').textContent = this.formatUptime(m.uptime_seconds);
        document.getElementById('sys-load').textContent =
            `${m.load_average.one_min.toFixed(2)} / ${m.load_average.five_min.toFixed(2)} / ${m.load_average.fifteen_min.toFixed(2)}`;
        document.getElementById('sys-cpu-model').textContent = m.cpu_model;
    },

    // ===== Services =====
    async loadServices() {
        const loading = document.getElementById('services-loading');
        const tbody = document.getElementById('services-tbody');
        loading.classList.remove('hidden');

        try {
            const services = await API.getServices();
            tbody.innerHTML = '';
            services.forEach(svc => {
                const statusClass = svc.active_state === 'active' ? 'badge-success' :
                    svc.active_state === 'failed' ? 'badge-danger' : 'badge-warning';
                const row = document.createElement('tr');
                row.dataset.name = svc.name.toLowerCase();
                row.innerHTML = `
                    <td><strong>${this.escapeHtml(svc.name)}</strong></td>
                    <td><span class="badge ${statusClass}">${this.escapeHtml(svc.active_state)}</span></td>
                    <td>${this.escapeHtml(svc.sub_state)}</td>
                    <td>${this.escapeHtml(svc.description)}</td>
                    <td>
                        <button class="btn btn-sm btn-success" onclick="App.doServiceAction('${svc.name}', 'start')">Start</button>
                        <button class="btn btn-sm btn-danger" onclick="App.doServiceAction('${svc.name}', 'stop')">Stop</button>
                        <button class="btn btn-sm btn-outline" onclick="App.doServiceAction('${svc.name}', 'restart')">Restart</button>
                    </td>
                `;
                tbody.appendChild(row);
            });
        } catch (err) {
            this.showToast('Failed to load services: ' + err.message, 'error');
        } finally {
            loading.classList.add('hidden');
        }
    },

    async doServiceAction(name, action) {
        try {
            const result = await API.serviceAction(name, action);
            this.showToast(result.message, 'success');
            setTimeout(() => this.loadServices(), 1000);
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    filterServices(query) {
        const rows = document.querySelectorAll('#services-tbody tr');
        const q = query.toLowerCase();
        rows.forEach(row => {
            row.style.display = row.dataset.name.includes(q) ? '' : 'none';
        });
    },

    // ===== Processes =====
    async loadProcesses() {
        try {
            const processes = await API.getProcesses();
            const tbody = document.getElementById('processes-tbody');
            tbody.innerHTML = '';
            processes.slice(0, 50).forEach(proc => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${proc.pid}</td>
                    <td>${this.escapeHtml(proc.name)}</td>
                    <td>${proc.cpu_usage.toFixed(1)}%</td>
                    <td>${proc.memory_mb} MB</td>
                    <td>${this.escapeHtml(proc.status)}</td>
                `;
                tbody.appendChild(row);
            });
        } catch (err) {
            this.showToast('Failed to load processes: ' + err.message, 'error');
        }
    },

    // ===== Logs =====
    async loadLogs() {
        const viewer = document.getElementById('log-viewer');
        viewer.innerHTML = '<div class="loading-state">Loading logs...</div>';

        try {
            const params = {
                unit: document.getElementById('log-unit').value,
                priority: document.getElementById('log-priority').value,
                search: document.getElementById('log-search').value,
                lines: 200,
            };

            const logs = await API.getLogs(params);
            viewer.innerHTML = '';

            if (logs.length === 0) {
                viewer.innerHTML = '<div class="loading-state">No log entries found</div>';
                return;
            }

            logs.forEach(entry => {
                const div = document.createElement('div');
                div.className = `log-entry priority-${this.priorityName(entry.priority)}`;
                div.innerHTML = `
                    <span class="log-time">${this.formatTimestamp(entry.timestamp)}</span>
                    <span class="log-unit">${this.escapeHtml(entry.unit)}</span>
                    <span class="log-msg">${this.escapeHtml(entry.message)}</span>
                `;
                viewer.appendChild(div);
            });

            // Scroll to bottom
            viewer.scrollTop = viewer.scrollHeight;
        } catch (err) {
            viewer.innerHTML = `<div class="loading-state">Error: ${err.message}</div>`;
        }

        // Load units for filter dropdown
        this.loadLogUnits();
    },

    async loadLogUnits() {
        try {
            const units = await API.getLogUnits();
            const select = document.getElementById('log-unit');
            const current = select.value;
            select.innerHTML = '<option value="">All Units</option>';
            units.forEach(unit => {
                const opt = document.createElement('option');
                opt.value = unit;
                opt.textContent = unit;
                if (unit === current) opt.selected = true;
                select.appendChild(opt);
            });
        } catch (err) {
            // Non-critical, ignore
        }
    },

    // ===== Updates =====
    async checkUpdates() {
        const btn = document.getElementById('check-updates-btn');
        const tbody = document.getElementById('updates-tbody');
        const status = document.getElementById('updates-status');
        const applyBtn = document.getElementById('apply-updates-btn');

        btn.disabled = true;
        btn.textContent = 'Checking...';
        status.textContent = '';

        try {
            const data = await API.checkUpdates();
            tbody.innerHTML = '';

            if (data.updates.length === 0) {
                status.textContent = 'System is up to date.';
                status.style.color = 'var(--success)';
                applyBtn.disabled = true;
            } else {
                data.updates.forEach(upd => {
                    const row = document.createElement('tr');
                    row.innerHTML = `
                        <td>${this.escapeHtml(upd.package)}</td>
                        <td>${this.escapeHtml(upd.current_version)}</td>
                        <td>${this.escapeHtml(upd.new_version)}</td>
                        <td>${this.escapeHtml(upd.repository)}</td>
                    `;
                    tbody.appendChild(row);
                });
                status.textContent = `${data.updates.length} update(s) available.`;
                status.style.color = 'var(--warning)';
                applyBtn.disabled = false;
            }
        } catch (err) {
            status.textContent = 'Error: ' + err.message;
            status.style.color = 'var(--danger)';
        } finally {
            btn.disabled = false;
            btn.textContent = 'Check for Updates';
        }
    },

    async applyUpdates() {
        if (!confirm('Apply all available system updates? This may take several minutes.')) return;

        const btn = document.getElementById('apply-updates-btn');
        btn.disabled = true;
        btn.textContent = 'Applying...';

        try {
            await API.applyUpdates();
            this.showToast('Updates applied successfully', 'success');
            document.getElementById('updates-tbody').innerHTML = '';
            document.getElementById('updates-status').textContent = 'System is up to date.';
        } catch (err) {
            this.showToast('Failed to apply updates: ' + err.message, 'error');
        } finally {
            btn.disabled = false;
            btn.textContent = 'Apply All Updates';
        }
    },

    // ===== Backups =====
    async loadBackups() {
        try {
            const data = await API.getBackups();
            const tbody = document.getElementById('backups-tbody');
            tbody.innerHTML = '';

            data.backups.forEach(backup => {
                const statusClass = backup.status === 'completed' ? 'badge-success' :
                    backup.status === 'failed' ? 'badge-danger' : 'badge-warning';
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td>${this.escapeHtml(backup.name)}</td>
                    <td>${this.formatBytes(backup.size_bytes)}</td>
                    <td><span class="badge ${statusClass}">${backup.status}</span></td>
                    <td>${this.formatDate(backup.created_at)}</td>
                    <td>
                        <button class="btn btn-sm btn-danger" onclick="App.deleteBackup('${backup.id}')">Delete</button>
                    </td>
                `;
                tbody.appendChild(row);
            });
        } catch (err) {
            this.showToast('Failed to load backups: ' + err.message, 'error');
        }
    },

    showCreateBackupModal() {
        this.openModal('Create Backup', `
            <div class="form-group">
                <label>Backup Name</label>
                <input type="text" id="backup-name" class="search-input" placeholder="e.g. daily-backup">
            </div>
            <div class="form-group">
                <label>Paths to Include (one per line)</label>
                <textarea id="backup-paths" class="search-input" rows="4" placeholder="/etc&#10;/var/www&#10;/home"
                    style="width:100%;resize:vertical;padding:10px;background:var(--bg-tertiary);border:1px solid var(--border);border-radius:var(--radius);color:var(--text-primary);font-family:var(--font-mono);font-size:13px;"></textarea>
            </div>
        `, [
            { text: 'Cancel', class: 'btn btn-outline', action: () => this.closeModal() },
            { text: 'Create', class: 'btn btn-primary', action: () => this.doCreateBackup() },
        ]);
    },

    async doCreateBackup() {
        const name = document.getElementById('backup-name').value.trim();
        const pathsText = document.getElementById('backup-paths').value.trim();
        const paths = pathsText.split('\n').map(p => p.trim()).filter(p => p);

        if (!name) { this.showToast('Backup name is required', 'warning'); return; }
        if (paths.length === 0) { this.showToast('At least one path is required', 'warning'); return; }

        try {
            await API.createBackup(name, paths);
            this.showToast('Backup created successfully', 'success');
            this.closeModal();
            this.loadBackups();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    async deleteBackup(id) {
        if (!confirm('Delete this backup permanently?')) return;
        try {
            await API.deleteBackup(id);
            this.showToast('Backup deleted', 'success');
            this.loadBackups();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    // ===== Users =====
    async loadUsers() {
        if (this.user?.role !== 'admin') {
            document.getElementById('page-users').innerHTML =
                '<h1 class="page-title">User Management</h1><p>Admin access required.</p>';
            return;
        }

        try {
            const users = await API.getUsers();
            const tbody = document.getElementById('users-tbody');
            tbody.innerHTML = '';

            users.forEach(user => {
                const statusBadge = user.is_active ?
                    '<span class="badge badge-success">Active</span>' :
                    '<span class="badge badge-danger">Disabled</span>';
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td><strong>${this.escapeHtml(user.username)}</strong></td>
                    <td><span class="badge badge-info">${user.role}</span></td>
                    <td>${statusBadge}</td>
                    <td>${user.last_login ? this.formatDate(user.last_login) : 'Never'}</td>
                    <td>
                        <button class="btn btn-sm btn-outline" onclick="App.showEditUserModal('${user.id}', '${user.username}', '${user.role}')">Edit</button>
                        <button class="btn btn-sm btn-danger" onclick="App.deleteUser('${user.id}')">Delete</button>
                    </td>
                `;
                tbody.appendChild(row);
            });
        } catch (err) {
            this.showToast('Failed to load users: ' + err.message, 'error');
        }
    },

    showCreateUserModal() {
        this.openModal('Create User', `
            <div class="form-group">
                <label>Username</label>
                <input type="text" id="new-username" class="search-input" placeholder="Username (3-32 chars)">
            </div>
            <div class="form-group">
                <label>Password</label>
                <input type="password" id="new-password" class="search-input" placeholder="Password (min 8 chars)">
            </div>
            <div class="form-group">
                <label>Role</label>
                <select id="new-role" class="select-input">
                    <option value="viewer">Viewer</option>
                    <option value="operator">Operator</option>
                    <option value="admin">Admin</option>
                </select>
            </div>
            <div class="form-group">
                <label>Display Name</label>
                <input type="text" id="new-display-name" class="search-input" placeholder="Optional">
            </div>
            <div class="form-group">
                <label>Email</label>
                <input type="email" id="new-email" class="search-input" placeholder="Optional">
            </div>
        `, [
            { text: 'Cancel', class: 'btn btn-outline', action: () => this.closeModal() },
            { text: 'Create', class: 'btn btn-primary', action: () => this.doCreateUser() },
        ]);
    },

    async doCreateUser() {
        const data = {
            username: document.getElementById('new-username').value.trim(),
            password: document.getElementById('new-password').value,
            role: document.getElementById('new-role').value,
            display_name: document.getElementById('new-display-name').value.trim() || undefined,
            email: document.getElementById('new-email').value.trim() || undefined,
        };

        if (!data.username || !data.password) {
            this.showToast('Username and password are required', 'warning');
            return;
        }

        try {
            await API.createUser(data);
            this.showToast('User created successfully', 'success');
            this.closeModal();
            this.loadUsers();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    showEditUserModal(id, username, role) {
        this.openModal(`Edit User: ${username}`, `
            <div class="form-group">
                <label>Role</label>
                <select id="edit-role" class="select-input">
                    <option value="viewer" ${role === 'viewer' ? 'selected' : ''}>Viewer</option>
                    <option value="operator" ${role === 'operator' ? 'selected' : ''}>Operator</option>
                    <option value="admin" ${role === 'admin' ? 'selected' : ''}>Admin</option>
                </select>
            </div>
            <div class="form-group">
                <label>New Password (leave empty to keep current)</label>
                <input type="password" id="edit-password" class="search-input" placeholder="New password">
            </div>
        `, [
            { text: 'Cancel', class: 'btn btn-outline', action: () => this.closeModal() },
            { text: 'Save', class: 'btn btn-primary', action: () => this.doUpdateUser(id) },
        ]);
    },

    async doUpdateUser(id) {
        const data = {};
        const role = document.getElementById('edit-role').value;
        const password = document.getElementById('edit-password').value;

        if (role) data.role = role;
        if (password) data.password = password;

        try {
            await API.updateUser(id, data);
            this.showToast('User updated successfully', 'success');
            this.closeModal();
            this.loadUsers();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    async deleteUser(id) {
        if (!confirm('Delete this user permanently?')) return;
        try {
            await API.deleteUser(id);
            this.showToast('User deleted', 'success');
            this.loadUsers();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    // ===== Configuration =====
    async loadConfig() {
        try {
            const configs = await API.getConfig();
            const tbody = document.getElementById('config-tbody');
            tbody.innerHTML = '';

            configs.forEach(cfg => {
                const row = document.createElement('tr');
                row.innerHTML = `
                    <td><strong>${this.escapeHtml(cfg.key)}</strong></td>
                    <td><code>${this.escapeHtml(cfg.value)}</code></td>
                    <td>${this.escapeHtml(cfg.description || '-')}</td>
                    <td>${this.formatDate(cfg.updated_at)}</td>
                    <td>
                        <button class="btn btn-sm btn-outline" onclick="App.showEditConfigModal('${this.escapeHtml(cfg.key)}', '${this.escapeHtml(cfg.value)}')">Edit</button>
                    </td>
                `;
                tbody.appendChild(row);
            });
        } catch (err) {
            this.showToast('Failed to load configuration: ' + err.message, 'error');
        }
    },

    showAddConfigModal() {
        this.openModal('Add Configuration', `
            <div class="form-group">
                <label>Key</label>
                <input type="text" id="config-key" class="search-input" placeholder="e.g. server.port">
            </div>
            <div class="form-group">
                <label>Value</label>
                <input type="text" id="config-value" class="search-input" placeholder="Value">
            </div>
            <div class="form-group">
                <label>Description</label>
                <input type="text" id="config-desc" class="search-input" placeholder="Optional description">
            </div>
        `, [
            { text: 'Cancel', class: 'btn btn-outline', action: () => this.closeModal() },
            { text: 'Save', class: 'btn btn-primary', action: () => this.doSaveConfig() },
        ]);
    },

    showEditConfigModal(key, value) {
        this.openModal(`Edit: ${key}`, `
            <div class="form-group">
                <label>Key</label>
                <input type="text" id="config-key" class="search-input" value="${key}" readonly>
            </div>
            <div class="form-group">
                <label>Value</label>
                <input type="text" id="config-value" class="search-input" value="${value}">
            </div>
            <div class="form-group">
                <label>Description</label>
                <input type="text" id="config-desc" class="search-input" placeholder="Optional">
            </div>
        `, [
            { text: 'Cancel', class: 'btn btn-outline', action: () => this.closeModal() },
            { text: 'Save', class: 'btn btn-primary', action: () => this.doSaveConfig() },
        ]);
    },

    async doSaveConfig() {
        const key = document.getElementById('config-key').value.trim();
        const value = document.getElementById('config-value').value.trim();
        const desc = document.getElementById('config-desc').value.trim() || undefined;

        if (!key || !value) {
            this.showToast('Key and value are required', 'warning');
            return;
        }

        try {
            await API.updateConfig(key, value, desc);
            this.showToast('Configuration saved', 'success');
            this.closeModal();
            this.loadConfig();
        } catch (err) {
            this.showToast(err.message, 'error');
        }
    },

    // ===== Theme =====
    loadTheme() {
        const theme = localStorage.getItem('coraos_theme') || 'dark';
        this.setTheme(theme);
    },

    setTheme(theme) {
        document.body.className = `theme-${theme}`;
        localStorage.setItem('coraos_theme', theme);
        document.getElementById('theme-select').value = theme;
    },

    toggleTheme() {
        const current = localStorage.getItem('coraos_theme') || 'dark';
        this.setTheme(current === 'dark' ? 'light' : 'dark');
    },

    // ===== Modal =====
    openModal(title, bodyHtml, buttons = []) {
        document.getElementById('modal-title').textContent = title;
        document.getElementById('modal-body').innerHTML = bodyHtml;

        const footer = document.getElementById('modal-footer');
        footer.innerHTML = '';
        buttons.forEach(btn => {
            const el = document.createElement('button');
            el.className = btn.class;
            el.textContent = btn.text;
            el.addEventListener('click', btn.action);
            footer.appendChild(el);
        });

        document.getElementById('modal-overlay').classList.remove('hidden');
    },

    closeModal() {
        document.getElementById('modal-overlay').classList.add('hidden');
    },

    // ===== Toast Notifications =====
    showToast(message, type = 'info') {
        const container = document.getElementById('toast-container');
        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        toast.textContent = message;
        container.appendChild(toast);

        setTimeout(() => {
            toast.style.opacity = '0';
            toast.style.transform = 'translateX(100%)';
            setTimeout(() => toast.remove(), 300);
        }, 4000);
    },

    // ===== Utility Functions =====
    formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
    },

    formatUptime(seconds) {
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        if (days > 0) return `${days}d ${hours}h ${minutes}m`;
        if (hours > 0) return `${hours}h ${minutes}m`;
        return `${minutes}m`;
    },

    formatDate(dateStr) {
        if (!dateStr) return '-';
        try {
            const d = new Date(dateStr);
            return d.toLocaleString();
        } catch {
            return dateStr;
        }
    },

    formatTimestamp(ts) {
        if (!ts) return '';
        // journalctl timestamps are in microseconds
        try {
            const ms = parseInt(ts) / 1000;
            const d = new Date(ms);
            return d.toLocaleTimeString();
        } catch {
            return ts;
        }
    },

    priorityName(p) {
        const names = { '0': 'emerg', '1': 'alert', '2': 'crit', '3': 'err', '4': 'warning', '5': 'notice', '6': 'info', '7': 'debug' };
        return names[p] || 'info';
    },

    escapeHtml(str) {
        if (!str) return '';
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    },
};

// Initialize the application when DOM is ready
document.addEventListener('DOMContentLoaded', () => App.init());
