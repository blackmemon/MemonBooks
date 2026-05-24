/**
 * pi.js — Purchase Invoices module (create, view, post)
 */

'use strict';

import { PurchasesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, StatusBadge, formatAmount, formatDate, FormGroup, SelectGroup,
} from '../../components/components.js';

let _table   = null;
let _modal   = null;
let _vendors = [];
let _items   = [];
let _uoms    = [];
let _taxes   = [];
let _terms   = [];
let _currs   = [];
let _lines   = [];

async function loadRefData() {
  const [vendors, items, uoms, taxes, terms, currs] = await Promise.allSettled([
    MastersApi.vendors.list({ active_only: true, limit: 500 }),
    MastersApi.items.dropdown(),
    MastersApi.uom.list(),
    fetch('/api/masters/tax?applies_to=purchases').then(r => r.json()).catch(() => []),
    MastersApi.paymentTerms.list(),
    MastersApi.currencies.list(),
  ]);
  _vendors = vendors.value?.items || vendors.value || [];
  _items   = items.value   || [];
  _uoms    = uoms.value    || [];
  _taxes   = taxes.value   || [];
  _terms   = terms.value   || [];
  _currs   = currs.value   || [];
}

// ─────────────────────────────────────────────────────────────────────────────
// Render PI lines inline table
// ─────────────────────────────────────────────────────────────────────────────

function calcLine(ln) {
  const gross  = (ln.qty||0) * (ln.unit_price||0);
  const disc   = gross * (ln.discount_pct||0) / 100;
  const net    = gross - disc;
  const taxR   = _taxes.find(t => t.id === ln.tax_id);
  const whtR   = ln.wht_rate || 0;
  const taxAmt = net * (taxR ? taxR.rate : 0) / 100;
  const whtAmt = net * whtR / 100;
  return { net, taxAmt, whtAmt, total: net + taxAmt };
}

function renderLines() {
  const tbody = document.getElementById('pi-lines-body');
  if (!tbody) return;
  if (_lines.length === 0) {
    tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;color:var(--text-muted);padding:1rem">Add lines below</td></tr>`;
    updatePITotals();
    return;
  }
  tbody.innerHTML = _lines.map((ln, i) => `
    <tr>
      <td style="min-width:160px">
        <select class="input input-sm" data-field="item_id" data-idx="${i}">
          <option value="">Select…</option>
          ${_items.map(it => `<option value="${it.id}" ${it.id===ln.item_id?'selected':''}>${it.code} — ${it.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:75px"><input class="input input-sm" type="number" min="0.01" step="0.01" value="${ln.qty||1}" data-field="qty" data-idx="${i}"></td>
      <td style="width:100px"><input class="input input-sm" type="number" min="0" step="0.01" value="${ln.unit_price||0}" data-field="unit_price" data-idx="${i}"></td>
      <td style="width:65px"><input class="input input-sm" type="number" min="0" max="100" step="0.01" value="${ln.discount_pct||0}" data-field="discount_pct" data-idx="${i}"></td>
      <td style="width:110px">
        <select class="input input-sm" data-field="tax_id" data-idx="${i}">
          <option value="">No Tax</option>
          ${_taxes.map(t => `<option value="${t.id}" ${t.id===ln.tax_id?'selected':''}>${t.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:70px"><input class="input input-sm" type="number" min="0" max="100" step="0.01" value="${ln.wht_rate||0}" data-field="wht_rate" data-idx="${i}" placeholder="0"></td>
      <td style="width:100px;text-align:right;font-size:0.82rem">${formatAmount(calcLine(ln).total,'PKR')}</td>
      <td style="width:36px">
        <button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </td>
    </tr>
  `).join('');

  tbody.querySelectorAll('[data-field]').forEach(el => {
    el.addEventListener('change', () => {
      const idx = parseInt(el.dataset.idx);
      const f   = el.dataset.field;
      _lines[idx][f] = ['qty','unit_price','discount_pct','wht_rate'].includes(f) ? parseFloat(el.value)||0 : el.value;
      renderLines();
    });
  });
  tbody.querySelectorAll('[data-remove]').forEach(btn => {
    btn.addEventListener('click', () => { _lines.splice(parseInt(btn.dataset.remove),1); renderLines(); });
  });
  updatePITotals();
}

function updatePITotals() {
  let subtotal = 0, disc = 0, tax = 0, wht = 0;
  for (const ln of _lines) {
    const c = calcLine(ln);
    subtotal += (ln.qty||0)*(ln.unit_price||0);
    disc     += (ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100;
    tax      += c.taxAmt;
    wht      += c.whtAmt;
  }
  const grand = subtotal - disc + tax;
  const due   = grand - wht;
  const s = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = formatAmount(v,'PKR'); };
  s('pi-subtotal', subtotal); s('pi-disc', disc); s('pi-tax', tax); s('pi-wht', wht);
  s('pi-grand', grand); s('pi-due', due);
}

// ─────────────────────────────────────────────────────────────────────────────
// Open modal (create only)
// ─────────────────────────────────────────────────────────────────────────────

async function openModal() {
  _lines = [];
  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();
  _modal = new Modal({
    title: 'New Purchase Invoice',
    size:  'xl',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">Vendor</label>
          <select class="input" id="pi-vendor">
            <option value="">Select vendor…</option>
            ${_vendors.map(v => `<option value="${v.id}" data-wht="${v.wht_rate||0}">${v.code} — ${v.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label required">Invoice Date</label>
          <input class="input" type="date" id="pi-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label">Due Date</label>
          <input class="input" type="date" id="pi-due-date">
        </div>
        <div class="form-group">
          <label class="form-label">Vendor Invoice #</label>
          <input class="input" type="text" id="pi-vendor-ref" placeholder="Vendor's reference number">
        </div>
        <div class="form-group">
          <label class="form-label">Payment Terms</label>
          <select class="input" id="pi-terms">
            <option value="">None</option>
            ${_terms.map(t => `<option value="${t.id}">${t.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Currency</label>
          <select class="input" id="pi-currency">
            <option value="">PKR (default)</option>
            ${_currs.map(c => `<option value="${c.id}">${c.code}</option>`).join('')}
          </select>
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <textarea class="input" id="pi-notes" rows="2" placeholder="Optional"></textarea>
      </div>
      <div style="display:flex;justify-content:space-between;align-items:center;margin:1rem 0 0.5rem">
        <strong style="font-size:0.9rem">Invoice Lines</strong>
        <button class="btn btn-sm btn-secondary" id="pi-add-line">+ Add Line</button>
      </div>
      <div style="overflow-x:auto">
        <table class="table line-table" style="min-width:750px">
          <thead>
            <tr>
              <th>Item</th><th style="width:75px">Qty</th><th style="width:100px">Unit Price</th>
              <th style="width:65px">Disc%</th><th style="width:110px">Tax</th>
              <th style="width:70px">WHT%</th><th style="width:100px;text-align:right">Total</th><th></th>
            </tr>
          </thead>
          <tbody id="pi-lines-body"></tbody>
        </table>
      </div>
      <div style="display:flex;justify-content:flex-end;margin-top:1rem">
        <table style="font-size:0.88rem;min-width:230px">
          <tr><td style="padding:3px 16px 3px 0;color:var(--text-muted)">Subtotal</td><td style="text-align:right" id="pi-subtotal">PKR 0.00</td></tr>
          <tr><td style="color:var(--text-muted)">Discount</td><td style="text-align:right" id="pi-disc">PKR 0.00</td></tr>
          <tr><td style="color:var(--text-muted)">Tax</td><td style="text-align:right" id="pi-tax">PKR 0.00</td></tr>
          <tr><td style="color:var(--text-muted)">WHT</td><td style="text-align:right" id="pi-wht">PKR 0.00</td></tr>
          <tr style="border-top:1px solid var(--border);font-weight:600"><td style="padding-top:6px">Grand Total</td><td style="text-align:right;padding-top:6px" id="pi-grand">PKR 0.00</td></tr>
          <tr style="font-weight:700;color:var(--primary)"><td>Amount Due</td><td style="text-align:right" id="pi-due">PKR 0.00</td></tr>
        </table>
      </div>
    `,
  });
  _modal.open();

  // Auto-fill WHT from vendor
  document.getElementById('pi-vendor')?.addEventListener('change', () => {
    const opt = document.getElementById('pi-vendor').selectedOptions[0];
    const vendorWht = parseFloat(opt?.dataset.wht || 0);
    _lines.forEach(l => { if (!l.wht_rate) l.wht_rate = vendorWht; });
    renderLines();
  });

  document.getElementById('pi-add-line')?.addEventListener('click', () => {
    _lines.push({ item_id:'', qty:1, unit_price:0, discount_pct:0, tax_id:'', wht_rate:0 });
    renderLines();
  });
  renderLines();

  _modal.setFooter(`
    <button class="btn btn-secondary" id="pi-cancel">Cancel</button>
    <button class="btn btn-primary" id="pi-save">Create Invoice</button>
  `);
  _modal.getFooter().querySelector('#pi-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#pi-save').addEventListener('click', savePI);
}

async function savePI() {
  const g = id => document.getElementById(id)?.value?.trim() || null;
  if (!g('pi-vendor'))  { Toast.warning('Select a vendor.'); return; }
  if (!g('pi-date'))    { Toast.warning('Invoice date required.'); return; }
  if (_lines.length===0){ Toast.warning('Add at least one line.'); return; }
  if (_lines.some(l => !l.item_id)) { Toast.warning('All lines need an item.'); return; }

  const payload = {
    vendor_id:       g('pi-vendor'),
    invoice_date:    g('pi-date'),
    due_date:        g('pi-due-date') || undefined,
    vendor_ref:      g('pi-vendor-ref') || undefined,
    payment_term_id: g('pi-terms') || undefined,
    currency_id:     g('pi-currency') || undefined,
    notes:           g('pi-notes') || undefined,
    lines: _lines.map(l => ({
      item_id:      l.item_id,
      qty:          l.qty,
      unit_price:   l.unit_price,
      discount_pct: l.discount_pct || 0,
      tax_id:       l.tax_id || undefined,
      wht_rate:     l.wht_rate || 0,
    })),
  };

  const btn = document.getElementById('pi-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    const res = await PurchasesApi.pi.create(payload);
    Toast.success(`Invoice created: ${res.number}`);
    _modal.close();
    await loadTable();
  } catch (err) {
    Toast.error(err.message || 'Failed to create PI.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Detail view modal
// ─────────────────────────────────────────────────────────────────────────────

function showDetailModal(data) {
  const m = new Modal({
    title: `PI ${data.pi_number} — ${data.vendor_name}`,
    size:  'xl',
    content: `
      <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Invoice Date</span><br>${formatDate(data.invoice_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Due Date</span><br>${data.due_date ? formatDate(data.due_date) : '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Vendor Ref</span><br>${data.vendor_ref||'—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Grand Total</span><br><strong>${formatAmount(data.grand_total,'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Amount Due</span><br><strong style="color:var(--danger)">${formatAmount(data.amount_due,'PKR')}</strong></div>
      </div>
      <table class="table" style="font-size:0.84rem">
        <thead><tr><th>#</th><th>Item</th><th style="text-align:right">Qty</th><th style="text-align:right">Price</th><th style="text-align:right">Disc%</th><th style="text-align:right">Tax</th><th style="text-align:right">WHT</th><th style="text-align:right">Total</th></tr></thead>
        <tbody>
          ${(data.lines||[]).map((l,i) => `
            <tr>
              <td>${i+1}</td>
              <td>${l.item_code} — ${l.item_name}</td>
              <td style="text-align:right">${l.qty}</td>
              <td style="text-align:right">${formatAmount(l.unit_price,'PKR')}</td>
              <td style="text-align:right">${l.discount_pct||0}%</td>
              <td style="text-align:right">${formatAmount(l.tax_amt,'PKR')}</td>
              <td style="text-align:right">${formatAmount(l.wht_amt,'PKR')}</td>
              <td style="text-align:right">${formatAmount(l.line_total,'PKR')}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
      <div style="display:flex;justify-content:flex-end;margin-top:1rem;font-size:0.88rem">
        <table>
          <tr><td style="padding:3px 16px 3px 0;color:var(--text-muted)">Subtotal</td><td>${formatAmount(data.subtotal,'PKR')}</td></tr>
          <tr><td style="color:var(--text-muted)">Discount</td><td>${formatAmount(data.discount_total,'PKR')}</td></tr>
          <tr><td style="color:var(--text-muted)">Tax</td><td>${formatAmount(data.tax_total,'PKR')}</td></tr>
          <tr><td style="color:var(--text-muted)">WHT</td><td>${formatAmount(data.wht_total,'PKR')}</td></tr>
          <tr style="font-weight:700;border-top:1px solid var(--border)"><td style="padding-top:6px">Grand Total</td><td>${formatAmount(data.grand_total,'PKR')}</td></tr>
        </table>
      </div>
    `,
  });
  m.open();

  const footerHtml = `
    <button class="btn btn-secondary" id="pi-det-close">Close</button>
    ${data.status === 'draft' ? `<button class="btn btn-primary" id="pi-det-post">Post Invoice</button>` : ''}
  `;
  m.setFooter(footerHtml);
  m.getFooter().querySelector('#pi-det-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#pi-det-post')?.addEventListener('click', async () => {
    const ok = await Confirm({
      title:       'Post Invoice?',
      message:     'Posting will create a journal voucher and lock the invoice.',
      type:        'primary',
      confirmText: 'Post',
    });
    if (ok) {
      try {
        await PurchasesApi.pi.post(data.id);
        Toast.success('Invoice posted. JV created.');
        m.close();
        await loadTable();
      } catch(e) { Toast.error(e.message); }
    }
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Load table
// ─────────────────────────────────────────────────────────────────────────────

async function loadTable() {
  try {
    const res = await PurchasesApi.pi.list({ limit: 200 });
    _table?.setData(res.items || []);
  } catch {
    Toast.error('Failed to load purchase invoices.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Purchase Invoices</h1>
        <p class="page-subtitle">Record vendor invoices with WHT and auto journal voucher</p>
      </div>
      <button class="btn btn-primary" id="add-pi-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Invoice
      </button>
    </div>
    <div id="pi-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#pi-table',
    columns: [
      { key: 'pi_number',    label: 'PI #',       width: '120px' },
      { key: 'vendor_name',  label: 'Vendor' },
      { key: 'invoice_date', label: 'Date',        width: '100px', render: formatDate },
      { key: 'due_date',     label: 'Due',         width: '100px', render: v => v ? formatDate(v) : '—' },
      { key: 'vendor_ref',   label: 'Vendor Ref',  width: '110px', render: v => v || '—' },
      { key: 'grand_total',  label: 'Total',       align: 'right', render: v => formatAmount(v,'PKR') },
      { key: 'amount_due',   label: 'Due Amt',     align: 'right', render: v => formatAmount(v,'PKR') },
      { key: 'status',       label: 'Status',      width: '90px',  render: v => StatusBadge(v) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            ${row.status === 'draft' ? `
              <button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button>
              <button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>
            ` : ''}
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view') {
        try { const d = await PurchasesApi.pi.get(row.id); showDetailModal(d); }
        catch { Toast.error('Failed to load invoice.'); }
        return;
      }
      if (action === 'post') {
        const ok = await Confirm({ title:'Post Invoice?', message:'Creates JV and locks invoice.', type:'primary', confirmText:'Post' });
        if (ok) { try { await PurchasesApi.pi.post(row.id); Toast.success('Posted.'); await loadTable(); } catch(e){ Toast.error(e.message); } }
        return;
      }
      if (action === 'cancel') {
        const ok = await Confirm({ title:'Cancel Invoice?', message:'Cancel this invoice?', type:'danger', confirmText:'Cancel' });
        if (ok) { try { await PurchasesApi.pi.cancel(row.id); Toast.success('Cancelled.'); await loadTable(); } catch(e){ Toast.error(e.message); } }
      }
    },
  });

  document.getElementById('add-pi-btn').addEventListener('click', openModal);
  await loadTable();
}

export default { init };
