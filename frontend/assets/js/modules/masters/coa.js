/**
 * coa.js — Chart of Accounts module (tree view + CRUD)
 */

'use strict';

import { MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, Validator,
  Badge, FormGroup, SelectGroup,
} from '../../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────────────────

let _treeData  = [];      // raw flat rows
let _modal     = null;
let _editId    = null;
let _collapsed = new Set(); // collapsed node IDs

// ─────────────────────────────────────────────────────────────────────────────
// Build tree structure from flat list
// ─────────────────────────────────────────────────────────────────────────────

function buildTree(rows, parentId = null) {
  return rows
    .filter(r => r.parent_id === parentId)
    .map(r => ({ ...r, children: buildTree(rows, r.id) }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Render tree HTML
// ─────────────────────────────────────────────────────────────────────────────

const TYPE_COLOR = {
  asset:     'info',
  liability: 'warning',
  equity:    'success',
  income:    'primary',
  expense:   'danger',
};

function renderNode(node, depth = 0) {
  const hasChildren = node.children && node.children.length > 0;
  const isCollapsed = _collapsed.has(node.id);
  const indent      = depth * 20;

  const chevron = hasChildren
    ? `<span class="coa-chevron${isCollapsed ? ' collapsed' : ''}" data-toggle="${node.id}">
         <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" width="12" height="12"><polyline points="6 9 12 15 18 9"/></svg>
       </span>`
    : `<span style="display:inline-block;width:20px"></span>`;

  const typeChip = Badge(node.account_type, TYPE_COLOR[node.account_type] || 'default', true);
  const ctrlChip = node.is_control ? `<span class="badge badge-outline" style="font-size:0.7rem;margin-left:4px">Control</span>` : '';
  const inactive = !node.is_active ? `<span style="font-size:0.75rem;color:var(--text-muted);margin-left:6px">[Inactive]</span>` : '';

  let rows = `
    <tr class="coa-row${!node.is_active ? ' coa-inactive' : ''}" data-id="${node.id}" data-depth="${depth}">
      <td style="padding-left:${indent + 12}px">
        ${chevron}
        <span class="coa-code">${node.code}</span>
        <span class="coa-name">${node.name}${inactive}</span>
      </td>
      <td>${typeChip}${ctrlChip}</td>
      <td style="text-align:center">${node.level}</td>
      <td>
        <div class="row-actions">
          <button class="btn btn-sm btn-secondary btn-icon" data-action="edit" data-id="${node.id}" title="Edit">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg>
          </button>
          <button class="btn btn-sm btn-secondary btn-icon" data-action="add-child" data-id="${node.id}" data-type="${node.account_type}" title="Add sub-account">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          </button>
          ${!hasChildren ? `
          <button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete" data-id="${node.id}" data-name="${node.name}" title="Deactivate">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/></svg>
          </button>` : ''}
        </div>
      </td>
    </tr>
  `;

  if (hasChildren && !isCollapsed) {
    for (const child of node.children) {
      rows += renderNode(child, depth + 1);
    }
  }
  return rows;
}

function renderTree(container) {
  const tree = buildTree(_treeData);

  if (_treeData.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="48" height="48"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg>
        <p>No accounts found. Add a root account to get started.</p>
      </div>`;
    return;
  }

  container.innerHTML = `
    <table class="table coa-table">
      <thead>
        <tr>
          <th>Account Code / Name</th>
          <th style="width:160px">Type</th>
          <th style="width:60px;text-align:center">Level</th>
          <th style="width:100px"></th>
        </tr>
      </thead>
      <tbody id="coa-tbody">
        ${tree.map(node => renderNode(node, 0)).join('')}
      </tbody>
    </table>
  `;

  // Collapse toggle
  container.querySelectorAll('[data-toggle]').forEach(el => {
    el.addEventListener('click', e => {
      e.stopPropagation();
      const id = el.dataset.toggle;
      if (_collapsed.has(id)) _collapsed.delete(id);
      else _collapsed.add(id);
      renderTree(container);
    });
  });

  // Row actions
  container.querySelectorAll('[data-action]').forEach(btn => {
    btn.addEventListener('click', async e => {
      e.stopPropagation();
      const { action, id, name, type } = btn.dataset;

      if (action === 'edit') {
        try {
          const detail = await fetch(`/api/masters/coa/${id}`, {
            headers: { Authorization: `Bearer ${sessionStorage.getItem('token')}` },
          }).then(r => r.json());
          openModal(detail);
        } catch { Toast.error('Failed to load account.'); }
      }

      if (action === 'add-child') {
        openModal(null, { parent_id: id, account_type: type });
      }

      if (action === 'delete') {
        const ok = await Confirm({
          title:       'Deactivate Account?',
          message:     `"${name}" will be deactivated. This cannot be undone if it has transactions.`,
          type:        'danger',
          confirmText: 'Deactivate',
        });
        if (ok) {
          try {
            await MastersApi.coa.delete(id);
            Toast.success('Account deactivated.');
            await loadTree(container);
          } catch (err) { Toast.error(err.message); }
        }
      }
    });
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Load tree data
// ─────────────────────────────────────────────────────────────────────────────

async function loadTree(container) {
  try {
    _treeData = await MastersApi.coa.list({ active_only: false });
    renderTree(container);
  } catch {
    Toast.error('Failed to load chart of accounts.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Modal form
// ─────────────────────────────────────────────────────────────────────────────

function buildForm(data = {}, defaults = {}) {
  const merged      = { ...defaults, ...data };
  const isEdit      = !!data.id;
  const parentFixed = !!defaults.parent_id && !isEdit;

  const typeOpts = [
    { value: 'asset',     label: 'Asset' },
    { value: 'liability', label: 'Liability' },
    { value: 'equity',    label: 'Equity' },
    { value: 'income',    label: 'Income' },
    { value: 'expense',   label: 'Expense' },
  ];

  // Parent dropdown — only root-level accounts of same type
  const parentOpts = _treeData
    .filter(a => a.is_active && a.is_control)
    .map(a => ({ value: a.id, label: `${a.code} — ${a.name}` }));

  return `
    <div class="grid-2" style="padding-top:0.5rem">
      ${FormGroup({ label:'Account Code', id:'coa-code', required:true, value: merged.code||'', placeholder:'e.g. 1001', extra: isEdit ? 'readonly' : '' })}
      ${FormGroup({ label:'Account Name', id:'coa-name', required:true, value: merged.name||'', placeholder:'e.g. Cash in Hand' })}
      ${SelectGroup({ label:'Account Type', id:'coa-type', value: merged.account_type||'asset', options: typeOpts, extra: (isEdit || parentFixed) ? 'disabled' : '' })}
      ${SelectGroup({ label:'Parent Account', id:'coa-parent', value: merged.parent_id||'', options: parentOpts, extra: parentFixed ? 'disabled' : '' })}
    </div>
    <div class="grid-2" style="margin-top:0.5rem">
      <div class="form-group">
        <label class="form-label" for="coa-control">
          <input type="checkbox" id="coa-control" style="margin-right:6px" ${merged.is_control ? 'checked' : ''}>
          Control Account (no direct postings)
        </label>
      </div>
      ${isEdit ? `
        <div class="form-group">
          <label class="form-label" for="coa-active">
            <input type="checkbox" id="coa-active" style="margin-right:6px" ${merged.is_active ? 'checked' : ''}>
            Active
          </label>
        </div>
      ` : ''}
    </div>
    <div class="form-group">
      <label class="form-label" for="coa-desc">Description</label>
      <textarea class="input" id="coa-desc" rows="2" placeholder="Optional">${merged.description||''}</textarea>
    </div>
  `;
}

const COA_SCHEMA = {
  'coa-code': { required: true, minLength: 2, maxLength: 20 },
  'coa-name': { required: true, minLength: 2, maxLength: 150 },
};

function collectCOAForm(defaults = {}) {
  const g = id => document.getElementById(id)?.value?.trim() || null;
  const c = id => document.getElementById(id)?.checked || false;
  return {
    code:         g('coa-code'),
    name:         g('coa-name'),
    account_type: g('coa-type') || defaults.account_type || 'asset',
    parent_id:    g('coa-parent') || defaults.parent_id || null,
    is_control:   c('coa-control'),
    is_active:    document.getElementById('coa-active') ? c('coa-active') : undefined,
    description:  g('coa-desc') || undefined,
  };
}

async function openModal(editData = null, defaults = {}) {
  _editId = editData?.id || null;
  if (_modal) _modal.close();
  _modal = new Modal({
    title:   _editId ? 'Edit Account' : 'New Account',
    size:    'md',
    content: buildForm(editData || {}, defaults),
  });
  _modal.open();

  _modal.setFooter(`
    <button class="btn btn-secondary" id="coa-cancel">Cancel</button>
    <button class="btn btn-primary" id="coa-save">${_editId ? 'Save Changes' : 'Create Account'}</button>
  `);

  const _defaults = defaults;
  _modal.getFooter().querySelector('#coa-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#coa-save').addEventListener('click', () => saveCOA(_defaults));
}

async function saveCOA(defaults = {}) {
  const data = collectCOAForm(defaults);
  let hasError = false;
  for (const [id, rules] of Object.entries(COA_SCHEMA)) {
    const el = document.getElementById(id);
    if (!el) continue;
    const r = Validator.validate(el.value, rules);
    if (!r.valid) { Validator.showError(el, r.message); hasError = true; }
    else Validator.clearError(el);
  }
  if (hasError) return;

  const btn = document.getElementById('coa-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    if (_editId) {
      const payload = { name: data.name, is_control: data.is_control, description: data.description };
      if (data.is_active !== undefined) payload.is_active = data.is_active;
      await MastersApi.coa.update(_editId, payload);
      Toast.success('Account updated.');
    } else {
      await MastersApi.coa.create(data);
      Toast.success('Account created.');
    }
    _modal.close();
    const container = document.getElementById('coa-tree-container');
    if (container) await loadTree(container);
  } catch (err) {
    Toast.error(err.message || 'Save failed.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Chart of Accounts</h1>
        <p class="page-subtitle">Manage the account hierarchy for double-entry bookkeeping</p>
      </div>
      <div style="display:flex;gap:0.5rem">
        <button class="btn btn-secondary" id="coa-expand-all">Expand All</button>
        <button class="btn btn-secondary" id="coa-collapse-all">Collapse All</button>
        <button class="btn btn-primary" id="add-coa-btn">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
          New Account
        </button>
      </div>
    </div>

    <style>
      .coa-table td, .coa-table th { padding: 8px 12px; }
      .coa-code { font-family: monospace; font-size: 0.82rem; color: var(--text-muted); margin-right: 10px; }
      .coa-name { font-weight: 500; }
      .coa-inactive .coa-name { color: var(--text-muted); font-style: italic; }
      .coa-chevron { display: inline-flex; align-items: center; cursor: pointer; margin-right: 4px; transition: transform 0.15s; }
      .coa-chevron.collapsed { transform: rotate(-90deg); }
      .coa-chevron:hover { color: var(--primary); }
    </style>

    <div class="card">
      <div class="card-body" style="padding:0">
        <div id="coa-tree-container">
          <div class="skeleton" style="height:300px;border-radius:0"></div>
        </div>
      </div>
    </div>
  `;

  const treeEl = document.getElementById('coa-tree-container');
  await loadTree(treeEl);

  document.getElementById('add-coa-btn').addEventListener('click', () => openModal());

  document.getElementById('coa-expand-all').addEventListener('click', () => {
    _collapsed.clear();
    renderTree(treeEl);
  });

  document.getElementById('coa-collapse-all').addEventListener('click', () => {
    _treeData.forEach(r => { if (r.level === 1) _collapsed.add(r.id); });
    renderTree(treeEl);
  });
}

export default { init };
