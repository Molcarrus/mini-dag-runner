// Global state
let currentDagId = null;
let eventSource = null;
let refreshInterval = null;

// API base URL
const API_BASE = '/api';

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    loadDagList();
    // Refresh DAG list periodically
    refreshInterval = setInterval(loadDagList, 5000);
});

// Load and display DAG list
async function loadDagList() {
    try {
        const response = await fetch(`${API_BASE}/dags`);
        const data = await response.json();
        renderDagList(data.dags);
    } catch (error) {
        console.error('Failed to load DAG list:', error);
        document.getElementById('dagList').innerHTML = `
            <div class="empty-state small">
                <p>Failed to load DAGs</p>
            </div>
        `;
    }
}

// Render DAG list in sidebar
function renderDagList(dags) {
    const container = document.getElementById('dagList');
    
    if (dags.length === 0) {
        container.innerHTML = `
            <div class="empty-state small">
                <p>No DAGs yet</p>
            </div>
        `;
        return;
    }
    
    container.innerHTML = dags.map(dag => `
        <div class="dag-card ${dag.id === currentDagId ? 'active' : ''}" onclick="selectDag('${dag.id}')">
            <div class="dag-card-header">
                <span class="dag-card-name">${dag.name || 'Unnamed DAG'}</span>
                <span class="status-badge ${dag.status}">${formatStatus(dag.status)}</span>
            </div>
            <div class="dag-card-id">${dag.id.slice(0, 8)}...</div>
            <div class="dag-card-meta">
                <span>${dag.task_count} tasks</span>
                <span>${formatTime(dag.created_at)}</span>
            </div>
        </div>
    `).join('');
}

// Select and load a DAG
async function selectDag(dagId) {
    currentDagId = dagId;
    
    // Update UI
    document.querySelectorAll('.dag-card').forEach(card => {
        card.classList.remove('active');
        if (card.onclick.toString().includes(dagId)) {
            card.classList.add('active');
        }
    });
    
    // Load DAG details
    await loadDagDetails(dagId);
    
    // Connect to SSE stream
    connectToEventStream(dagId);
}

// Load DAG details
async function loadDagDetails(dagId) {
    try {
        const response = await fetch(`${API_BASE}/dags/${dagId}/status`);
        const data = await response.json();
        renderDagVisualization(data);
    } catch (error) {
        console.error('Failed to load DAG details:', error);
    }
}

// Render DAG visualization
function renderDagVisualization(data) {
    const { dag, progress } = data;
    
    // Update header
    const header = document.getElementById('dagHeader');
    header.innerHTML = `
        <div>
            <h2>${dag.name || 'Unnamed DAG'}</h2>
            <span style="font-size: 0.75rem; color: var(--text-muted); font-family: 'JetBrains Mono', monospace;">${dag.id}</span>
        </div>
        <div class="progress-container">
            <div class="progress-bar">
                <div class="progress-fill" style="width: ${progress.percent_complete}%"></div>
            </div>
            <span class="progress-text">${Math.round(progress.percent_complete)}% (${progress.succeeded + progress.failed + progress.skipped}/${progress.total})</span>
            <span class="status-badge ${dag.status}">${formatStatus(dag.status)}</span>
        </div>
    `;
    
    // Build levels from tasks
    const levels = dag.execution_levels;
    const container = document.getElementById('dagVisualization');
    
    container.innerHTML = `
        <div class="execution-levels">
            ${levels.map((levelTasks, levelIdx) => `
                <div class="level">
                    <div class="level-header">
                        <span>Level ${levelIdx}</span>
                        <div class="level-line"></div>
                        <span>${levelTasks.length} task${levelTasks.length > 1 ? 's' : ''}</span>
                    </div>
                    <div class="level-tasks">
                        ${levelTasks.map(taskId => {
                            const task = dag.tasks[taskId];
                            return renderTaskNode(task);
                        }).join('')}
                    </div>
                </div>
            `).join('')}
        </div>
    `;
}

// Render a single task node
function renderTaskNode(task) {
    const status = getTaskStatusClass(task.status);
    const icon = getStatusIcon(status);
    const deps = task.definition.depends_on;
    
    return `
        <div class="task-node ${status}" onclick='showTaskDetails(${JSON.stringify(task).replace(/'/g, "\\'")})'>
            <div class="task-icon">${icon}</div>
            <div class="task-name">${task.definition.id}</div>
            <div class="task-command">${task.definition.command}</div>
            ${deps.length > 0 ? `<div class="task-deps">← ${deps.join(', ')}</div>` : ''}
            ${task.duration_ms ? `<div class="task-duration">⏱ ${task.duration_ms}ms</div>` : ''}
        </div>
    `;
}

// Get task status class
function getTaskStatusClass(status) {
    if (typeof status === 'object' && status.failed) {
        return 'failed';
    }
    return status;
}

// Get status icon
function getStatusIcon(status) {
    switch (status) {
        case 'pending': return '⏳';
        case 'running': return '🔄';
        case 'succeeded': return '✓';
        case 'failed': return '✗';
        case 'skipped': return '⊘';
        default: return '?';
    }
}

// Connect to SSE event stream
function connectToEventStream(dagId) {
    // Close existing connection
    if (eventSource) {
        eventSource.close();
    }
    
    eventSource = new EventSource(`${API_BASE}/dags/${dagId}/stream`);
    
    eventSource.addEventListener('dag_started', (e) => {
        const event = JSON.parse(e.data);
        addEventToLog('dag_started', event);
    });
    
    eventSource.addEventListener('task_started', (e) => {
        const event = JSON.parse(e.data);
        addEventToLog('task_started', event);
        updateTaskStatus(event.task_id, 'running');
    });
    
    eventSource.addEventListener('task_completed', (e) => {
        const event = JSON.parse(e.data);
        addEventToLog('task_completed', event);
        updateTaskStatus(event.task_id, getTaskStatusClass(event.status));
        loadDagDetails(dagId); // Refresh to show duration
    });
    
    eventSource.addEventListener('level_completed', (e) => {
        const event = JSON.parse(e.data);
        addEventToLog('level_completed', event);
    });
    
    eventSource.addEventListener('dag_completed', (e) => {
        const event = JSON.parse(e.data);
        addEventToLog('dag_completed', event);
        loadDagDetails(dagId); // Refresh final state
        loadDagList(); // Refresh sidebar
    });
    
    eventSource.addEventListener('error', (e) => {
        console.log('SSE connection error or closed');
    });
}

// Update task status in visualization
function updateTaskStatus(taskId, status) {
    const nodes = document.querySelectorAll('.task-node');
    nodes.forEach(node => {
        if (node.querySelector('.task-name').textContent === taskId) {
            node.className = `task-node ${status}`;
            node.querySelector('.task-icon').textContent = getStatusIcon(status);
        }
    });
}

// Add event to log
function addEventToLog(type, event) {
    const log = document.getElementById('eventLog');
    
    // Remove empty state if present
    const emptyState = log.querySelector('.empty-state');
    if (emptyState) {
        emptyState.remove();
    }
    
    const statusClass = event.status ? getTaskStatusClass(event.status) : '';
    const time = new Date().toLocaleTimeString();
    
    let body = '';
    switch (type) {
        case 'dag_started':
            body = `DAG execution started with ${event.total_tasks} tasks`;
            break;
        case 'task_started':
            body = `Task <code>${event.task_id}</code> started (Level ${event.level})`;
            break;
        case 'task_completed':
            const status = getTaskStatusClass(event.status);
            body = `Task <code>${event.task_id}</code> ${status} in ${event.duration_ms}ms`;
            break;
        case 'level_completed':
            body = `Level ${event.level} completed (${event.tasks_in_level.join(', ')})`;
            break;
        case 'dag_completed':
            body = `DAG ${getTaskStatusClass(event.status)} in ${event.total_duration_ms}ms`;
            break;
        default:
            body = JSON.stringify(event);
    }
    
    const eventHtml = `
        <div class="event-item ${type} ${statusClass}">
            <div class="event-header">
                <span class="event-type">${type.replace('_', ' ')}</span>
                <span class="event-time">${time}</span>
            </div>
            <div class="event-body">${body}</div>
        </div>
    `;
    
    log.insertAdjacentHTML('afterbegin', eventHtml);
    
    // Keep only last 50 events
    const events = log.querySelectorAll('.event-item');
    if (events.length > 50) {
        events[events.length - 1].remove();
    }
}

// Clear event log
function clearEvents() {
    const log = document.getElementById('eventLog');
    log.innerHTML = `
        <div class="empty-state small">
            <p>Events will appear here</p>
        </div>
    `;
}

// Show task details modal
function showTaskDetails(task) {
    const modal = document.getElementById('taskModal');
    const body = document.getElementById('taskModalBody');
    
    const status = getTaskStatusClass(task.status);
    const error = typeof task.status === 'object' && task.status.failed ? task.status.failed.error : null;
    
    body.innerHTML = `
        <div class="form-group">
            <label>Task ID</label>
            <div style="font-family: 'JetBrains Mono', monospace; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px;">${task.definition.id}</div>
        </div>
        <div class="form-group">
            <label>Command</label>
            <div style="font-family: 'JetBrains Mono', monospace; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px; color: var(--accent-purple);">${task.definition.command}</div>
        </div>
        <div class="form-group">
            <label>Status</label>
            <span class="status-badge ${status}">${status}</span>
        </div>
        ${task.duration_ms ? `
            <div class="form-group">
                <label>Duration</label>
                <div>${task.duration_ms}ms</div>
            </div>
        ` : ''}
        ${task.output ? `
            <div class="form-group">
                <label>Output</label>
                <div style="font-family: 'JetBrains Mono', monospace; padding: 0.5rem; background: var(--bg-tertiary); border-radius: 4px; font-size: 0.8rem;">${task.output}</div>
            </div>
        ` : ''}
        ${error ? `
            <div class="form-group">
                <label>Error</label>
                <div style="font-family: 'JetBrains Mono', monospace; padding: 0.5rem; background: rgba(248, 81, 73, 0.1); border-radius: 4px; font-size: 0.8rem; color: var(--accent-red);">${error}</div>
            </div>
        ` : ''}
        ${task.definition.depends_on.length > 0 ? `
            <div class="dependency-info">
                <h4>Dependencies</h4>
                <div class="dependency-list">
                    ${task.definition.depends_on.map(dep => `<span class="dependency-tag">${dep}</span>`).join('')}
                </div>
            </div>
        ` : ''}
    `;
    
    modal.classList.add('open');
}

// Close task modal
function closeTaskModal() {
    document.getElementById('taskModal').classList.remove('open');
}

// Open submit modal
function openSubmitModal() {
    document.getElementById('submitModal').classList.add('open');
}

// Close submit modal
function closeSubmitModal() {
    document.getElementById('submitModal').classList.remove('open');
}

// Load template into form
function loadTemplate(name) {
    const template = templates[name];
    if (template) {
        document.getElementById('dagName').value = template.name;
        document.getElementById('dagJson').value = JSON.stringify(template, null, 2);
    }
}

// Submit DAG
async function submitDag() {
    const name = document.getElementById('dagName').value;
    const jsonStr = document.getElementById('dagJson').value;
    
    let payload;
    try {
        payload = JSON.parse(jsonStr);
    } catch (e) {
        alert('Invalid JSON: ' + e.message);
        return;
    }
    
    // If name is provided separately, override
    if (name && !payload.name) {
        payload.name = name;
    }
    
    try {
        const response = await fetch(`${API_BASE}/dags`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload),
        });
        
        if (!response.ok) {
            const error = await response.json();
            alert('Failed to submit DAG: ' + error.error);
            return;
        }
        
        const data = await response.json();
        
        // Close modal and refresh
        closeSubmitModal();
        await loadDagList();
        
        // Select the new DAG
        selectDag(data.dag_id);
        
        // Clear form
        document.getElementById('dagName').value = '';
        document.getElementById('dagJson').value = '';
        
    } catch (error) {
        alert('Failed to submit DAG: ' + error.message);
    }
}

// Format status for display
function formatStatus(status) {
    return status.replace('_', ' ');
}

// Format time
function formatTime(isoString) {
    const date = new Date(isoString);
    return date.toLocaleTimeString();
}