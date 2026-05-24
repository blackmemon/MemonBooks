/**
 * ledger.js — General Ledger, Trial Balance, Account Balance views
 */

'use strict';

import { AccountsApi, MastersApi } from '../../core/api.js';
import {
  Toast, DataTable, formatAmount, formatDate,
} from '../../components/components.js';

let _coa         = [];
let _activeTab   = 'ledger';
let _ledgerTable = null;
let _tbTable     = null;

async function loadCOA() {
  try {
    const r = await MastersApi.coa.list({ limit: 1000 });
    _coa = r.items || [];
  } catch { _coa = []; }
}

// ─────────────────────────────────────────────────────────────────────────────
// LEDGER TAB
// ─────────────────────────────────────────────────────────────────────────────

function renderLedgerTab(container) {
  const today  = new Date().toISOString().split('T')[0];
  const firstDOM = today.substring(0, 7) + '-01';

  container.innerHTML = `
    <div class="card" style="margin-bottom:1.5rem">
      <div class="card-body">
        <div style="display:flex;gap:1rem;flex-wrap:wrap;align-items:flex-end">
          <div class="form-group" style="margin:0;min-width:220px;flex:1">
            <label class="form-label required">Account</label>
            <select class="input" id="ledger-account">
              <option value="">Select account…</option>
              ${_coa.map(a => `<option value="${a.id}">${a.code} — ${a.name}</option>`).join('')}
            </select>
          </div>
          <div class="form-group" style="margin:0">
            <label class="form-label">From</label>
            <input class="input" type="date" id="ledger-from" value="${firstDOM}">
          </div>
          <div class="form-group" style="margin:0">
            <label class="form-label">To</label>
            <input class="input" type="date" id="ledger-to" value="${today}">
          </div>
          <button class="btn btn-primary" id="ledger-run">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
            Run Ledger
          </button>
        </div>
      </div>
    </div>

    <div id="ledger-summary" style="display:none;margin-bottom:1rem">
      <div style="display:flex;gap:1rem;flex-wrap:wrap">
        <div class="stat-card" style="flex:1;min-width:160px">
          <div class="stat-label">Account</div>
          <div class="stat-value" id="sum-acct" style="font-size:1rem;font-weight:600">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:140px">
          <div class="stat-label">Opening Balance</div>
          <div class="stat-value" id="sum-ob" style="font-size:1.1rem">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:140px">
          <div class="stat-label">Total Debit</div>
          <div class="stat-value" id="sum-dr" style="font-size:1.1rem;color:var(--primary)">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:140px">
          <div class="stat-label">Total Credit</div>
          <div class="stat-value" id="sum-cr" style="font-size:1.1rem;color:var(--danger)">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:140px">
          <div class="stat-label">Closing Balance</div>
          <div class="stat-value" id="sum-closing" style="font-size:1.1rem;font-weight:700">—</div>
        </div>
      </div>
    </div>

    <div id="ledger-table-wrap"></div>
  `;

  document.getElementById('ledger-run').addEventListener('click', runLedger);
}

async function runLedger() {
  const accountId = document.getElementById('ledger-account')?.value;
  const dateFrom  = document.getElementById('ledger-from')?.value  || undefined;
  const dateTo    = document.getElementById('ledger-to')?.value    || undefined;

  if (!accountId) { Toast.warning('Select an account to view ledger.'); return; }

  const btn = document.getElementById('ledger-run');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    const r = await AccountsApi.ledger({ account_id: accountId, date_from: dateFrom, date_to: dateTo, limit: 500 });

    // Summary
    const summaryEl = document.getElementById('ledger-summary');
    if (summaryEl) summaryEl.style.display = 'flex';

    const acc  = r.account || {};
    document.getElementById('sum-acct').textContent     = `${acc.code || ''} ${acc.name || ''}`;
    document.getElementById('sum-ob').textContent       = formatAmount(r.opening_balance || 0, 'PKR');

    const items   = r.items || [];
    const totalDr = items.reduce((s, i) => s + (i.debit || 0), 0);
    const totalCr = items.reduce((s, i) => s + (i.credit || 0), 0);
    const closing = (r.opening_balance || 0) + totalDr - totalCr;

    document.getElementById('sum-dr').textContent      = formatAmount(totalDr, 'PKR');
    document.getElementById('sum-cr').textContent      = formatAmount(totalCr, 'PKR');
    document.getElementById('sum-closing').textContent = formatAmount(closing, 'PKR');

    // Table
    const wrap = document.getElementById('ledger-table-wrap');
    if (wrap) {
      if (_ledgerTable) { _ledgerTable = null; wrap.innerHTML = ''; }
      wrap.innerHTML = '<div id="ledger-inner"></div>';

      _ledgerTable = new DataTable({
        el: '#ledger-inner',
        columns: [
          { key: 'jv_date',      label: 'Date',    width: '100px', render: formatDate },
          { key: 'jv_number',    label: 'JV #',    width: '110px' },
          { key: 'source_type',  label: 'Source',  width: '100px', render: v => v || 'Manual' },
          { key: 'narration',    label: 'Narration' },
          { key: 'debit',        label: 'Debit',   align: 'right', width: '120px', render: v => v > 0 ? formatAmount(v, 'PKR') : '—' },
          { key: 'credit',       label: 'Credit',  align: 'right', width: '120px', render: v => v > 0 ? formatAmount(v, 'PKR') : '—' },
          { key: 'running_balance', label: 'Balance', align: 'right', width: '130px',
            render: v => {
              const cls = v < 0 ? 'color:var(--danger)' : 'color:var(--text)';
              return `<span style="${cls};font-weight:600">${formatAmount(v, 'PKR')}</span>`;
            }
          },
        ],
      });
      _ledgerTable.setData(items);

      if (!items.length) {
        wrap.innerHTML += `<div style="text-align:center;padding:2rem;color:var(--text-muted)">No transactions found for the selected period.</div>`;
      }
    }
  } catch (e) {
    Toast.error(e.message || 'Failed to load ledger.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// TRIAL BALANCE TAB
// ─────────────────────────────────────────────────────────────────────────────

function renderTrialBalanceTab(container) {
  const today    = new Date().toISOString().split('T')[0];
  const firstDOM = today.substring(0, 7) + '-01';

  container.innerHTML = `
    <div class="card" style="margin-bottom:1.5rem">
      <div class="card-body">
        <div style="display:flex;gap:1rem;flex-wrap:wrap;align-items:flex-end">
          <div class="form-group" style="margin:0">
            <label class="form-label">From Date</label>
            <input class="input" type="date" id="tb-from" value="${firstDOM}">
          </div>
          <div class="form-group" style="margin:0">
            <label class="form-label">To Date</label>
            <input class="input" type="date" id="tb-to" value="${today}">
          </div>
          <button class="btn btn-primary" id="tb-run">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
            Generate Trial Balance
          </button>
        </div>
      </div>
    </div>

    <div id="tb-balance-status" style="display:none;margin-bottom:1rem"></div>
    <div id="tb-summary" style="display:none;margin-bottom:1rem">
      <div style="display:flex;gap:1rem;flex-wrap:wrap">
        <div class="stat-card" style="flex:1;min-width:180px">
          <div class="stat-label">Total Debit</div>
          <div class="stat-value" id="tb-total-dr" style="color:var(--primary)">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:180px">
          <div class="stat-label">Total Credit</div>
          <div class="stat-value" id="tb-total-cr" style="color:var(--danger)">—</div>
        </div>
        <div class="stat-card" style="flex:1;min-width:180px">
          <div class="stat-label">Status</div>
          <div class="stat-value" id="tb-balanced" style="font-size:1.1rem">—</div>
        </div>
      </div>
    </div>

    <div id="tb-table-wrap"></div>
  `;

  document.getElementById('tb-run').addEventListener('click', runTrialBalance);
}

async function runTrialBalance() {
  const dateFrom = document.getElementById('tb-from')?.value || undefined;
  const dateTo   = document.getElementById('tb-to')?.value   || undefined;

  const btn = document.getElementById('tb-run');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    const r = await AccountsApi.trialBalance({ date_from: dateFrom, date_to: dateTo });

    // Summary
    document.getElementById('tb-summary').style.display = 'flex';
    document.getElementById('tb-total-dr').textContent = formatAmount(r.total_debit, 'PKR');
    document.getElementById('tb-total-cr').textContent = formatAmount(r.total_credit, 'PKR');
    const balEl = document.getElementById('tb-balanced');
    if (balEl) {
      balEl.textContent = r.is_balanced ? '✓ Balanced' : '✗ Not Balanced';
      balEl.style.color = r.is_balanced ? 'var(--success)' : 'var(--danger)';
    }

    // Table
    const wrap = document.getElementById('tb-table-wrap');
    if (wrap) {
      wrap.innerHTML = '<div id="tb-inner"></div>';
      const items = (r.items || []).filter(row => row.total_debit > 0 || row.total_credit > 0);

      _tbTable = new DataTable({
        el: '#tb-inner',
        columns: [
          { key: 'code',          label: 'Code',    width: '100px' },
          { key: 'name',          label: 'Account Name',
            render: (v, row) => {
              const indent = Math.max(0, (row.level || 1) - 1) * 16;
              return `<span style="padding-left:${indent}px">${v}</span>`;
            }
          },
          { key: 'account_type',  label: 'Type',    width: '110px', render: v => v ? v.replace(/_/g,' ') : '' },
          { key: 'total_debit',   label: 'Debit',   align: 'right', width: '140px', render: v => v > 0 ? formatAmount(v, 'PKR') : '—' },
          { key: 'total_credit',  label: 'Credit',  align: 'right', width: '140px', render: v => v > 0 ? formatAmount(v, 'PKR') : '—' },
          { key: 'net',           label: 'Net',     align: 'right', width: '140px',
            render: v => {
              const cls = v < 0 ? 'color:var(--danger)' : '';
              return `<span style="${cls};font-weight:600">${formatAmount(v, 'PKR')}</span>`;
            }
          },
        ],
      });
      _tbTable.setData(items);
    }
  } catch (e) {
    Toast.error(e.message || 'Failed to generate trial balance.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// MODULE INIT
// ─────────────────────────────────────────────────────────────────────────────

function renderTabs(mainContainer) {
  const tabs = [
    { id: 'ledger',        label: 'General Ledger' },
    { id: 'trial-balance', label: 'Trial Balance' },
  ];

  mainContainer.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Accounts</h1>
        <p class="page-subtitle">Ledger and financial statements</p>
      </div>
    </div>
    <div class="tabs" style="margin-bottom:1.5rem">
      ${tabs.map(t => `
        <button class="tab-btn ${t.id === _activeTab ? 'active' : ''}" data-tab="${t.id}">${t.label}</button>
      `).join('')}
    </div>
    <div id="acc-tab-content"></div>
  `;

  mainContainer.querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      mainContainer.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      _activeTab = btn.dataset.tab;
      switchTab(mainContainer);
    });
  });

  switchTab(mainContainer);
}

function switchTab(mainContainer) {
  const content = mainContainer.querySelector('#acc-tab-content');
  if (!content) return;
  _ledgerTable = null;
  _tbTable     = null;

  if (_activeTab === 'ledger')        renderLedgerTab(content);
  else if (_activeTab === 'trial-balance') renderTrialBalanceTab(content);
}

export async function init(container, query) {
  // Auto-select tab based on current route path
  const path = window.location.hash.slice(1).split('?')[0];
  if (path.includes('trial-balance')) _activeTab = 'trial-balance';
  else _activeTab = 'ledger';

  await loadCOA();
  renderTabs(container);
}

export default { init };
