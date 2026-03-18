// Drive Explorer page component
function drivesPage() {
  return {
    drives: [],
    activeDrive: 'main',
    currentPath: '/',
    breadcrumbs: [{name: 'Root', path: '/'}],
    entries: [],
    loading: true,
    loadError: null,
    searchQuery: '',
    searchResults: [],
    searching: false,
    selectedFile: null,
    viewMode: 'list',
    showUpload: false,
    uploadProgress: null,
    dragOver: false,
    pipelineStatus: null,
    rules: [],
    repos: [],
    editingTags: false,
    tagInput: '',

    async init() {
      await this.loadDrives();
      await this.loadEntries();
      await this.loadRules();
      await this.loadRepos();
    },

    async loadDrives() {
      try {
        const r = await fetch('/api/drives', {headers: window.apiHeaders ? window.apiHeaders() : {}});
        this.drives = await r.json();
        if (this.drives.length && !this.drives.find(d => d.name === this.activeDrive)) {
          this.activeDrive = this.drives[0].name;
        }
      } catch(e) { console.error('loadDrives', e); }
    },

    async loadEntries() {
      this.loading = true;
      this.loadError = null;
      this.selectedFile = null;
      try {
        const r = await fetch(`/api/drives/${this.activeDrive}/ls?path=${encodeURIComponent(this.currentPath)}`, {headers: window.apiHeaders ? window.apiHeaders() : {}});
        if (!r.ok) throw new Error(await r.text());
        this.entries = await r.json();
      } catch(e) {
        this.loadError = e.message;
        this.entries = [];
      }
      this.loading = false;
    },

    async loadRules() {
      try {
        const r = await fetch(`/api/drives/${this.activeDrive}/rules`, {headers: window.apiHeaders ? window.apiHeaders() : {}});
        this.rules = await r.json();
      } catch(e) { this.rules = []; }
    },

    async loadRepos() {
      try {
        const r = await fetch(`/api/drives/${this.activeDrive}/repos`, {headers: window.apiHeaders ? window.apiHeaders() : {}});
        this.repos = await r.json();
      } catch(e) { this.repos = []; }
    },

    async loadPipelineStatus() {
      try {
        const r = await fetch(`/api/drives/${this.activeDrive}/index/status`, {headers: window.apiHeaders ? window.apiHeaders() : {}});
        this.pipelineStatus = await r.json();
      } catch(e) { this.pipelineStatus = null; }
    },

    navigateTo(path) {
      this.currentPath = path;
      const parts = path.split('/').filter(Boolean);
      this.breadcrumbs = [{name: this.activeDrive, path: '/'}];
      let acc = '';
      for (const p of parts) {
        acc += '/' + p;
        this.breadcrumbs.push({name: p, path: acc});
      }
      this.loadEntries();
    },

    navigateUp() {
      if (this.currentPath === '/') return;
      const parts = this.currentPath.split('/').filter(Boolean);
      parts.pop();
      this.navigateTo('/' + parts.join('/'));
    },

    clickEntry(entry) {
      if (entry.is_dir) {
        this.navigateTo(entry.path);
      } else {
        this.selectedFile = entry;
        this.editingTags = false;
      }
    },

    async doSearch() {
      if (!this.searchQuery.trim()) { this.searchResults = []; return; }
      this.searching = true;
      try {
        const r = await fetch(`/api/drives/${this.activeDrive}/search?q=${encodeURIComponent(this.searchQuery)}&type=metadata`, {headers: window.apiHeaders ? window.apiHeaders() : {}});
        const data = await r.json();
        this.searchResults = Array.isArray(data) ? data : [];
      } catch(e) { this.searchResults = []; }
      this.searching = false;
    },

    fileIcon(entry) {
      if (entry.is_dir) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>';
      const ext = entry.name.split('.').pop().toLowerCase();
      if (['pdf'].includes(ext)) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/></svg>';
      if (['jpg','jpeg','png','gif','svg','webp'].includes(ext)) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#10b981" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/></svg>';
      if (['mp3','wav','ogg'].includes(ext)) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8b5cf6" stroke-width="2"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>';
      if (['mp4','webm','mov'].includes(ext)) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#ec4899" stroke-width="2"><rect x="2" y="2" width="20" height="20" rx="2.18"/><path d="m10 8 6 4-6 4V8z"/></svg>';
      if (['js','ts','rs','py','go','java','c','cpp','json','toml','yaml','yml','html','css'].includes(ext)) return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#06b6d4" stroke-width="2"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>';
      return '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/></svg>';
    },

    fileIconLarge(entry) {
      return this.fileIcon(entry).replace(/width="16"/g, 'width="40"').replace(/height="16"/g, 'height="40"');
    },

    fileIconMedium(entry) {
      return this.fileIcon(entry).replace(/width="16"/g, 'width="28"').replace(/height="16"/g, 'height="28"');
    },

    formatSize(bytes) {
      if (!bytes) return '\u2014';
      if (bytes < 1024) return bytes + ' B';
      if (bytes < 1024*1024) return (bytes/1024).toFixed(1) + ' KB';
      if (bytes < 1024*1024*1024) return (bytes/(1024*1024)).toFixed(1) + ' MB';
      return (bytes/(1024*1024*1024)).toFixed(2) + ' GB';
    },

    formatDate(iso) {
      if (!iso) return '\u2014';
      try { return new Date(iso).toLocaleDateString(undefined, {month:'short', day:'numeric', year:'numeric', hour:'2-digit', minute:'2-digit'}); }
      catch(e) { return iso; }
    },

    async handleDrop(event) {
      this.dragOver = false;
      const files = event.dataTransfer?.files;
      if (!files || !files.length) return;
      for (const file of files) {
        await this.uploadFile(file);
      }
    },

    async uploadFile(file) {
      const path = this.currentPath.replace(/\/+$/, '') + '/' + file.name;
      this.uploadProgress = {name: file.name, pct: 0};
      try {
        const buf = await file.arrayBuffer();
        const r = await fetch(`/api/drives/${this.activeDrive}/file?path=${encodeURIComponent(path)}`, {
          method: 'PUT',
          headers: {'Content-Type': 'application/octet-stream', ...(window.apiHeaders ? window.apiHeaders() : {})},
          body: buf,
        });
        if (!r.ok) throw new Error(await r.text());
        this.uploadProgress = null;
        this.loadEntries();
      } catch(e) {
        this.uploadProgress = null;
        alert('Upload failed: ' + e.message);
      }
    },

    async deleteFile(entry) {
      if (!confirm('Delete ' + entry.name + '?')) return;
      try {
        await fetch(`/api/drives/${this.activeDrive}/file?path=${encodeURIComponent(entry.path)}`, {method: 'DELETE', headers: window.apiHeaders ? window.apiHeaders() : {}});
        this.selectedFile = null;
        this.loadEntries();
      } catch(e) { alert('Delete failed: ' + e.message); }
    },

    async saveTags() {
      if (!this.selectedFile) return;
      const tags = this.tagInput.split(',').map(t => t.trim()).filter(Boolean);
      try {
        await fetch(`/api/drives/${this.activeDrive}/tags`, {
          method: 'PUT',
          headers: {'Content-Type': 'application/json', ...(window.apiHeaders ? window.apiHeaders() : {})},
          body: JSON.stringify({path: this.selectedFile.path, tags}),
        });
        this.editingTags = false;
      } catch(e) { alert('Failed to save tags: ' + e.message); }
    },

    downloadUrl(entry) {
      return `/api/drives/${this.activeDrive}/file?path=${encodeURIComponent(entry.path)}`;
    },
  };
}
