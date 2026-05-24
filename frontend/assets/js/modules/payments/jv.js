/**
 * jv.js — Manual Journal Vouchers module
 */

'use strict';

import { PaymentsApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table   = null;
let _modal   = null;
let _coa     = [];
let _lines   = [];

async function loadRefData() {
  try {
    const r = await MastersApi.coa.list({ limit: 1000 });
    _coa = (r.items || []).filter(a => a.level >= 3); // Only posting-level accounts
  } catch { _coa = []; }
}

function renderLines() {
  const tbody = document.getElementById('jv-lines-body');
  if (!tbody) return;

  if (!_lines.length) {
    tbody.innerHTML = `<tr><td colspan="4" style="text-align:center;color:var(--text-muted);padding:1rem">Add at least 2 lines</td></tr>`;
    updateTotals();
    return;
  }

  const coaOptions = _coa.map(a => `<option value="${a.id}">${a.code} — ${a.name}</option>`).join('');

  tbody.innerHTML = _lines.map((ln, i) => `
    <tr>
      <td style="min-width:200px">
        <select class="input input-sm" data-field="account_id" data-idx="${i}">
          <option value="">Select Account…</option>
          ${_coa.map(a => `<option value="${a.id}" ${a.id === ln.account_id ? 'selected' : ''}>${a.code} — ${a.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:130px">
        <input class="input input-sm" type="number" min="0" step="0.01" value="${ln.debit || ''}" placeholder="0.00" data-field="debit" data-idx="${i}">
      </td>
      <td style="width:130px">
        <input class="input input-sm" type="number" min="0" step="0.01" value="${ln.credit || ''}" placeholder="0.00" data-field="credit" data-idx="${i}">
      </td>
      <td style="min-width:140px">
        <input class="input input-sm" type="text" value="${ln.narration || ''}" placeholder="Line narration…" data-field="narration" data-idx="${i}">
      </td>
      <td style="width:36px">
        <button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}" title="Remove line">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </td>
    </tr>
  `).join('');

  tbody.querySelectorAll('[data-field]').forEach(el => {
    el.addEventListener('change', () => {
      const idx = parseInt(el.dataset.idx);
      const f   = el.dataset.field;
      if (f === 'debit' || f === 'credit') {
        _lines[idx][f] = parseFloat(el.value) || 0;
        // Clear opposite side if this side has a value
        if (f === 'debit'  && _lines[idx].debit  > 0) _lines[idx].credit = 0;
        if (f === 'credit' && _lines[idx].credit > 0) _lines[idx].debit  = 0;
      } else {
        _lines[idx][f] = el.value;
      }
      updateTotals();
    });
    el.addEventListener('input', () => {
      if (el.dataset.field === 'debit' || el.dataset.field === 'credit') updateTotals();
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

function updateTotals() {
  let totalDr = 0, totalCr = 0;
  _lines.forEach(ln => { totalDr += ln.debit || 0; totalCr += ln.credit || 0; });

  const drEl  = document.getElementById('jv-total-dr');
  const crEl  = document.getElementById('jv-total-cr');
  const diffEl = document.getElementById('jv-diff');

  if (drEl)  drEl.textContent  = formatAmount(totalDr, 'PKR');
  if (crEl)  crEl.textContent  = formatAmount(totalCr, 'PKR');

  const diff = Math.abs(totalDr - totalCr);
  if (diffEl) {
    diffEl.textContent = diff < 0.01 ? '✓ Balanced' : `Difference: ${formatAmount(diff, 'PKR')}`;
    diffEl.style.color = diff < 0.01 ? 'var(--success)' : 'var(--danger)';
    diffEl.style.fontWeight = '600';
  }
}

async function openModal() {
  _lines = [
    { account_id: '', debit: 0, credit: 0, narration: '' },
    { account_id: '', debit: 0, credit: 0, narration: '' },
  ];
  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();

  _modal = new Modal({
    title: 'New Journal Voucher',
    size: 'xl',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">JV Date</label>
          <input class="input" type="date" id="jv-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label required">Narration / Description</label>
          <input class="input" type="text" id="jv-narration" placeholder="Describe this journal entry…" maxlength="300">
        </div>
      </div>
      <div style="display:flex;justify-content:space-between;align-items:center;margin:1rem 0 0.5rem">
        <strong style="font-size:0.9rem">Journal Lines</strong>
        <button class="btn btn-sm btn-secondary" id="jv-add-line">+ Add Line</button>
      </div>
      <div style="overflow-x:auto">
        <table class="table line-table" style="min-width:640px">
          <thead>
            <tr>
              <th>Account</th>
              <th style="width:130px;text-align:right">Debit</th>
              <th style="width:130px;text-align:right">Credit</th>
              <th>Narration</th>
              <th style="width:36px"></th>
            </tr>
          </thead>
          <tbody id="jv-lines-body"></tbody>
          <tfoot>
            <tr style="border-top:2px solid var(--border);font-weight:600">
              <td style="padding:8px 0">Totals</td>
              <td style="text-align:right;padding:8px 0" id="jv-total-dr">PKR 0.00</td>
              <td style="text-align:right;padding:8px 0" id="jv-total-cr">PKR 0.00</td>
              <td colspan="2" style="text-align:right;padding:8px 0"><span id="jv-diff" style="font-size:0.85rem">—</span></td>
            </tr>
          </tfoot>
        </table>
      </div>
    `,
  });
  _modal.open();
  _modal.setFooter(`
    <button class="btn btn-secondary" id="jv-cancel">Cancel</button>
    <button class="btn btn-primary" id="jv-save">Create JV (Draft)</button>
  `);
  _modal.getFooter().querySelector('#jv-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#jv-save').addEventListener('click', saveJV);
  document.getElementById('jv-add-line')?.addEventListener('click', () => {
    _lines.push({ account_id: '', debit: 0, credit: 0, narration: '' });
    renderLines();
  });
  renderLines();
}

async function saveJV() {
  const date   = document.getElementById('jv-date')?.value?.trim();
  const narr   = document.getElementById('jv-narration')?.value?.trim();
  if (!date) { Toast.warning('JV date required.'); return; }
  if (!narr || narr.length < 3) { Toast.warning('Narration required (min 3 chars).'); return; }
  if (_lines.length < 2) { Toast.warning('At least 2 lines required.'); return; }
  if (_lines.some(l => !l.account_id)) { Toast.warning('All lines must have an account selected.'); return; }

  const totalDr = _lines.reduce((s, l) => s + (l.debit || 0), 0);
  const totalCr = _lines.reduce((s, l) => s + (l.credit || 0), 0);
  if (Math.abs(totalDr - totalCr) > 0.01) {
    Toast.warning(`JV not balanced. Dr=${formatAmount(totalDr,'PKR')} Cr=${formatAmount(totalCr,'PKR')}`);
    return;
  }

  const payload = {
    jv_date:   date,
    narration: narr,
    lines:     _lines.map(l => ({
      account_id: l.account_id,
      debit:      l.debit  || 0,
      credit:     l.credit || 0,
      narration:  l.narration || undefined,
    })),
  };

  const btn = document.getElementById('jv-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    const r = await PaymentsApi.jv.create(payload);
    Toast.success(`JV created: ${r.number}`);
    _modal.close();
    await loadTable();
  } catch (e) {
    Toast.error(e.message);
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

async function showDetail(jvId) {
  let data;
  try {
    data = await PaymentsApi.jv.get(jvId);
  } catch {
    Toast.error('Failed to load JV.');
    return;
  }

  const totalDr = (data.lines || []).reduce((s, l) => s + (l.debit || 0), 0);
  const totalCr = (data.lines || []).reduce((s, l) => s + (l.credit || 0), 0);

  const m = new Modal({
    title: `JV ${data.jv_number}`,
    size: 'xl',
    content: `
      <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Date</span><br>${formatDate(data.jv_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Source</span><br>${data.source_type || 'Manual'}</div>
        <div style="grid-column:span 3"><span style="font-size:0.8rem;color:var(--text-muted)">Narration</span><br>${data.narration}</div>
      </div>
      <table class="table" style="font-size:0.85rem">
        <thead>
          <tr><th>Account</th><th style="text-align:right">Debit</th><th style="text-align:right">Credit</th><th>Narration</th></tr>
        </thead>
        <tbody>
          ${(data.lines || []).map(l => `
            <tr>
              <td>${l.account_code || ''} — ${l.account_name || ''}</td>
              <td style="text-align:right">${l.debit > 0 ? formatAmount(l.debit, 'PKR') : '—'}</td>
              <td style="text-align:right">${l.credit > 0 ? formatAmount(l.credit, 'PKR') : '—'}</td>
              <td style="color:var(--text-muted)">${l.narration || ''}</td>
            </tr>
          `).join('')}
        </tbody>
        <tfoot>
          <tr style="border-top:2px solid var(--border);font-weight:700">
            <td>Total</td>
            <td style="text-align:right">${formatAmount(totalDr, 'PKR')}</td>
            <td style="text-align:right">${formatAmount(totalCr, 'PKR')}</td>
            <td></td>
          </tr>
        </tfoot>
      </table>
    `,
  });
  m.open();

  const isDraft  = data.status === 'draft';
  const isPosted = data.status === 'posted';
  m.setFooter(`
    <button class="btn btn-secondary" id="jv-det-close">Close</button>
    ${isDraft  ? `<button class="btn btn-primary" id="jv-det-post">Post JV</button>` : ''}
    ${isPosted ? `<button class="btn btn-warning"  id="jv-det-rev">Reverse JV</button>` : ''}
  `);
  m.getFooter().querySelector('#jv-det-close').addEventListener('click', () => m.close());

  m.getFooter().querySelector('#jv-det-post')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Post JV?', message: 'This will post the journal voucher.', type: 'primary', confirmText: 'Post' });
    if (ok) {
      try {
        await PaymentsApi.jv.post(data.id);
        Toast.success('JV posted.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });

  m.getFooter().querySelector('#jv-det-rev')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Reverse JV?', message: 'Creates a reversing entry with swapped Dr/Cr. The original JV will be marked as reversed.', type: 'warning', confirmText: 'Reverse' });
    if (ok) {
      try {
        await PaymentsApi.jv.reverse(data.id);
        Toast.success('JV reversed. A new reversing entry has been created.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
}

async function loadTable() {
  try {
    const r = await PaymentsApi.jv.list({ limit: 200 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load journal vouchers.');
  }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Journal Vouchers</h1>
        <p class="page-subtitle">Manual and auto-generated journal entries</p>
      </div>
      <button class="btn btn-primary" id="add-jv-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Manual JV
      </button>
    </div>
    <div id="jv-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#jv-table',
    columns: [
      { key: 'jv_number',   label: 'JV #', width: '120px' },
      { key: 'jv_date',     label: 'Date', width: '100px', render: formatDate },
      { key: 'source_type', label: 'Source', width: '110px', render: v => v || 'Manual' },
      { key: 'narration',   label: 'Narration' },
      {
        key: 'total_debit', label: 'Total Dr', align: 'right', width: '130px',
        render: v => formatAmount(v || 0, 'PKR'),
      },
      { key: 'status', label: 'Status', width: '90px', render: v => StatusBadge(v) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            ${row.status === 'draft' ? `<button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button>` : ''}
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view') { await showDetail(row.id); return; }
      if (action === 'post') {
        const ok = await Confirm({ title: 'Post JV?', message: 'Post this journal voucher?', type: 'primary', confirmText: 'Post' });
        if (ok) {
          try {
            await PaymentsApi.jv.post(row.id);
            Toast.success('JV posted.');
            await loadTable();
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  document.getElementById('add-jv-btn').addEventListener('click', openModal);
  await loadTable();
}

export default { init };
