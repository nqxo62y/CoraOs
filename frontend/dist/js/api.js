const API = {
    baseUrl: '/api',
    token: null,

    init() {
        this.token = localStorage.getItem('coraos_token');
    },

    setToken(token) {
        this.token = token;
        localStorage.setItem('coraos_token', token);
    },

    clearToken() {
        this.token = null;
        localStorage.removeItem('coraos_token');
        localStorage.removeItem('coraos_user');
    },

    isAuthenticated() {
        return !!this.token;
    },

    async request(method, path, body = null) {
        const headers = {
            'Content-Type': 'application/json',
        };

        if (this.token) {
            headers['Authorization'] = `Bearer ${this.token}`;
        }

        const options = { method, headers };
        if (body) {
            options.body = JSON.stringify(body);
        }

        const response = await fetch(`${this.baseUrl}${path}`, options);

        if (response.status === 401) {
            this.clearToken();
            window.location.reload();
            throw new Error('Session expired');
        }

        const data = await response.json().catch(() => null);

        if (!response.ok) {
            const error = data?.error || `Request failed with status ${response.status}`;
            throw new Error(error);
        }

        return data;
    },

    async login(username, password) {
        const data = await this.request('POST', '/auth/login', { username, password });
        this.setToken(data.token);
        localStorage.setItem('coraos_user', JSON.stringify(data.user));
        return data;
    },

    async logout() {
        try {
            await this.request('POST', '/auth/logout');
        } finally {
            this.clearToken();
        }
    },

    async getMe() {
        return this.request('GET', '/auth/me');
    },

    async getMetrics() {
        return this.request('GET', '/system/metrics');
    },

    async getProcesses() {
        return this.request('GET', '/system/processes');
    },

    async getServices() {
        return this.request('GET', '/services');
    },

    async getService(name) {
        return this.request('GET', `/services/${encodeURIComponent(name)}`);
    },

    async serviceAction(name, action) {
        return this.request('POST', `/services/${encodeURIComponent(name)}/action`, {
            service: name,
            action: action,
        });
    },

    async getLogs(params = {}) {
        const query = new URLSearchParams();
        if (params.unit) query.set('unit', params.unit);
        if (params.priority) query.set('priority', params.priority);
        if (params.lines) query.set('lines', params.lines);
        if (params.search) query.set('search', params.search);
        return this.request('GET', `/logs?${query.toString()}`);
    },

    async getLogUnits() {
        return this.request('GET', '/logs/units');
    },

    async getAuditLogs(params = {}) {
        const query = new URLSearchParams();
        if (params.page) query.set('page', params.page);
        if (params.per_page) query.set('per_page', params.per_page);
        return this.request('GET', `/audit?${query.toString()}`);
    },

    async checkUpdates() {
        return this.request('GET', '/updates/check');
    },

    async applyUpdates() {
        return this.request('POST', '/updates/apply');
    },

    async getBackups() {
        return this.request('GET', '/backups');
    },

    async createBackup(name, includePaths) {
        return this.request('POST', '/backups', { name, include_paths: includePaths });
    },

    async deleteBackup(id) {
        return this.request('DELETE', `/backups/${id}`);
    },

    async getUsers() {
        return this.request('GET', '/users');
    },

    async createUser(data) {
        return this.request('POST', '/users', data);
    },

    async updateUser(id, data) {
        return this.request('PUT', `/users/${id}`, data);
    },

    async deleteUser(id) {
        return this.request('DELETE', `/users/${id}`);
    },

    async getConfig() {
        return this.request('GET', '/config');
    },

    async updateConfig(key, value, description) {
        return this.request('POST', '/config', { key, value, description });
    },

    async getFtpStatus() {
        return this.request('GET', '/storage/ftp');
    },

    async installFtp() {
        return this.request('POST', '/storage/ftp/install');
    },

    async updateFtpConfig(config) {
        return this.request('POST', '/storage/ftp/config', config);
    },

    async addFtpUser(username, password, directory) {
        return this.request('POST', '/storage/ftp/users', { username, password, directory });
    },

    async deleteFtpUser(name) {
        return this.request('DELETE', `/storage/ftp/users/${encodeURIComponent(name)}`);
    },

    async getRaidArrays() {
        return this.request('GET', '/storage/raid');
    },

    async getBlockDevices() {
        return this.request('GET', '/storage/disks');
    },

    async createRaid(name, level, devices) {
        return this.request('POST', '/storage/raid', { name, level, devices });
    },

    async deleteRaid(name) {
        return this.request('DELETE', `/storage/raid/${encodeURIComponent(name)}`);
    },

    async getMounts() {
        return this.request('GET', '/storage/mounts');
    },

    async mountDevice(device, mount_point, filesystem, options, persistent) {
        return this.request('POST', '/storage/mount', { device, mount_point, filesystem, options, persistent });
    },

    async unmountDevice(mount_point) {
        return this.request('POST', '/storage/unmount', { mount_point });
    },
};
