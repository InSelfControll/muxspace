// Muxspace GUI Frontend
// Access Tauri APIs through window.__TAURI__

// Get Tauri APIs
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

// State
let currentView = 'dashboard';
let selectedWorkspace = null;
let projects = [];
let workspaces = [];
let activeProjects = [];
let detectedTools = [];

// DOM Elements
let els = {};

// Initialize
async function init() {
    console.log('Initializing Muxspace GUI...');
    
    // Cache DOM elements
    cacheElements();
    
    // Check if Tauri is loaded
    if (!window.__TAURI__) {
        console.error('Tauri is not loaded! Running in browser mode.');
        showNotification('Warning: Running in browser mode. Some features may not work.', 'warning');
    } else {
        console.log('Tauri APIs loaded successfully');
    }
    
    // Setup event listeners first
    setupEventListeners();
    
    // Load initial data
    try {
        await Promise.all([
            scanForWorkspaces(),
            loadProjects(),
            loadWorkspaces(),
            loadActiveProjects(),
            loadStats(),
        ]);
        console.log('Muxspace GUI initialized');
    } catch (e) {
        console.error('Failed to initialize:', e);
        showNotification('Some features failed to load. Check console.', 'error');
    }
}

function cacheElements() {
    els = {
        projectList: document.getElementById('project-list'),
        workspaceList: document.getElementById('workspace-list'),
        pageTitle: document.getElementById('page-title'),
        content: document.getElementById('content'),
        dashboardView: document.getElementById('dashboard-view'),
        workspaceDetail: document.getElementById('workspace-detail'),
        toolsView: document.getElementById('tools-view'),
        modalCreate: document.getElementById('modal-create'),
        modalLoad: document.getElementById('modal-load'),
        statProjects: document.getElementById('stat-projects'),
        statWorkspaces: document.getElementById('stat-workspaces'),
        statTools: document.getElementById('stat-tools'),
        activeProjectsList: document.getElementById('active-projects-list'),
        toolsGrid: document.getElementById('tools-grid'),
    };
}

// Scan for workspace configs
async function scanForWorkspaces() {
    try {
        console.log('Scanning for workspaces...');
        const configs = await invoke('scan_for_workspaces');
        console.log(`Found ${configs.length} workspace configs:`, configs.map(c => c.name));
        window.workspaceConfigs = configs;
        return configs;
    } catch (e) {
        console.error('Failed to scan workspaces:', e);
        return [];
    }
}

// Data Loading
async function loadProjects() {
    try {
        projects = await invoke('get_projects');
        console.log('Loaded projects:', projects);
        renderProjects();
    } catch (e) {
        console.error('Failed to load projects:', e);
        if (els.projectList) {
            els.projectList.innerHTML = '<li class="error">Failed to load projects</li>';
        }
    }
}

async function loadWorkspaces() {
    try {
        workspaces = await invoke('get_workspaces');
        console.log('Loaded workspaces:', workspaces);
        renderWorkspaces();
    } catch (e) {
        console.error('Failed to load workspaces:', e);
        if (els.workspaceList) {
            els.workspaceList.innerHTML = '<li class="error">Failed to load workspaces</li>';
        }
    }
}

async function loadActiveProjects() {
    try {
        activeProjects = await invoke('get_active_projects');
        console.log('Active projects:', activeProjects);
        renderActiveProjects();
    } catch (e) {
        console.error('Failed to load active projects:', e);
        activeProjects = [];
        renderActiveProjects();
    }
}

async function loadStats() {
    try {
        detectedTools = await invoke('get_detected_tools');
        console.log('Detected tools:', detectedTools.length);
        
        if (els.statProjects) els.statProjects.textContent = projects.length;
        if (els.statWorkspaces) els.statWorkspaces.textContent = workspaces.length;
        if (els.statTools) els.statTools.textContent = detectedTools.length;
    } catch (e) {
        console.error('Failed to load stats:', e);
        if (els.statTools) els.statTools.textContent = '0';
    }
}

// Rendering
function renderProjects() {
    if (!els.projectList) return;
    
    if (projects.length === 0) {
        els.projectList.innerHTML = '<li class="empty">No projects found. Create a workspace first.</li>';
        return;
    }
    
    els.projectList.innerHTML = projects.map(p => `
        <li onclick="window.selectProject('${escapeHtml(p.name)}')" 
            class="${activeProjects.includes(p.name) ? 'active-project' : ''}">
            <span>${escapeHtml(p.name)}</span>
            <span class="count">${p.workspaces.length}</span>
        </li>
    `).join('');
}

function renderWorkspaces() {
    if (!els.workspaceList) return;
    
    if (workspaces.length === 0) {
        els.workspaceList.innerHTML = '<li class="empty">No workspaces found. Click "+ New Workspace" to create one.</li>';
        return;
    }
    
    els.workspaceList.innerHTML = workspaces.map(w => `
        <li onclick="window.showWorkspaceDetail('${escapeHtml(w.name)}')" 
            class="${selectedWorkspace === w.name ? 'active' : ''}">
            <span>${escapeHtml(w.name)}</span>
            ${w.project ? `<span class="project-tag">${escapeHtml(w.project)}</span>` : ''}
        </li>
    `).join('');
}

function renderActiveProjects() {
    const list = els.activeProjectsList;
    if (!list) return;
    
    if (activeProjects.length === 0) {
        list.innerHTML = '<p class="empty">No active projects. Run "muxspace restore" in terminal or click "Activate" on a workspace to add it here.</p>';
        return;
    }
    
    list.innerHTML = activeProjects.map((p, i) => `
        <div class="activity-item">
            <span class="activity-rank">#${i + 1}</span>
            <span class="activity-name">${escapeHtml(p)}</span>
            <button class="btn btn-sm" onclick="window.activateProject('${escapeHtml(p)}')">Activate</button>
        </div>
    `).join('');
}

// View Navigation
function showDashboard() {
    console.log('Showing dashboard');
    currentView = 'dashboard';
    if (els.pageTitle) els.pageTitle.textContent = 'Dashboard';
    if (els.dashboardView) els.dashboardView.classList.remove('hidden');
    if (els.workspaceDetail) els.workspaceDetail.classList.add('hidden');
    if (els.toolsView) els.toolsView.classList.add('hidden');
    
    // Reload data
    Promise.all([
        loadProjects(),
        loadWorkspaces(),
        loadActiveProjects(),
        loadStats(),
    ]);
}

async function showTools() {
    console.log('Showing tools');
    currentView = 'tools';
    if (els.pageTitle) els.pageTitle.textContent = 'Detected Tools';
    
    try {
        detectedTools = await invoke('get_detected_tools');
        console.log('Tools loaded:', detectedTools);
        
        const grid = els.toolsGrid;
        if (!grid) {
            console.error('tools-grid element not found');
            return;
        }
        
        if (detectedTools.length === 0) {
            grid.innerHTML = '<p class="empty">No tools detected. Make sure you have development tools installed and on your PATH.</p>';
        } else {
            // Group by category
            const byCategory = detectedTools.reduce((acc, t) => {
                acc[t.category] = acc[t.category] || [];
                acc[t.category].push(t);
                return acc;
            }, {});
            
            grid.innerHTML = Object.entries(byCategory).map(([cat, catTools]) => `
                <div class="tool-category">
                    <h4>${formatCategory(cat)}</h4>
                    <div class="tool-category-grid">
                        ${catTools.map(t => `
                            <div class="tool-card">
                                <div class="tool-icon">${getToolIcon(t.category)}</div>
                                <div class="tool-info">
                                    <h4>${escapeHtml(t.name)}</h4>
                                    <p>${escapeHtml(t.path)}</p>
                                </div>
                            </div>
                        `).join('')}
                    </div>
                </div>
            `).join('');
        }
        
        if (els.dashboardView) els.dashboardView.classList.add('hidden');
        if (els.workspaceDetail) els.workspaceDetail.classList.add('hidden');
        if (els.toolsView) els.toolsView.classList.remove('hidden');
    } catch (e) {
        console.error('Failed to load tools:', e);
        if (els.toolsGrid) {
            els.toolsGrid.innerHTML = `<p class="error">Failed to load tools: ${escapeHtml(e.toString())}</p>`;
        }
        showNotification('Failed to load tools: ' + e, 'error');
    }
}

async function showWorkspaceDetail(name) {
    console.log('Showing workspace detail:', name);
    selectedWorkspace = name;
    currentView = 'workspace';
    if (els.pageTitle) els.pageTitle.textContent = 'Workspace Details';
    
    try {
        const config = await invoke('get_workspace_config', { name });
        console.log('Workspace config:', config);
        
        if (config) {
            const detailName = document.getElementById('detail-name');
            const detailProject = document.getElementById('detail-project');
            
            if (detailName) detailName.textContent = config.name;
            if (detailProject) detailProject.textContent = config.project || 'no project';
            
            // Render panes
            const panesEl = document.getElementById('detail-panes');
            if (panesEl) {
                if (config.panes && config.panes.length > 0) {
                    panesEl.innerHTML = config.panes.map((p, i) => `
                        <div class="pane-item">
                            <span class="pane-icon">📟</span>
                            <div class="pane-info">
                                <div class="pane-title">Pane ${i + 1}</div>
                                <div class="pane-path">${escapeHtml(p.cwd)}</div>
                                ${p.command ? `<div class="pane-cmd">$ ${escapeHtml(p.command)}</div>` : ''}
                            </div>
                        </div>
                    `).join('');
                } else {
                    panesEl.innerHTML = '<p class="empty">No panes configured</p>';
                }
            }
            
            // Render tools
            const toolsEl = document.getElementById('detail-tools');
            if (toolsEl) {
                if (config.tools && config.tools.length > 0) {
                    toolsEl.innerHTML = config.tools.map(t => `
                        <div class="tool-item">
                            <span class="tool-icon">🛠️</span>
                            <div class="tool-info">
                                <div class="tool-name">${escapeHtml(t.app)}</div>
                                <div class="tool-kind">${t.kind}</div>
                            </div>
                        </div>
                    `).join('');
                } else {
                    toolsEl.innerHTML = '<p class="empty">No tools configured</p>';
                }
            }
            
            // Render browser config
            const browserEl = document.getElementById('detail-browser');
            if (browserEl) {
                if (config.browser && config.browser.urls && config.browser.urls.length > 0) {
                    browserEl.innerHTML = `
                        <div class="browser-config">
                            <div class="browser-kind">${config.browser.kind}</div>
                            <ul class="browser-urls">
                                ${config.browser.urls.map(url => `<li>${escapeHtml(url)}</li>`).join('')}
                            </ul>
                        </div>
                    `;
                } else {
                    browserEl.innerHTML = '<p class="empty">No browser configured</p>';
                }
            }
            
            // Setup buttons
            const btnLaunch = document.getElementById('btn-launch');
            if (btnLaunch) {
                btnLaunch.onclick = () => launchWorkspace(name);
            }
            
            const btnActivate = document.getElementById('btn-activate');
            if (btnActivate) {
                btnActivate.onclick = () => activateProject(config.project || name);
            }
        }
        
        if (els.dashboardView) els.dashboardView.classList.add('hidden');
        if (els.workspaceDetail) els.workspaceDetail.classList.remove('hidden');
        if (els.toolsView) els.toolsView.classList.add('hidden');
        renderWorkspaces();
    } catch (e) {
        console.error('Failed to load workspace config:', e);
        showNotification('Failed to load workspace details: ' + e, 'error');
    }
}

// Actions
function selectProject(name) {
    console.log('Selected project:', name);
    showNotification(`Project "${name}" selected`, 'info');
}

async function launchWorkspace(name) {
    try {
        showNotification(`Launching workspace "${name}"...`, 'info');
        const pids = await invoke('start_workspace', { name });
        console.log('Launched workspace with PIDs:', pids);
        
        const config = await invoke('get_workspace_config', { name });
        if (config && config.project) {
            await invoke('add_active_project', { name: config.project });
            await loadActiveProjects();
        }
        
        showNotification(`Workspace "${name}" launched with ${pids.length} external process(es)!`, 'success');
    } catch (e) {
        console.error('Failed to launch workspace:', e);
        showNotification('Failed to launch: ' + e, 'error');
    }
}

async function activateProject(projectName) {
    try {
        await invoke('add_active_project', { name: projectName });
        await loadActiveProjects();
        await loadProjects();
        showNotification(`Project "${projectName}" activated!`, 'success');
    } catch (e) {
        console.error('Failed to activate project:', e);
        showNotification('Failed to activate: ' + e, 'error');
    }
}

async function exportWorkspaces() {
    try {
        showNotification('Exporting workspaces...', 'info');
        const path = await invoke('export_workspaces');
        showNotification(`Workspaces exported to: ${path}`, 'success');
    } catch (e) {
        console.error('Failed to export:', e);
        showNotification('Failed to export: ' + e, 'error');
    }
}

async function importWorkspaces() {
    try {
        showNotification('Importing workspaces...', 'info');
        const count = await invoke('import_workspaces', { path: null });
        showNotification(`Imported ${count} workspace(s)`, 'success');
        
        await scanForWorkspaces();
        await loadWorkspaces();
        await loadStats();
    } catch (e) {
        console.error('Failed to import:', e);
        showNotification('Failed to import: ' + e, 'error');
    }
}

// Load workspace from file using Tauri dialog
async function loadWorkspaceFromFile() {
    try {
        showNotification('Opening file picker...', 'info');
        
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'YAML Config',
                extensions: ['yaml', 'yml']
            }]
        });
        
        if (!selected) {
            console.log('No file selected');
            return;
        }
        
        const selectedPath = Array.isArray(selected) ? selected[0] : selected;
        console.log('Selected file:', selectedPath);
        
        showNotification('Loading workspace...', 'info');
        const config = await invoke('load_workspace_from_path', { path: selectedPath });
        
        // Save it to the workspaces directory
        const savedPath = await invoke('save_workspace_config', { cfg: config });
        console.log('Saved to:', savedPath);
        
        showNotification(`Workspace "${config.name}" loaded successfully!`, 'success');
        
        // Reload data
        await scanForWorkspaces();
        await loadWorkspaces();
        await loadStats();
        
        // Show the new workspace
        showWorkspaceDetail(config.name);
    } catch (e) {
        console.error('Failed to load workspace:', e);
        showNotification('Failed to load workspace: ' + e, 'error');
    }
}

// Show load project modal
function showLoadProjectModal() {
    if (els.modalLoad) {
        els.modalLoad.classList.remove('hidden');
    }
}

function hideLoadProjectModal() {
    if (els.modalLoad) {
        els.modalLoad.classList.add('hidden');
    }
}

// Load project from existing data
async function loadProjectFromData(e) {
    e.preventDefault();
    
    const name = document.getElementById('load-project-name').value.trim();
    const cwd = document.getElementById('load-project-cwd').value.trim();
    const existingData = document.getElementById('load-project-data').value.trim();
    
    if (!name || !cwd) {
        showNotification('Project name and directory are required', 'error');
        return;
    }
    
    try {
        let config;
        
        if (existingData) {
            // Parse existing YAML
            try {
                config = await invoke('load_workspace_from_path', { path: existingData });
                // Override name if provided
                if (name) config.name = name;
            } catch (e) {
                showNotification('Failed to parse existing config: ' + e, 'error');
                return;
            }
        } else {
            // Create new basic config
            config = {
                name,
                project: name,
                panes: [{ cwd, command: null }],
                tools: [],
                browser: null
            };
        }
        
        const savedPath = await invoke('save_workspace_config', { cfg: config });
        console.log('Project saved to:', savedPath);
        
        hideLoadProjectModal();
        
        await scanForWorkspaces();
        await loadWorkspaces();
        await loadStats();
        
        showNotification(`Project "${name}" added successfully!`, 'success');
        showWorkspaceDetail(config.name);
    } catch (e) {
        console.error('Failed to load project:', e);
        showNotification('Failed to load project: ' + e, 'error');
    }
}

// Notification system
function showNotification(message, type = 'info') {
    const container = document.getElementById('notifications');
    if (!container) return;
    
    const notif = document.createElement('div');
    notif.className = `notification notification-${type}`;
    notif.textContent = message;
    container.appendChild(notif);
    
    setTimeout(() => notif.classList.add('show'), 10);
    
    setTimeout(() => {
        notif.classList.remove('show');
        setTimeout(() => notif.remove(), 300);
    }, 4000);
}

// Modal functions
function showCreateModal() {
    if (els.modalCreate) {
        els.modalCreate.classList.remove('hidden');
    }
}

function hideCreateModal() {
    if (els.modalCreate) {
        els.modalCreate.classList.add('hidden');
        const form = document.getElementById('form-create');
        if (form) form.reset();
    }
}

async function createWorkspace(e) {
    e.preventDefault();
    
    const name = document.getElementById('input-name').value.trim();
    const project = document.getElementById('input-project').value.trim() || null;
    const cwd = document.getElementById('input-cwd').value.trim();
    const command = document.getElementById('input-command').value.trim() || null;
    
    if (!name || !cwd) {
        showNotification('Name and Working Directory are required', 'error');
        return;
    }
    
    const config = {
        name,
        project,
        panes: [{ cwd, command }],
        tools: [],
        browser: null
    };
    
    try {
        showNotification('Creating workspace...', 'info');
        const savedPath = await invoke('save_workspace_config', { cfg: config });
        console.log('Workspace saved to:', savedPath);
        
        hideCreateModal();
        
        await scanForWorkspaces();
        await loadWorkspaces();
        await loadStats();
        
        showNotification(`Workspace "${name}" created!`, 'success');
        showWorkspaceDetail(name);
    } catch (e) {
        console.error('Failed to create workspace:', e);
        showNotification('Failed to create: ' + e, 'error');
    }
}

// Event Listeners
function setupEventListeners() {
    console.log('Setting up event listeners');
    
    // Main buttons
    const btnCreate = document.getElementById('btn-create');
    const btnCancel = document.getElementById('btn-cancel');
    const btnExport = document.getElementById('btn-export');
    const btnImport = document.getElementById('btn-import');
    const btnDetect = document.getElementById('btn-detect');
    const btnLoadFile = document.getElementById('btn-load-file');
    const btnLoadProject = document.getElementById('btn-load-project');
    const btnCancelLoad = document.getElementById('btn-cancel-load');
    const formCreate = document.getElementById('form-create');
    const formLoad = document.getElementById('form-load-project');
    const navDashboard = document.getElementById('nav-dashboard');
    const navWorkspaces = document.getElementById('nav-workspaces');
    const navTools = document.getElementById('nav-tools');
    
    if (btnCreate) btnCreate.onclick = showCreateModal;
    if (btnCancel) btnCancel.onclick = hideCreateModal;
    if (btnExport) btnExport.onclick = exportWorkspaces;
    if (btnImport) btnImport.onclick = importWorkspaces;
    if (btnDetect) btnDetect.onclick = showTools;
    if (btnLoadFile) btnLoadFile.onclick = loadWorkspaceFromFile;
    if (btnLoadProject) btnLoadProject.onclick = showLoadProjectModal;
    if (btnCancelLoad) btnCancelLoad.onclick = hideLoadProjectModal;
    if (formCreate) formCreate.onsubmit = createWorkspace;
    if (formLoad) formLoad.onsubmit = loadProjectFromData;
    
    // Navigation
    if (navDashboard) navDashboard.onclick = showDashboard;
    if (navWorkspaces) navWorkspaces.onclick = () => {
        showDashboard();
        if (els.workspaceList) {
            els.workspaceList.scrollIntoView({ behavior: 'smooth' });
        }
    };
    if (navTools) navTools.onclick = showTools;
    
    // Close modals on overlay click
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.onclick = () => {
            hideCreateModal();
            hideLoadProjectModal();
        };
    });
    
    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            hideCreateModal();
            hideLoadProjectModal();
        }
    });
    
    console.log('Event listeners set up');
}

// Utility functions
function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function formatCategory(category) {
    const map = {
        'terminal': '🖥️ Terminal Emulators',
        'clieditor': '📝 CLI Editors',
        'guiide': '🎨 GUI IDEs',
        'aiassistant': '🤖 AI Assistants',
        'browser': '🌐 Browsers'
    };
    return map[category.toLowerCase()] || category;
}

function getToolIcon(category) {
    const map = {
        'terminal': '🖥️',
        'clieditor': '📝',
        'guiide': '🎨',
        'aiassistant': '🤖',
        'browser': '🌐'
    };
    return map[category.toLowerCase()] || '🔧';
}

// Expose functions to window for onclick handlers
window.selectProject = selectProject;
window.showWorkspaceDetail = showWorkspaceDetail;
window.activateProject = activateProject;
window.loadWorkspaceFromFile = loadWorkspaceFromFile;
window.showLoadProjectModal = showLoadProjectModal;
window.hideLoadProjectModal = hideLoadProjectModal;
window.loadProjectFromData = loadProjectFromData;
window.showDashboard = showDashboard;
window.showTools = showTools;
window.showCreateModal = showCreateModal;
window.hideCreateModal = hideCreateModal;
window.exportWorkspaces = exportWorkspaces;
window.importWorkspaces = importWorkspaces;

// Start
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
