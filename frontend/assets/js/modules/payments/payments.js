/**
 * payments.js — Cash/Bank Payments module (payments to vendors + WHT)
 */

'use strict';

import { PaymentsApi, MastersApi, PurchasesApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table    = null;
let _modal    = null;
let _vendors  = [];
let _bankAccts = [];

async function loadRefData() {
  const [vends, coa] = await Promise.allSettled([
    MastersApi.vendors.list({ active_only: true, limit: 500 }),
    MastersApi.coa.list({ limit: 500 }),
  ]);
  _vendors   = vends.value?.items || [];
  const allCoa = coa.value?.items || [];
  _bankAccts = allCoa.filter(a => a.account_type === 'asset' && (a.name.toLowerCase().includes('bank') || a.name.toLowerCase().includes('cash')));
}

async function openModal() {
  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();

  _modal = new Modal({
    title: 'New Payment',
    size: 'lg',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">Vendor</label>
          <select class="input" id="cp-vendor">
            <option value="">Select Vendor…</option>
            ${_vendors.map(v => `<option value="${v.id}" data-wht="${v.wht_rate || 0}">${v.code} — ${v.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label required">Payment Date</label>
          <input class="input" type="date" id="cp-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label required">Gross Amount</label>
          <input class="input" type="number" id="cp-amount" min="0.01" step="0.01" placeholder="0.00">
        </div>
        <div class="form-group">
          <label class="form-label">WHT Amount</label>
          <input class="input" type="number" id="cp-wht" min="0" step="0.01" placeholder="0.00" value="0">
        </div>
        <div class="form-group">
          <label class="form-label required">Bank / Cash Account</label>
          <select class="input" id="cp-bank">
            <option value="">Select Account…</option>
            ${_bankAccts.map(a => `<option value="${a.id}">${a.code} — ${a.name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Linked Purchase Invoice (optional)</label>
          <select class="input" id="cp-pi">
            <option value="">None</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Reference / Cheque No.</label>
          <input class="input" type="text" id="cp-ref" maxlength="50" placeholder="Optional">
        </div>
        <div class="form-group">
          <label class="form-label">Net Payable (computed)</label>
          <input class="input" type="text" id="cp-net" readonly placeholder="Amount − WHT">
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <textarea class="input" id="cp-notes" rows="2" placeholder="Optional"></textarea>
      </div>
    `,
  });
  _modal.open();
  _modal.setFooter(`
    <button class="btn btn-secondary" id="cp-cancel">Cancel</button>
    <button class="btn btn-primary" id="cp-save">Create Payment</button>
  `);
  _modal.getFooter().querySelector('#cp-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#cp-save').addEventListener('click', savePayment);

  // Auto-calculate net payable
  const updateNet = () => {
    const amt = parseFloat(document.getElementById('cp-amount')?.value) || 0;
    const wht = parseFloat(document.getElementById('cp-wht')?.value) || 0;
    const netEl = document.getElementById('cp-net');
    if (netEl) netEl.value = formatAmount(Math.max(0, amt - wht), 'PKR');
  };
  document.getElementById('cp-amount').addEventListener('input', updateNet);
  document.getElementById('cp-wht').addEventListener('input', updateNet);

  // Vendor change → auto-fill WHT + load open PIs
  document.getElementById('cp-vendor').addEventListener('change', async (e) => {
    const opt = e.target.selectedOptions[0];
    const whtRate = parseFloat(opt?.dataset.wht || 0);
    const amtEl = document.getElementById('cp-amount');
    const whtEl = document.getElementById('cp-wht');
    const amt = parseFloat(amtEl?.value) || 0;
    if (whtEl && whtRate > 0 && amt > 0) {
      whtEl.value = (amt * whtRate / 100).toFixed(2);
      updateNet();
    }

    const vid = e.target.value;
    const piSel = document.getElementById('cp-pi');
    piSel.innerHTML = '<option value="">None</option>';
    if (!vid) return;
    try {
      const r = await PurchasesApi.pi.list({ vendor_id: vid, status: 'posted', limit: 200 });
      const open = (r.items || []).filter(p => p.amount_due > 0);
      open.forEach(p => {
        const opt2 = document.createElement('option');
        opt2.value = p.id;
        opt2.textContent = `${p.pi_number} — Due: ${formatAmount(p.amount_due, 'PKR')}`;
        piSel.appendChild(opt2);
      });
    } catch { /* ignore */ }
  });

  // Re-compute WHT when amount changes (with last selected rate)
  document.getElementById('cp-amount').addEventListener('blur', () => {
    const opt = document.getElementById('cp-vendor')?.selectedOptions[0];
    const whtRate = parseFloat(opt?.dataset.wht || 0);
    const amt = parseFloat(document.getElementById('cp-amount')?.value) || 0;
    if (whtRate > 0 && amt > 0) {
      document.getElementById('cp-wht').value = (amt * whtRate / 100).toFixed(2);
      updateNet();
    }
  });
}

async function savePayment() {
  const g = id => document.getElementById(id)?.value?.trim() || null;
  if (!g('cp-vendor'))  { Toast.warning('Select a vendor.'); return; }
  if (!g('cp-date'))    { Toast.warning('Payment date required.'); return; }
  const amt = parseFloat(g('cp-amount'));
  if (!amt || amt <= 0) { Toast.warning('Enter a valid amount.'); return; }
  if (!g('cp-bank'))    { Toast.warning('Select a bank/cash account.'); return; }

  const payload = {
    vendor_id:       g('cp-vendor'),
    payment_date:    g('cp-date'),
    amount:          amt,
    wht_amount:      parseFloat(g('cp-wht') || '0') || 0,
    bank_account_id: g('cp-bank'),
    pi_id:           g('cp-pi') || undefined,
    reference:       g('cp-ref') || undefined,
    notes:           g('cp-notes') || undefined,
  };

  const btn = document.getElementById('cp-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    const r = await PaymentsApi.payments.create(payload);
    Toast.success(`Payment created: ${r.number}`);
    _modal.close();
    await loadTable();
  } catch (e) {
    Toast.error(e.message);
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

function showDetail(data) {
  const net = (data.amount || 0) - (data.wht_amount || 0);
  const m = new Modal({
    title: `Payment ${data.payment_number} — ${data.vendor_name}`,
    size: 'md',
    content: `
      <div style="display:grid;grid-template-columns:repeat(2,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Date</span><br>${formatDate(data.payment_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Gross Amount</span><br><strong>${formatAmount(data.amount, 'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">WHT Deducted</span><br>${formatAmount(data.wht_amount || 0, 'PKR')}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Net Paid</span><br><strong style="color:var(--primary)">${formatAmount(net, 'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Reference</span><br>${data.reference || '—'}</div>
      </div>
      ${data.notes ? `<div style="font-size:0.85rem;color:var(--text-muted)"><strong>Notes:</strong> ${data.notes}</div>` : ''}
    `,
  });
  m.open();
  const canPost = data.status === 'draft';
  m.setFooter(`
    <button class="btn btn-secondary" id="cp-det-close">Close</button>
    ${canPost ? `<button class="btn btn-primary" id="cp-det-post">Post Payment</button>` : ''}
    ${canPost ? `<button class="btn btn-ghost-danger" id="cp-det-cancel">Cancel</button>` : ''}
  `);
  m.getFooter().querySelector('#cp-det-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#cp-det-post')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Post Payment?', message: 'Creates Journal Voucher and locks the payment.', type: 'primary', confirmText: 'Post' });
    if (ok) {
      try {
        await PaymentsApi.payments.post(data.id);
        Toast.success('Payment posted.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
  m.getFooter().querySelector('#cp-det-cancel')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Cancel Payment?', message: 'Cancel this payment?', type: 'danger', confirmText: 'Cancel' });
    if (ok) {
      try {
        await PaymentsApi.payments.cancel(data.id);
        Toast.success('Cancelled.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
}

async function loadTable() {
  try {
    const r = await PaymentsApi.payments.list({ limit: 200 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load payments.');
  }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Cash / Bank Payments</h1>
        <p class="page-subtitle">Vendor payments with WHT deduction</p>
      </div>
      <button class="btn btn-primary" id="add-cp-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Payment
      </button>
    </div>
    <div id="cp-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#cp-table',
    columns: [
      { key: 'payment_number', label: 'Payment #', width: '130px' },
      { key: 'vendor_name',    label: 'Vendor' },
      { key: 'payment_date',   label: 'Date', width: '100px', render: formatDate },
      { key: 'amount',         label: 'Gross', align: 'right', render: v => formatAmount(v, 'PKR') },
      { key: 'wht_amount',     label: 'WHT', align: 'right', render: v => formatAmount(v || 0, 'PKR') },
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
          const d = await PaymentsApi.payments.get(row.id);
          showDetail(d);
        } catch { Toast.error('Failed to load payment.'); }
        return;
      }
      if (action === 'post') {
        const ok = await Confirm({ title: 'Post Payment?', message: 'Creates JV and locks the payment.', type: 'primary', confirmText: 'Post' });
        if (ok) {
          try {
            await PaymentsApi.payments.post(row.id);
            Toast.success('Posted.');
            await loadTable();
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  document.getElementById('add-cp-btn').addEventListener('click', openModal);
  await loadTable();
}

export default { init };
