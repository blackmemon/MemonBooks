/**
 * grn.js — Goods Receipt Note module
 */

'use strict';

import { PurchasesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, StatusBadge, formatAmount, formatDate, FormGroup, SelectGroup,
} from '../../components/components.js';

let _table   = null;
let _modal   = null;
let _locs    = [];
let _grnLines = [];   // editable qty lines in create modal

async function loadRefData() {
  const [locs] = await Promise.allSettled([MastersApi.locations.list()]);
  _locs = locs.value || [];
}

function locOpts() {
  return `<option value="">Select…</option>` + _locs.map(l => `<option value="${l.id}">${l.name}</option>`).join('');
}

// ─────────────────────────────────────────────────────────────────────────────
// Create GRN from approved PO
// ─────────────────────────────────────────────────────────────────────────────

async function openCreateModal(po) {
  _grnLines = po.lines.map(l => ({
    po_line_id:   l.id,
    item_id:      l.item_id,
    item_code:    l.item_code,
    item_name:    l.item_name,
    uom_code:     l.uom_code,
    qty_ordered:  l.qty,
    qty_remaining: l.qty - l.qty_received,
    qty_received: 0,
    qty_accepted: 0,
    qty_rejected: 0,
    rejection_reason: '',
  }));

  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();
  _modal = new Modal({
    title: `New GRN — PO ${po.po_number}`,
    size:  'xl',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">GRN Date</label>
          <input class="input" type="date" id="grn-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label">Default Location</label>
          <select class="input" id="grn-location">${locOpts()}</select>
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <textarea class="input" id="grn-notes" rows="2" placeholder="Optional"></textarea>
      </div>
      <div style="overflow-x:auto;margin-top:1rem">
        <table class="table line-table" style="min-width:700px">
          <thead>
            <tr>
              <th>Item</th><th style="width:60px">UOM</th>
              <th style="width:80px;text-align:right">Ordered</th>
              <th style="width:80px;text-align:right">Remaining</th>
              <th style="width:90px">Received</th>
              <th style="width:90px">Accepted</th>
              <th style="width:90px">Rejected</th>
              <th>Reject Reason</th>
            </tr>
          </thead>
          <tbody id="grn-lines-body"></tbody>
        </table>
      </div>
    `,
  });
  _modal.open();
  renderGRNLines(po.po_id || po.id);

  _modal.setFooter(`
    <button class="btn btn-secondary" id="grn-cancel">Cancel</button>
    <button class="btn btn-primary" id="grn-save">Create GRN</button>
  `);
  _modal.getFooter().querySelector('#grn-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#grn-save').addEventListener('click', () => saveGRN(po.id || po.po_id));
}

function renderGRNLines(poId) {
  const tbody = document.getElementById('grn-lines-body');
  if (!tbody) return;
  tbody.innerHTML = _grnLines.map((ln, i) => `
    <tr data-idx="${i}">
      <td>${ln.item_code} — ${ln.item_name}</td>
      <td>${ln.uom_code||'—'}</td>
      <td style="text-align:right">${ln.qty_ordered}</td>
      <td style="text-align:right">${ln.qty_remaining.toFixed(2)}</td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" max="${ln.qty_remaining}" value="${ln.qty_received}" data-field="qty_received" data-idx="${i}"></td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" value="${ln.qty_accepted}" data-field="qty_accepted" data-idx="${i}"></td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" value="${ln.qty_rejected}" data-field="qty_rejected" data-idx="${i}"></td>
      <td><input class="input input-sm" type="text" value="${ln.rejection_reason}" data-field="rejection_reason" data-idx="${i}" placeholder="Optional"></td>
    </tr>
  `).join('');

  tbody.querySelectorAll('[data-field]').forEach(inp => {
    inp.addEventListener('change', () => {
      const idx   = parseInt(inp.dataset.idx);
      const field = inp.dataset.field;
      const num   = ['qty_received','qty_accepted','qty_rejected'];
      _grnLines[idx][field] = num.includes(field) ? parseFloat(inp.value)||0 : inp.value;

      // Auto-fill accepted = received when rejected not set
      if (field === 'qty_received') {
        const rej = _grnLines[idx].qty_rejected;
        _grnLines[idx].qty_accepted = Math.max(0, _grnLines[idx].qty_received - rej);
        renderGRNLines(poId);
      }
    });
  });
}

async function saveGRN(poId) {
  const grnDate = document.getElementById('grn-date')?.value;
  const locId   = document.getElementById('grn-location')?.value || null;
  const notes   = document.getElementById('grn-notes')?.value?.trim() || null;

  if (!grnDate) { Toast.warning('GRN date is required.'); return; }

  const lines = _grnLines.filter(l => l.qty_received > 0).map(l => ({
    po_line_id:       l.po_line_id,
    item_id:          l.item_id,
    qty_received:     l.qty_received,
    qty_accepted:     l.qty_accepted,
    qty_rejected:     l.qty_rejected,
    rejection_reason: l.rejection_reason || undefined,
    location_id:      locId || undefined,
  }));

  if (lines.length === 0) { Toast.warning('Enter received quantities for at least one line.'); return; }

  // Validate accepted + rejected = received
  const invalid = lines.find(l => Math.abs(l.qty_accepted + l.qty_rejected - l.qty_received) > 0.001);
  if (invalid) { Toast.error('Accepted + Rejected must equal Received for all lines.'); return; }

  const btn = document.getElementById('grn-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    const res = await PurchasesApi.grn.create({ po_id: poId, grn_date: grnDate, location_id: locId, notes, lines });
    Toast.success(`GRN created: ${res.number}`);
    _modal.close();
    await loadTable();
  } catch (err) {
    Toast.error(err.message || 'Failed to create GRN.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// View detail modal
// ─────────────────────────────────────────────────────────────────────────────

function showDetailModal(data) {
  const m = new Modal({
    title: `GRN ${data.grn_number} — ${data.vendor_name}`,
    size:  'xl',
    content: `
      <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">PO #</span><br>${data.po_number||'—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">GRN Date</span><br>${formatDate(data.grn_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
      </div>
      <table class="table" style="font-size:0.85rem">
        <thead>
          <tr><th>#</th><th>Item</th><th>UOM</th>
            <th style="text-align:right">Ordered</th>
            <th style="text-align:right">Received</th>
            <th style="text-align:right">Accepted</th>
            <th style="text-align:right">Rejected</th>
            <th>Reject Reason</th>
          </tr>
        </thead>
        <tbody>
          ${(data.lines||[]).map((l,i) => `
            <tr>
              <td>${i+1}</td>
              <td>${l.item_code} — ${l.item_name}</td>
              <td>${l.uom_code||'—'}</td>
              <td style="text-align:right">${l.qty_ordered||0}</td>
              <td style="text-align:right">${l.qty_received||0}</td>
              <td style="text-align:right;color:var(--success)">${l.qty_accepted||0}</td>
              <td style="text-align:right;color:var(--danger)">${l.qty_rejected||0}</td>
              <td style="font-size:0.8rem;color:var(--text-muted)">${l.rejection_reason||'—'}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
      ${data.notes ? `<p style="margin-top:1rem;font-size:0.85rem;color:var(--text-muted)"><strong>Notes:</strong> ${data.notes}</p>` : ''}
    `,
  });
  m.open();

  const footer = `
    <button class="btn btn-secondary" id="grn-detail-close">Close</button>
    ${data.status === 'draft' ? `<button class="btn btn-success" id="grn-detail-accept">Accept & Update Stock</button>` : ''}
  `;
  m.setFooter(footer);
  m.getFooter().querySelector('#grn-detail-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#grn-detail-accept')?.addEventListener('click', async () => {
    const ok = await Confirm({
      title:       'Accept GRN?',
      message:     'This will update stock levels for all accepted quantities. Cannot be undone.',
      type:        'success',
      confirmText: 'Accept & Update Stock',
    });
    if (ok) {
      try {
        await PurchasesApi.grn.accept(data.id);
        Toast.success('GRN accepted. Stock updated.');
        m.close();
        await loadTable();
      } catch(e) { Toast.error(e.message); }
    }
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// PO selector modal (pick approved PO to create GRN against)
// ─────────────────────────────────────────────────────────────────────────────

async function openPOSelector() {
  try {
    const res = await PurchasesApi.po.list({ status: 'approved', limit: 100 });
    const pos  = res.items || [];
    if (pos.length === 0) {
      Toast.info('No approved POs available to receive.');
      return;
    }
    const sel = new Modal({
      title:   'Select Purchase Order',
      size:    'md',
      content: `
        <p style="font-size:0.85rem;color:var(--text-muted);margin-bottom:0.75rem">Select an approved PO to create a Goods Receipt Note against:</p>
        <table class="table" style="font-size:0.85rem">
          <thead><tr><th>PO #</th><th>Vendor</th><th>Date</th><th style="text-align:right">Total</th><th></th></tr></thead>
          <tbody>
            ${pos.map(p => `
              <tr>
                <td>${p.po_number}</td>
                <td>${p.vendor_name}</td>
                <td>${formatDate(p.po_date)}</td>
                <td style="text-align:right">${formatAmount(p.grand_total,'PKR')}</td>
                <td><button class="btn btn-sm btn-primary" data-po-id="${p.id}" data-po-num="${p.po_number}">Select</button></td>
              </tr>
            `).join('')}
          </tbody>
        </table>
      `,
    });
    sel.open();
    sel.setFooter(`<button class="btn btn-secondary" id="po-sel-cancel">Cancel</button>`);
    sel.getFooter().querySelector('#po-sel-cancel').addEventListener('click', () => sel.close());

    sel.getBody().querySelectorAll('[data-po-id]').forEach(btn => {
      btn.addEventListener('click', async () => {
        sel.close();
        try {
          const po = await PurchasesApi.po.get(btn.dataset.poId);
          po.id = btn.dataset.poId;
          openCreateModal(po);
        } catch { Toast.error('Failed to load PO.'); }
      });
    });
  } catch {
    Toast.error('Failed to load POs.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Load table
// ─────────────────────────────────────────────────────────────────────────────

async function loadTable() {
  try {
    const res = await PurchasesApi.grn.list({ limit: 200 });
    _table?.setData(res.items || []);
  } catch {
    Toast.error('Failed to load GRNs.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Goods Receipt Notes</h1>
        <p class="page-subtitle">Record goods received against purchase orders</p>
      </div>
      <button class="btn btn-primary" id="add-grn-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New GRN
      </button>
    </div>
    <div id="grn-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#grn-table',
    columns: [
      { key: 'grn_number',  label: 'GRN #',   width: '120px' },
      { key: 'po_number',   label: 'PO #',    width: '110px', render: v => v || '—' },
      { key: 'vendor_name', label: 'Vendor' },
      { key: 'grn_date',    label: 'Date',    width: '100px', render: formatDate },
      { key: 'status',      label: 'Status',  width: '100px', render: v => StatusBadge(v) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            ${row.status === 'draft' ? `
              <button class="btn btn-sm btn-success" data-action="accept" style="font-size:0.78rem;padding:3px 10px">Accept</button>
              <button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>
            ` : ''}
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view') {
        try { const d = await PurchasesApi.grn.get(row.id); showDetailModal(d); }
        catch { Toast.error('Failed to load GRN.'); }
        return;
      }
      if (action === 'accept') {
        const ok = await Confirm({ title:'Accept GRN?', message:'This will update stock. Cannot be undone.', type:'success', confirmText:'Accept' });
        if (ok) { try { await PurchasesApi.grn.accept(row.id); Toast.success('GRN accepted.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
        return;
      }
      if (action === 'cancel') {
        const ok = await Confirm({ title:'Cancel GRN?', message:`Cancel GRN "${row.grn_number}"?`, type:'danger', confirmText:'Cancel' });
        if (ok) { try { await PurchasesApi.grn.cancel(row.id); Toast.success('GRN cancelled.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
      }
    },
  });

  document.getElementById('add-grn-btn').addEventListener('click', openPOSelector);
  await loadTable();
}

export default { init };
