/**
 * receipts.js — Cash/Bank Receipts module (customer payments received)
 */

'use strict';

import { PaymentsApi, MastersApi, SalesApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table     = null;
let _modal     = null;
let _customers = [];
let _bankAccts = [];

async function loadRefData() {
  const [custs, coa] = await Promise.allSettled([
    MastersApi.customers.list({ active_only: true, limit: 500 }),
    MastersApi.coa.list({ limit: 500 }),
  ]);
  _customers = custs.value?.items || [];
  const allCoa = coa.value?.items || [];
  // Bank/Cash accounts only (asset type)
  _bankAccts = allCoa.filter(a => a.account_type === 'asset' && (a.name.toLowerCase().includes('bank') || a.name.toLowerCase().includes('cash')));
}

async function openModal() {
  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();

  _modal = new Modal({
    title: 'New Receipt',
    size: 'lg',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">Customer</label>
          <select class="input" id="rc-customer">
            <option value="">Select Customer…</option>
            ${_customers.map(c => `<option value="${c.id}">${c.code} — ${c.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label required">Receipt Date</label>
          <input class="input" type="date" id="rc-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label required">Amount Received</label>
          <input class="input" type="number" id="rc-amount" min="0.01" step="0.01" placeholder="0.00">
        </div>
        <div class="form-group">
          <label class="form-label required">Bank / Cash Account</label>
          <select class="input" id="rc-bank">
            <option value="">Select Account…</option>
            ${_bankAccts.map(a => `<option value="${a.id}">${a.code} — ${a.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Linked Sale Invoice (optional)</label>
          <select class="input" id="rc-si">
            <option value="">None</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Reference / Cheque No.</label>
          <input class="input" type="text" id="rc-ref" maxlength="50" placeholder="Optional">
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <textarea class="input" id="rc-notes" rows="2" placeholder="Optional"></textarea>
      </div>
    `,
  });
  _modal.open();
  _modal.setFooter(`
    <button class="btn btn-secondary" id="rc-cancel">Cancel</button>
    <button class="btn btn-primary" id="rc-save">Create Receipt</button>
  `);
  _modal.getFooter().querySelector('#rc-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#rc-save').addEventListener('click', saveReceipt);

  // When customer selected, load their open SI list
  document.getElementById('rc-customer').addEventListener('change', async (e) => {
    const cid = e.target.value;
    const siSel = document.getElementById('rc-si');
    siSel.innerHTML = '<option value="">None</option>';
    if (!cid) return;
    try {
      const r = await SalesApi.si.list({ customer_id: cid, status: 'posted', limit: 200 });
      const open = (r.items || []).filter(s => s.amount_due > 0);
      open.forEach(s => {
        const opt = document.createElement('option');
        opt.value = s.id;
        opt.textContent = `${s.si_number} — Due: ${formatAmount(s.amount_due, 'PKR')}`;
        siSel.appendChild(opt);
      });
    } catch { /* ignore */ }
  });
}

async function saveReceipt() {
  const g = id => document.getElementById(id)?.value?.trim() || null;
  if (!g('rc-customer')) { Toast.warning('Select a customer.'); return; }
  if (!g('rc-date'))     { Toast.warning('Receipt date required.'); return; }
  if (!g('rc-amount') || parseFloat(g('rc-amount')) <= 0) { Toast.warning('Enter a valid amount.'); return; }
  if (!g('rc-bank'))     { Toast.warning('Select a bank/cash account.'); return; }

  const payload = {
    customer_id:          g('rc-customer'),
    receipt_date:         g('rc-date'),
    amount:               parseFloat(g('rc-amount')),
    bank_account_id:      g('rc-bank'),
    si_id:                g('rc-si') || undefined,
    reference:            g('rc-ref') || undefined,
    notes:                g('rc-notes') || undefined,
  };

  const btn = document.getElementById('rc-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    const r = await PaymentsApi.receipts.create(payload);
    Toast.success(`Receipt created: ${r.number}`);
    _modal.close();
    await loadTable();
  } catch (e) {
    Toast.error(e.message);
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

function showDetail(data) {
  const m = new Modal({
    title: `Receipt ${data.receipt_number} — ${data.customer_name}`,
    size: 'md',
    content: `
      <div style="display:grid;grid-template-columns:repeat(2,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Date</span><br>${formatDate(data.receipt_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Amount</span><br><strong>${formatAmount(data.amount, 'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Reference</span><br>${data.reference || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Customer</span><br>${data.customer_name}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Linked SI</span><br>${data.si_id ? data.si_id : '—'}</div>
      </div>
      ${data.notes ? `<div style="font-size:0.85rem;color:var(--text-muted)"><strong>Notes:</strong> ${data.notes}</div>` : ''}
    `,
  });
  m.open();
  const canPost = data.status === 'draft';
  m.setFooter(`
    <button class="btn btn-secondary" id="rc-det-close">Close</button>
    ${canPost ? `<button class="btn btn-primary" id="rc-det-post">Post Receipt</button>` : ''}
    ${canPost ? `<button class="btn btn-ghost-danger" id="rc-det-cancel">Cancel</button>` : ''}
  `);
  m.getFooter().querySelector('#rc-det-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#rc-det-post')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Post Receipt?', message: 'This will create a Journal Voucher and lock the receipt.', type: 'primary', confirmText: 'Post' });
    if (ok) {
      try {
        await PaymentsApi.receipts.post(data.id);
        Toast.success('Receipt posted.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
  m.getFooter().querySelector('#rc-det-cancel')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Cancel Receipt?', message: 'Cancel this receipt?', type: 'danger', confirmText: 'Cancel' });
    if (ok) {
      try {
        await PaymentsApi.receipts.cancel(data.id);
        Toast.success('Cancelled.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
}

async function loadTable() {
  try {
    const r = await PaymentsApi.receipts.list({ limit: 200 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load receipts.');
  }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Cash / Bank Receipts</h1>
        <p class="page-subtitle">Record customer payments received</p>
      </div>
      <button class="btn btn-primary" id="add-rc-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Receipt
      </button>
    </div>
    <div id="rc-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#rc-table',
    columns: [
      { key: 'receipt_number', label: 'Receipt #', width: '130px' },
      { key: 'customer_name',  label: 'Customer' },
      { key: 'receipt_date',   label: 'Date', width: '100px', render: formatDate },
      { key: 'amount',         label: 'Amount', align: 'right', render: v => formatAmount(v, 'PKR') },
      { key: 'reference',      label: 'Reference', width: '130px', render: v => v || '—' },
      { key: 'status',         label: 'Status', width: '90px', render: v => StatusBadge(v) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            ${row.status === 'draft' ? `
              <button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button>
            ` : ''}
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view') {
        try {
          const d = await PaymentsApi.receipts.get(row.id);
          showDetail(d);
        } catch { Toast.error('Failed to load receipt.'); }
        return;
      }
      if (action === 'post') {
        const ok = await Confirm({ title: 'Post Receipt?', message: 'Creates JV and locks the receipt.', type: 'primary', confirmText: 'Post' });
        if (ok) {
          try {
            await PaymentsApi.receipts.post(row.id);
            Toast.success('Posted.');
            await loadTable();
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  document.getElementById('add-rc-btn').addEventListener('click', openModal);
  await loadTable();
}

export default { init };
