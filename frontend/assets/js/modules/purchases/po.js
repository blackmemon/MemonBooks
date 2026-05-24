/**
 * po.js — Purchase Orders module (CRUD + approval flow)
 */

'use strict';

import { PurchasesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, StatusBadge, Validator, formatAmount, formatDate,
  FormGroup, SelectGroup,
} from '../../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────────────────

let _table    = null;
let _modal    = null;
let _editId   = null;
let _lines    = [];        // editable PO lines
let _vendors  = [];
let _items    = [];
let _uoms     = [];
let _taxes    = [];
let _currs    = [];
let _terms    = [];
let _locs     = [];

// ─────────────────────────────────────────────────────────────────────────────
// Load reference data
// ─────────────────────────────────────────────────────────────────────────────

async function loadRefData() {
  const [vendors, items, uoms, taxes, currs, terms, locs] = await Promise.allSettled([
    MastersApi.vendors.list({ active_only: true, limit: 500 }),
    MastersApi.items.dropdown(),
    MastersApi.uom.list(),
    fetch('/api/masters/tax?applies_to=purchases').then(r => r.json()).catch(() => []),
    MastersApi.currencies.list(),
    MastersApi.paymentTerms.list(),
    MastersApi.locations.list(),
  ]);
  _vendors = vendors.value?.items || vendors.value || [];
  _items   = items.value  || [];
  _uoms    = uoms.value   || [];
  _taxes   = taxes.value  || [];
  _currs   = currs.value  || [];
  _terms   = terms.value  || [];
  _locs    = locs.value   || [];
}

// ─────────────────────────────────────────────────────────────────────────────
// Build line options
// ─────────────────────────────────────────────────────────────────────────────

function vendorOpts()   { return _vendors.map(v => `<option value="${v.id}">${v.code} — ${v.name}</option>`).join(''); }
function itemOpts()     { return _items.map(i   => `<option value="${i.id}" data-price="${i.standard_cost||0}">${i.code} — ${i.name}</option>`).join(''); }
function uomOpts()      { return `<option value="">—</option>` + _uoms.map(u => `<option value="${u.id}">${u.code}</option>`).join(''); }
function taxOpts()      { return `<option value="">No Tax</option>` + _taxes.map(t => `<option value="${t.id}" data-rate="${t.rate}">${t.name}</option>`).join(''); }
function currOpts()     { return `<option value="">Default</option>` + _currs.map(c => `<option value="${c.id}">${c.code}</option>`).join(''); }
function termOpts()     { return `<option value="">None</option>` + _terms.map(t => `<option value="${t.id}">${t.name}</option>`).join(''); }
function locOpts()      { return `<option value="">Default</option>` + _locs.map(l => `<option value="${l.id}">${l.name}</option>`).join(''); }

// ─────────────────────────────────────────────────────────────────────────────
// Header form
// ─────────────────────────────────────────────────────────────────────────────

function buildHeader(data = {}) {
  const today = new Date().toISOString().split('T')[0];
  return `
    <div class="grid-2">
      <div class="form-group">
        <label class="form-label required">Vendor</label>
        <select class="input" id="po-vendor" ${_editId ? 'disabled' : ''}>
          <option value="">Select vendor…</option>
          ${vendorOpts()}
        </select>
        <div class="form-error" id="po-vendor-err"></div>
      </div>
      <div class="form-group">
        <label class="form-label required">PO Date</label>
        <input class="input" type="date" id="po-date" value="${data.po_date || today}">
        <div class="form-error" id="po-date-err"></div>
      </div>
      <div class="form-group">
        <label class="form-label">Expected Date</label>
        <input class="input" type="date" id="po-expected" value="${data.expected_date || ''}">
      </div>
      <div class="form-group">
        <label class="form-label">Currency</label>
        <select class="input" id="po-currency"><option value="">PKR (default)</option>${currOpts()}</select>
      </div>
      <div class="form-group">
        <label class="form-label">Payment Terms</label>
        <select class="input" id="po-terms">${termOpts()}</select>
      </div>
      <div class="form-group">
        <label class="form-label">Default Location</label>
        <select class="input" id="po-location">${locOpts()}</select>
      </div>
    </div>
    <div class="form-group">
      <label class="form-label">Notes</label>
      <textarea class="input" id="po-notes" rows="2" placeholder="Optional">${data.notes || ''}</textarea>
    </div>
  `;
}

// ─────────────────────────────────────────────────────────────────────────────
// Lines table (inline editing)
// ─────────────────────────────────────────────────────────────────────────────

function renderLines() {
  const tbody = document.getElementById('po-lines-body');
  if (!tbody) return;

  if (_lines.length === 0) {
    tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;color:var(--text-muted);padding:1rem">No lines — click "Add Line" to start</td></tr>`;
    updateTotals();
    return;
  }

  tbody.innerHTML = _lines.map((ln, i) => `
    <tr data-idx="${i}">
      <td style="min-width:180px">
        <select class="input input-sm" data-field="item_id" data-idx="${i}">
          <option value="">Select item…</option>
          ${_items.map(it => `<option value="${it.id}" data-price="${it.standard_cost||0}" ${it.id===ln.item_id?'selected':''}>${it.code} — ${it.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:70px">
        <select class="input input-sm" data-field="uom_id" data-idx="${i}">
          ${_uoms.map(u => `<option value="${u.id}" ${u.id===ln.uom_id?'selected':''}>${u.code}</option>`).join('')}
        </select>
      </td>
      <td style="width:85px">
        <input class="input input-sm" type="number" min="0.01" step="0.01" value="${ln.qty||1}" data-field="qty" data-idx="${i}">
      </td>
      <td style="width:100px">
        <input class="input input-sm" type="number" min="0" step="0.01" value="${ln.unit_price||0}" data-field="unit_price" data-idx="${i}">
      </td>
      <td style="width:70px">
        <input class="input input-sm" type="number" min="0" max="100" step="0.01" value="${ln.discount_pct||0}" data-field="discount_pct" data-idx="${i}">
      </td>
      <td style="width:110px">
        <select class="input input-sm" data-field="tax_id" data-idx="${i}">
          <option value="">No Tax</option>
          ${_taxes.map(t => `<option value="${t.id}" data-rate="${t.rate}" ${t.id===ln.tax_id?'selected':''}>${t.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:100px;text-align:right;padding-right:8px">
        <span class="line-total" data-idx="${i}">${formatAmount(calcLineTotal(ln), 'PKR')}</span>
      </td>
      <td style="width:36px">
        <button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </td>
    </tr>
  `).join('');

  // Events
  tbody.querySelectorAll('[data-field]').forEach(el => {
    el.addEventListener('change', () => {
      const idx   = parseInt(el.dataset.idx);
      const field = el.dataset.field;
      if (field === 'item_id') {
        const opt = el.selectedOptions[0];
        _lines[idx].item_id    = el.value;
        _lines[idx].unit_price = parseFloat(opt?.dataset.price || 0);
        renderLines();
        return;
      }
      const v = (field === 'qty' || field === 'unit_price' || field === 'discount_pct')
        ? parseFloat(el.value) || 0
        : el.value;
      _lines[idx][field] = v;
      // Update line total display
      const span = document.querySelector(`.line-total[data-idx="${idx}"]`);
      if (span) span.textContent = formatAmount(calcLineTotal(_lines[idx]), 'PKR');
      updateTotals();
    });
  });

  tbody.querySelectorAll('[data-remove]').forEach(btn => {
    btn.addEventListener('click', () => {
      _lines.splice(parseInt(btn.dataset.remove), 1);
      renderLines();
    });
  });

  updateTotals();
}

function calcLineTotal(ln) {
  const gross  = (ln.qty || 0) * (ln.unit_price || 0);
  const disc   = gross * (ln.discount_pct || 0) / 100;
  const taxOpt = _taxes.find(t => t.id === ln.tax_id);
  const taxRate = taxOpt ? taxOpt.rate : 0;
  const net    = gross - disc;
  return net + (net * taxRate / 100);
}

function updateTotals() {
  const subtotal = _lines.reduce((s, ln) => s + (ln.qty||0)*(ln.unit_price||0), 0);
  const discount = _lines.reduce((s, ln) => s + (ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100, 0);
  const tax      = _lines.reduce((s, ln) => {
    const net = (ln.qty||0)*(ln.unit_price||0) - (ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100;
    const r = _taxes.find(t => t.id === ln.tax_id);
    return s + (r ? net * r.rate / 100 : 0);
  }, 0);
  const grand = subtotal - discount + tax;

  const set = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = formatAmount(v, 'PKR'); };
  set('po-subtotal', subtotal);
  set('po-discount', discount);
  set('po-tax', tax);
  set('po-grand', grand);
}

// ─────────────────────────────────────────────────────────────────────────────
// Open modal
// ─────────────────────────────────────────────────────────────────────────────

async function openModal(editData = null) {
  _editId = editData?.id || null;
  _lines  = editData?.lines?.map(l => ({
    item_id:      l.item_id,
    uom_id:       l.uom_id,
    qty:          l.qty,
    unit_price:   l.unit_price,
    discount_pct: l.discount_pct || 0,
    tax_id:       l.tax_id || '',
  })) || [];

  if (_modal) _modal.close();
  _modal = new Modal({
    title:   _editId ? `Edit PO — ${editData.po_number}` : 'New Purchase Order',
    size:    'xl',
    content: `
      <div class="tabs">
        <button class="tab-btn active" data-tab="header">Header</button>
        <button class="tab-btn" data-tab="lines">Lines</button>
      </div>
      <div class="tab-panel active" data-tab="header" style="padding-top:1rem" id="po-header-panel">
        ${buildHeader(editData || {})}
      </div>
      <div class="tab-panel" data-tab="lines" style="padding-top:1rem">
        <div style="display:flex;justify-content:flex-end;margin-bottom:0.75rem">
          <button class="btn btn-sm btn-secondary" id="po-add-line">+ Add Line</button>
        </div>
        <div style="overflow-x:auto">
          <table class="table line-table" style="min-width:750px">
            <thead>
              <tr>
                <th>Item</th><th>UOM</th><th>Qty</th><th>Unit Price</th>
                <th>Disc%</th><th>Tax</th><th style="text-align:right">Total</th><th></th>
              </tr>
            </thead>
            <tbody id="po-lines-body"></tbody>
          </table>
        </div>
        <div style="display:flex;justify-content:flex-end;margin-top:1rem">
          <table style="font-size:0.88rem;min-width:220px">
            <tr><td style="padding:3px 12px 3px 0;color:var(--text-muted)">Subtotal</td><td style="text-align:right" id="po-subtotal">PKR 0.00</td></tr>
            <tr><td style="padding:3px 12px 3px 0;color:var(--text-muted)">Discount</td><td style="text-align:right" id="po-discount">PKR 0.00</td></tr>
            <tr><td style="padding:3px 12px 3px 0;color:var(--text-muted)">Tax</td><td style="text-align:right" id="po-tax">PKR 0.00</td></tr>
            <tr style="border-top:2px solid var(--border);font-weight:700">
              <td style="padding-top:6px">Grand Total</td>
              <td style="text-align:right;padding-top:6px" id="po-grand">PKR 0.00</td>
            </tr>
          </table>
        </div>
      </div>
    `,
  });
  _modal.open();

  // Populate vendor select for edit
  if (_editId && editData?.vendor_id) {
    setTimeout(() => {
      const sel = document.getElementById('po-vendor');
      if (sel) sel.value = editData.vendor_id;
    }, 50);
  }

  // Tabs
  _modal.getBody().querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      _modal.getBody().querySelectorAll('.tab-btn,.tab-panel').forEach(el => el.classList.remove('active'));
      btn.classList.add('active');
      _modal.getBody().querySelector(`.tab-panel[data-tab="${btn.dataset.tab}"]`)?.classList.add('active');
    });
  });

  renderLines();
  document.getElementById('po-add-line')?.addEventListener('click', () => {
    _lines.push({ item_id: '', uom_id: _uoms[0]?.id || '', qty: 1, unit_price: 0, discount_pct: 0, tax_id: '' });
    renderLines();
  });

  _modal.setFooter(`
    <button class="btn btn-secondary" id="po-cancel">Cancel</button>
    <button class="btn btn-primary" id="po-save">${_editId ? 'Save Changes' : 'Create PO'}</button>
  `);
  _modal.getFooter().querySelector('#po-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#po-save').addEventListener('click', savePO);
}

// ─────────────────────────────────────────────────────────────────────────────
// Save
// ─────────────────────────────────────────────────────────────────────────────

async function savePO() {
  const g = id => document.getElementById(id)?.value?.trim() || null;

  if (!g('po-vendor')) { Toast.warning('Select a vendor.'); return; }
  if (!g('po-date'))   { Toast.warning('PO date required.'); return; }
  if (_lines.length === 0) { Toast.warning('Add at least one line.'); return; }
  const emptyItem = _lines.find(l => !l.item_id);
  if (emptyItem) { Toast.warning('All lines must have an item selected.'); return; }

  const payload = {
    vendor_id:       g('po-vendor'),
    po_date:         g('po-date'),
    expected_date:   g('po-expected') || undefined,
    currency_id:     g('po-currency') || undefined,
    payment_term_id: g('po-terms')    || undefined,
    location_id:     g('po-location') || undefined,
    notes:           g('po-notes')    || undefined,
    lines: _lines.map(l => ({
      item_id:      l.item_id,
      uom_id:       l.uom_id || undefined,
      qty:          l.qty,
      unit_price:   l.unit_price,
      discount_pct: l.discount_pct || 0,
      tax_id:       l.tax_id || undefined,
    })),
  };

  const btn = document.getElementById('po-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    if (_editId) {
      await PurchasesApi.po.update(_editId, payload);
      Toast.success('PO updated.');
    } else {
      const res = await PurchasesApi.po.create(payload);
      Toast.success(`PO created: ${res.number}`);
    }
    _modal.close();
    await loadTable();
  } catch (err) {
    Toast.error(err.message || 'Save failed.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Status action buttons
// ─────────────────────────────────────────────────────────────────────────────

function actionButtons(row) {
  const btns = [];
  if (row.status === 'draft') {
    btns.push(`<button class="btn btn-sm btn-secondary btn-icon" data-action="edit" title="Edit"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg></button>`);
    btns.push(`<button class="btn btn-sm btn-primary" data-action="submit" style="font-size:0.78rem;padding:3px 10px">Submit</button>`);
    btns.push(`<button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete" title="Delete"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/></svg></button>`);
  }
  if (row.status === 'submitted') {
    btns.push(`<button class="btn btn-sm btn-success" data-action="approve" style="font-size:0.78rem;padding:3px 10px">Approve</button>`);
    btns.push(`<button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>`);
  }
  if (row.status === 'approved' || row.status === 'partial') {
    btns.push(`<button class="btn btn-sm btn-info" data-action="view" style="font-size:0.78rem;padding:3px 10px">View</button>`);
  }
  if (!btns.length) {
    btns.push(`<button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg></button>`);
  }
  return `<div class="row-actions">${btns.join('')}</div>`;
}

// ─────────────────────────────────────────────────────────────────────────────
// Load table
// ─────────────────────────────────────────────────────────────────────────────

async function loadTable() {
  try {
    const res = await PurchasesApi.po.list({ limit: 200 });
    _table?.setData(res.items || []);
  } catch {
    Toast.error('Failed to load purchase orders.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Purchase Orders</h1>
        <p class="page-subtitle">Create and manage purchase orders sent to vendors</p>
      </div>
      <button class="btn btn-primary" id="add-po-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New PO
      </button>
    </div>
    <div id="po-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#po-table',
    columns: [
      { key: 'po_number',   label: 'PO #',      width: '120px' },
      { key: 'vendor_name', label: 'Vendor' },
      { key: 'po_date',     label: 'Date',       width: '100px', render: formatDate },
      { key: 'expected_date', label: 'Expected', width: '100px', render: v => v ? formatDate(v) : '—' },
      { key: 'grand_total', label: 'Total',      align: 'right', render: v => formatAmount(v, 'PKR') },
      { key: 'status',      label: 'Status',     width: '100px', render: v => StatusBadge(v) },
      { key: '_actions',    label: '',           sortable: false, render: (_, row) => actionButtons(row) },
    ],
    onRowAction: async (action, row) => {
      if (action === 'edit' || action === 'view') {
        try {
          const detail = await PurchasesApi.po.get(row.id);
          if (action === 'edit') openModal(detail);
          else showDetailModal(detail);
        } catch { Toast.error('Failed to load PO.'); }
        return;
      }
      if (action === 'submit') {
        try { await PurchasesApi.po.submit(row.id); Toast.success('PO submitted for approval.'); await loadTable(); }
        catch (e) { Toast.error(e.message); }
        return;
      }
      if (action === 'approve') {
        const ok = await Confirm({ title:'Approve PO?', message:`Approve "${row.po_number}"?`, type:'success', confirmText:'Approve' });
        if (ok) { try { await PurchasesApi.po.approve(row.id); Toast.success('PO approved.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
        return;
      }
      if (action === 'cancel') {
        const ok = await Confirm({ title:'Cancel PO?', message:`Cancel "${row.po_number}"?`, type:'danger', confirmText:'Cancel PO' });
        if (ok) { try { await PurchasesApi.po.cancel(row.id); Toast.success('PO cancelled.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
        return;
      }
      if (action === 'delete') {
        const ok = await Confirm({ title:'Delete PO?', message:`Delete "${row.po_number}"? This cannot be undone.`, type:'danger', confirmText:'Delete' });
        if (ok) { try { await PurchasesApi.po.delete(row.id); Toast.success('PO deleted.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
      }
    },
  });

  document.getElementById('add-po-btn').addEventListener('click', () => openModal());
  await loadTable();
}

// ─────────────────────────────────────────────────────────────────────────────
// View-only detail modal
// ─────────────────────────────────────────────────────────────────────────────

function showDetailModal(data) {
  const m = new Modal({
    title: `PO ${data.po_number} — ${data.vendor_name}`,
    size:  'xl',
    content: `
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:1rem;margin-bottom:1.5rem">
        <div><span style="color:var(--text-muted);font-size:0.82rem">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="color:var(--text-muted);font-size:0.82rem">Date</span><br>${formatDate(data.po_date)}</div>
        <div><span style="color:var(--text-muted);font-size:0.82rem">Expected</span><br>${data.expected_date ? formatDate(data.expected_date) : '—'}</div>
        <div><span style="color:var(--text-muted);font-size:0.82rem">Grand Total</span><br><strong>${formatAmount(data.grand_total, 'PKR')}</strong></div>
      </div>
      <table class="table" style="font-size:0.85rem">
        <thead><tr><th>#</th><th>Item</th><th>UOM</th><th style="text-align:right">Qty</th><th style="text-align:right">Price</th><th style="text-align:right">Disc%</th><th style="text-align:right">Total</th><th style="text-align:right">Rcvd</th></tr></thead>
        <tbody>
          ${(data.lines||[]).map((l, i) => `
            <tr>
              <td>${i+1}</td>
              <td>${l.item_code} — ${l.item_name}</td>
              <td>${l.uom_code||'—'}</td>
              <td style="text-align:right">${l.qty}</td>
              <td style="text-align:right">${formatAmount(l.unit_price,'PKR')}</td>
              <td style="text-align:right">${l.discount_pct||0}%</td>
              <td style="text-align:right">${formatAmount(l.line_total,'PKR')}</td>
              <td style="text-align:right">${l.qty_received||0}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
      <div style="display:flex;justify-content:flex-end;margin-top:1rem;font-size:0.9rem">
        <table>
          <tr><td style="padding:3px 16px 3px 0;color:var(--text-muted)">Subtotal</td><td>${formatAmount(data.subtotal,'PKR')}</td></tr>
          <tr><td style="color:var(--text-muted)">Discount</td><td>${formatAmount(data.discount_total,'PKR')}</td></tr>
          <tr><td style="color:var(--text-muted)">Tax</td><td>${formatAmount(data.tax_total,'PKR')}</td></tr>
          <tr style="font-weight:700;border-top:1px solid var(--border)"><td style="padding-top:6px">Grand Total</td><td>${formatAmount(data.grand_total,'PKR')}</td></tr>
        </table>
      </div>
      ${data.notes ? `<p style="margin-top:1rem;font-size:0.85rem;color:var(--text-muted)"><strong>Notes:</strong> ${data.notes}</p>` : ''}
    `,
  });
  m.open();
  m.setFooter(`<button class="btn btn-secondary" id="po-detail-close">Close</button>`);
  m.getFooter().querySelector('#po-detail-close').addEventListener('click', () => m.close());
}

export default { init };
