/**
 * masters.js — Lookup tables module
 * Manages: UOM, Currencies, Tax Config, Payment Terms,
 *           Inventory Locations, Fiscal Years, Doc Sequences
 */

'use strict';

import { MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, Validator, FormGroup, SelectGroup, formatAmount,
} from '../../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// Active section state
// ─────────────────────────────────────────────────────────────────────────────

let _section = 'uom';
let _table   = null;
let _modal   = null;
let _editId  = null;

const SECTIONS = [
  { id: 'uom',          label: 'Units of Measure' },
  { id: 'currencies',   label: 'Currencies' },
  { id: 'tax',          label: 'Tax Config' },
  { id: 'payment-terms',label: 'Payment Terms' },
  { id: 'locations',    label: 'Locations' },
  { id: 'fiscal-years', label: 'Fiscal Years' },
  { id: 'doc-seq',      label: 'Doc Sequences' },
];

// ─────────────────────────────────────────────────────────────────────────────
// UOM
// ─────────────────────────────────────────────────────────────────────────────

const UOM = {
  columns: [
    { key: 'code',      label: 'Code',   width: '100px' },
    { key: 'name',      label: 'Name' },
    { key: 'is_active', label: 'Status', width: '80px',
      render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
    { key: '_actions', label: '', sortable: false, render: (_, row) => rowActions(row) },
  ],
  async list()  { return (await MastersApi.uom.list({ active_only: false })) || []; },
  buildForm(data = {}) {
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Code', id:'frm-code', required:true, value: data.code||'', placeholder:'e.g. KG', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. Kilogram' })}
      </div>
    `;
  },
  schema: {
    'frm-code': { required: true },
    'frm-name': { required: true },
  },
  collect() {
    return {
      code: document.getElementById('frm-code')?.value?.trim().toUpperCase(),
      name: document.getElementById('frm-name')?.value?.trim(),
    };
  },
  async create(d) { return MastersApi.uom.create(d); },
  async update(id, d) { return MastersApi.uom.update(id, { name: d.name }); },
};

// ─────────────────────────────────────────────────────────────────────────────
// Currencies
// ─────────────────────────────────────────────────────────────────────────────

const CURRENCIES = {
  columns: [
    { key: 'code',          label: 'Code',   width: '70px' },
    { key: 'name',          label: 'Name' },
    { key: 'symbol',        label: 'Symbol', width: '60px' },
    { key: 'exchange_rate', label: 'Rate',   align: 'right', render: v => v.toFixed(4) },
    { key: 'is_base',       label: 'Base',   width: '60px',  render: v => v ? Badge('Yes','success',true) : '' },
    { key: 'is_active',     label: 'Status', width: '80px',
      render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
    { key: '_actions', label: '', sortable: false, render: (_, row) => rowActions(row) },
  ],
  async list()  { return (await MastersApi.currencies.list({ active_only: false })) || []; },
  buildForm(data = {}) {
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Code (ISO)', id:'frm-code', required:true, value: data.code||'', placeholder:'e.g. USD', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. US Dollar' })}
        ${FormGroup({ label:'Symbol', id:'frm-symbol', required:true, value: data.symbol||'', placeholder:'e.g. $' })}
        ${FormGroup({ label:'Exchange Rate (to PKR)', id:'frm-rate', type:'number', required:true, value: data.exchange_rate||1, placeholder:'e.g. 278.50' })}
      </div>
      <div class="form-group" style="margin-top:0.5rem">
        <label class="form-label">
          <input type="checkbox" id="frm-is-base" style="margin-right:6px" ${data.is_base ? 'checked' : ''}>
          Base Currency (all amounts stored in this currency)
        </label>
      </div>
    `;
  },
  schema: {
    'frm-code':   { required: true, minLength: 3, maxLength: 3 },
    'frm-name':   { required: true },
    'frm-symbol': { required: true },
    'frm-rate':   { required: true },
  },
  collect() {
    const g  = id => document.getElementById(id)?.value?.trim() || null;
    const gn = id => parseFloat(document.getElementById(id)?.value) || 1;
    return {
      code:          g('frm-code'),
      name:          g('frm-name'),
      symbol:        g('frm-symbol'),
      exchange_rate: gn('frm-rate'),
      is_base:       document.getElementById('frm-is-base')?.checked || false,
    };
  },
  async create(d) { return MastersApi.currencies.create(d); },
  async update(id, d) { return MastersApi.currencies.update(id, { exchange_rate: d.exchange_rate, name: d.name, symbol: d.symbol }); },
};

// ─────────────────────────────────────────────────────────────────────────────
// Tax Config
// ─────────────────────────────────────────────────────────────────────────────

const TAX = {
  columns: [
    { key: 'name',       label: 'Name' },
    { key: 'tax_type',   label: 'Type',       width: '80px' },
    { key: 'rate',       label: 'Rate %',     align: 'right', render: v => `${v}%` },
    { key: 'applies_to', label: 'Applies To', width: '100px' },
    { key: 'is_active',  label: 'Status',     width: '80px',
      render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
    { key: '_actions', label: '', sortable: false, render: (_, row) => rowActions(row) },
  ],
  async list() {
    const res = await fetch('/api/masters/tax?active_only=false', {
      headers: { Authorization: `Bearer ${sessionStorage.getItem('token')}` },
    }).then(r => r.json()).catch(() => []);
    return res || [];
  },
  buildForm(data = {}) {
    const typeOpts = [
      { value: 'GST',    label: 'GST — General Sales Tax' },
      { value: 'WHT',    label: 'WHT — Withholding Tax' },
      { value: 'FED',    label: 'FED — Federal Excise Duty' },
      { value: 'SED',    label: 'SED — Special Excise Duty' },
      { value: 'custom', label: 'Custom' },
    ];
    const appOpts = [
      { value: 'purchases', label: 'Purchases' },
      { value: 'sales',     label: 'Sales' },
      { value: 'both',      label: 'Both' },
    ];
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Tax Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. GST 17%' })}
        ${SelectGroup({ label:'Tax Type', id:'frm-type', value: data.tax_type||'GST', options: typeOpts })}
        ${FormGroup({ label:'Rate (%)', id:'frm-rate', type:'number', required:true, value: data.rate||0, placeholder:'e.g. 17' })}
        ${SelectGroup({ label:'Applies To', id:'frm-applies', value: data.applies_to||'both', options: appOpts })}
      </div>
    `;
  },
  schema: {
    'frm-name': { required: true },
    'frm-rate': { required: true, type: 'percent' },
  },
  collect() {
    const g  = id => document.getElementById(id)?.value?.trim() || null;
    const gn = id => parseFloat(document.getElementById(id)?.value) || 0;
    return {
      name:       g('frm-name'),
      tax_type:   g('frm-type') || 'GST',
      rate:       gn('frm-rate'),
      applies_to: g('frm-applies') || 'both',
    };
  },
  async create(d) {
    return fetch('/api/masters/tax', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionStorage.getItem('token')}` },
      body:    JSON.stringify(d),
    }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(new Error(e.detail))); return r.json(); });
  },
  async update(id, d) {
    return fetch(`/api/masters/tax/${id}`, {
      method:  'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionStorage.getItem('token')}` },
      body:    JSON.stringify({ name: d.name, rate: d.rate }),
    }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(new Error(e.detail))); return r.json(); });
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Payment Terms
// ─────────────────────────────────────────────────────────────────────────────

const PAYMENT_TERMS = {
  columns: [
    { key: 'name',          label: 'Name' },
    { key: 'due_days',      label: 'Due Days', align: 'right', width: '100px' },
    { key: 'discount_days', label: 'Disc Days', align: 'right', width: '100px', render: v => v||'—' },
    { key: 'discount_pct',  label: 'Disc %',   align: 'right', width: '80px',  render: v => v ? `${v}%` : '—' },
    { key: 'is_active',     label: 'Status',   width: '80px',
      render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
    { key: '_actions', label: '', sortable: false, render: (_, row) => rowActions(row) },
  ],
  async list() { return (await MastersApi.paymentTerms.list({ active_only: false })) || []; },
  buildForm(data = {}) {
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. Net 30 Days' })}
        ${FormGroup({ label:'Due Days', id:'frm-due', type:'number', required:true, value: data.due_days||30 })}
        ${FormGroup({ label:'Discount Days', id:'frm-disc-days', type:'number', value: data.discount_days||'', placeholder:'Optional' })}
        ${FormGroup({ label:'Discount %', id:'frm-disc-pct', type:'number', value: data.discount_pct||'', placeholder:'Optional' })}
      </div>
    `;
  },
  schema: {
    'frm-name': { required: true },
    'frm-due':  { required: true },
  },
  collect() {
    const g  = id => document.getElementById(id)?.value?.trim() || null;
    const gi = id => parseInt(document.getElementById(id)?.value) || 0;
    const gn = id => { const v = parseFloat(document.getElementById(id)?.value); return isNaN(v) ? null : v; };
    return {
      name:          g('frm-name'),
      due_days:      gi('frm-due'),
      discount_days: gn('frm-disc-days'),
      discount_pct:  gn('frm-disc-pct'),
    };
  },
  async create(d) { return MastersApi.paymentTerms.create(d); },
  async update(id, d) {
    return fetch(`/api/masters/payment-terms/${id}`, {
      method:  'PUT',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionStorage.getItem('token')}` },
      body:    JSON.stringify({ name: d.name, due_days: d.due_days, discount_days: d.discount_days, discount_pct: d.discount_pct }),
    }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(new Error(e.detail))); return r.json(); });
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Inventory Locations
// ─────────────────────────────────────────────────────────────────────────────

const LOCATIONS = {
  columns: [
    { key: 'code',      label: 'Code',    width: '100px' },
    { key: 'name',      label: 'Name' },
    { key: 'address',   label: 'Address', render: v => v||'—' },
    { key: 'is_active', label: 'Status',  width: '80px',
      render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
    { key: '_actions', label: '', sortable: false, render: (_, row) => rowActions(row) },
  ],
  async list() { return (await MastersApi.locations.list({ active_only: false })) || []; },
  buildForm(data = {}) {
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Location Code', id:'frm-code', required:true, value: data.code||'', placeholder:'e.g. WH-01', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Location Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. Main Warehouse' })}
      </div>
      <div class="form-group">
        <label class="form-label" for="frm-address">Address</label>
        <textarea class="input" id="frm-address" rows="2" placeholder="Optional">${data.address||''}</textarea>
      </div>
    `;
  },
  schema: {
    'frm-code': { required: true },
    'frm-name': { required: true },
  },
  collect() {
    const g = id => document.getElementById(id)?.value?.trim() || null;
    return { code: g('frm-code'), name: g('frm-name'), address: g('frm-address') || undefined };
  },
  async create(d) { return MastersApi.locations.create(d); },
  async update(id, d) { return MastersApi.locations.update(id, { name: d.name, address: d.address }); },
};

// ─────────────────────────────────────────────────────────────────────────────
// Fiscal Years
// ─────────────────────────────────────────────────────────────────────────────

const FISCAL_YEARS = {
  columns: [
    { key: 'name',       label: 'Name',       width: '120px' },
    { key: 'start_date', label: 'Start',      width: '110px' },
    { key: 'end_date',   label: 'End',        width: '110px' },
    { key: 'is_active',  label: 'Active',     width: '70px',  render: v => v ? Badge('Yes','success',true) : '' },
    { key: 'is_closed',  label: 'Status',     width: '90px',
      render: v => v ? Badge('Closed','danger',true) : Badge('Open','success',true) },
    {
      key: '_actions', label: '', sortable: false,
      render: (_, row) => `
        ${!row.is_closed ? `
          <button class="btn btn-sm btn-danger" data-action="close" data-id="${row.id}" data-name="${row.name}" style="font-size:0.78rem;padding:3px 10px">Close Year</button>
        ` : ''}
      `,
    },
  ],
  async list() {
    const res = await fetch('/api/masters/fiscal-years', {
      headers: { Authorization: `Bearer ${sessionStorage.getItem('token')}` },
    }).then(r => r.json()).catch(() => []);
    return res || [];
  },
  buildForm(data = {}) {
    return `
      <div class="grid-2" style="padding-top:0.5rem">
        ${FormGroup({ label:'Year Name', id:'frm-name', required:true, value: data.name||'', placeholder:'e.g. FY 2024-25' })}
      </div>
      <div class="grid-2">
        ${FormGroup({ label:'Start Date', id:'frm-start', type:'date', required:true, value: data.start_date||'' })}
        ${FormGroup({ label:'End Date', id:'frm-end', type:'date', required:true, value: data.end_date||'' })}
      </div>
    `;
  },
  schema: {
    'frm-name':  { required: true },
    'frm-start': { required: true },
    'frm-end':   { required: true },
  },
  collect() {
    const g = id => document.getElementById(id)?.value?.trim() || null;
    return { name: g('frm-name'), start_date: g('frm-start'), end_date: g('frm-end') };
  },
  async create(d) {
    return fetch('/api/masters/fiscal-years', {
      method:  'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionStorage.getItem('token')}` },
      body:    JSON.stringify(d),
    }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(new Error(e.detail))); return r.json(); });
  },
  async update() { return; }, // fiscal years are not updated, only closed
};

// ─────────────────────────────────────────────────────────────────────────────
// Doc Sequences (read-only display)
// ─────────────────────────────────────────────────────────────────────────────

const DOC_SEQ = {
  columns: [
    { key: 'doc_type',    label: 'Document Type', width: '150px' },
    { key: 'prefix',      label: 'Prefix',        width: '80px' },
    { key: 'last_number', label: 'Last Number',   align: 'right', width: '120px' },
    { key: 'year',        label: 'Year',          width: '70px' },
  ],
  async list() {
    const res = await fetch('/api/masters/doc-sequences', {
      headers: { Authorization: `Bearer ${sessionStorage.getItem('token')}` },
    }).then(r => r.json()).catch(() => []);
    return res || [];
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Helper: row actions HTML
// ─────────────────────────────────────────────────────────────────────────────

function rowActions(row) {
  return `
    <div class="row-actions">
      <button class="btn btn-sm btn-secondary btn-icon" data-action="edit" title="Edit">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg>
      </button>
    </div>
  `;
}

// ─────────────────────────────────────────────────────────────────────────────
// Section config map
// ─────────────────────────────────────────────────────────────────────────────

const SECTION_MAP = {
  'uom':           UOM,
  'currencies':    CURRENCIES,
  'tax':           TAX,
  'payment-terms': PAYMENT_TERMS,
  'locations':     LOCATIONS,
  'fiscal-years':  FISCAL_YEARS,
  'doc-seq':       DOC_SEQ,
};

// ─────────────────────────────────────────────────────────────────────────────
// Load current section table
// ─────────────────────────────────────────────────────────────────────────────

async function loadSection(sec) {
  const cfg = SECTION_MAP[sec];
  if (!cfg) return;
  try {
    const data = await cfg.list();
    _table?.setData(Array.isArray(data) ? data : data.items || []);
  } catch {
    Toast.error(`Failed to load ${sec}.`);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Open modal for current section
// ─────────────────────────────────────────────────────────────────────────────

async function openModal(editData = null) {
  const cfg = SECTION_MAP[_section];
  if (!cfg?.buildForm) return;
  _editId = editData?.id || null;

  if (_modal) _modal.close();
  _modal = new Modal({
    title:   _editId ? `Edit ${SECTIONS.find(s => s.id === _section)?.label}` : `New ${SECTIONS.find(s => s.id === _section)?.label}`,
    size:    'sm',
    content: cfg.buildForm(editData || {}),
  });
  _modal.open();

  _modal.setFooter(`
    <button class="btn btn-secondary" id="ms-cancel">Cancel</button>
    <button class="btn btn-primary" id="ms-save">${_editId ? 'Save Changes' : 'Create'}</button>
  `);
  _modal.getFooter().querySelector('#ms-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#ms-save').addEventListener('click', () => saveCurrent(cfg));
}

// ─────────────────────────────────────────────────────────────────────────────
// Save
// ─────────────────────────────────────────────────────────────────────────────

async function saveCurrent(cfg) {
  if (!cfg?.schema || !cfg?.collect) return;

  let hasError = false;
  for (const [id, rules] of Object.entries(cfg.schema)) {
    const el = document.getElementById(id);
    if (!el) continue;
    const r = Validator.validate(el.value, rules);
    if (!r.valid) { Validator.showError(el, r.message); hasError = true; }
    else Validator.clearError(el);
  }
  if (hasError) return;

  const btn = document.getElementById('ms-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    const data = cfg.collect();
    if (_editId) { await cfg.update(_editId, data); Toast.success('Updated.'); }
    else { await cfg.create(data); Toast.success('Created.'); }
    _modal.close();
    await loadSection(_section);
  } catch (err) {
    Toast.error(err.message || 'Save failed.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Switch section
// ─────────────────────────────────────────────────────────────────────────────

async function switchSection(sec, container) {
  _section = sec;
  _editId  = null;
  if (_modal) { _modal.close(); _modal = null; }

  // Update nav tabs
  container.querySelectorAll('.masters-tab').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.sec === sec);
  });

  // Show/hide add button
  const cfg     = SECTION_MAP[sec];
  const addBtn  = container.querySelector('#masters-add-btn');
  const secLabel = SECTIONS.find(s => s.id === sec)?.label || sec;
  if (addBtn) {
    addBtn.style.display = cfg?.buildForm ? '' : 'none';
    addBtn.textContent   = `+ New ${secLabel.replace(/s$/, '')}`;
  }

  // Re-build DataTable with new columns
  const tableEl = container.querySelector('#masters-table-wrap');
  if (!tableEl) return;
  tableEl.innerHTML = '';

  if (!cfg) return;

  _table = new DataTable({
    el: tableEl,
    columns: cfg.columns || [],
    onRowAction: async (action, row) => {
      if (action === 'edit' && cfg.buildForm) {
        openModal(row);
        return;
      }
      // Fiscal year close
      if (action === 'close') {
        const ok = await Confirm({
          title:       'Close Fiscal Year?',
          message:     `"${row.name}" will be permanently closed. All periods will be locked.`,
          type:        'danger',
          confirmText: 'Close Year',
        });
        if (ok) {
          try {
            await fetch(`/api/masters/fiscal-years/${row.id}/close`, {
              method:  'POST',
              headers: { Authorization: `Bearer ${sessionStorage.getItem('token')}` },
            }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(new Error(e.detail))); return r.json(); });
            Toast.success('Fiscal year closed.');
            await loadSection(_section);
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  await loadSection(sec);
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Masters Setup</h1>
        <p class="page-subtitle">Configure lookup tables: UOM, currencies, taxes, payment terms and more</p>
      </div>
      <button class="btn btn-primary" id="masters-add-btn">+ New</button>
    </div>

    <div class="masters-nav" style="display:flex;gap:0.25rem;flex-wrap:wrap;margin-bottom:1rem;border-bottom:1px solid var(--border);padding-bottom:0.5rem">
      ${SECTIONS.map(s => `
        <button class="btn btn-sm masters-tab ${s.id === _section ? 'btn-primary active' : 'btn-secondary'}" data-sec="${s.id}">${s.label}</button>
      `).join('')}
    </div>

    <div class="card" style="padding:0;overflow:hidden">
      <div id="masters-table-wrap"></div>
    </div>
  `;

  // Tab clicks
  container.querySelectorAll('.masters-tab').forEach(btn => {
    btn.addEventListener('click', () => switchSection(btn.dataset.sec, container));
  });

  // Add button
  container.querySelector('#masters-add-btn').addEventListener('click', () => openModal());

  // Load first section
  await switchSection(_section, container);
}

export default { init };
