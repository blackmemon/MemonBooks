/**
 * payroll.js — Payroll Runs module
 */

'use strict';

import { HrApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table = null;
let _modal = null;

const MONTHS = ['January','February','March','April','May','June','July','August','September','October','November','December'];

async function openRunModal() {
  const now   = new Date();
  const today = now.toISOString().split('T')[0];
  const month = now.getMonth() + 1;
  const year  = now.getFullYear();

  if (_modal) _modal.close();
  _modal = new Modal({
    title: 'Generate Payroll Run',
    size:  'md',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">Month</label>
          <select class="input" id="pr-month">
            ${MONTHS.map((m, i) => `<option value="${i+1}" ${i+1===month?'selected':''}>${m}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label required">Year</label>
          <input class="input" type="number" id="pr-year" value="${year}" min="2000" max="2100">
        </div>
        <div class="form-group">
          <label class="form-label required">Run Date</label>
          <input class="input" type="date" id="pr-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label">Department (optional)</label>
          <input class="input" type="text" id="pr-dept" placeholder="Leave blank for all departments">
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <textarea class="input" id="pr-notes" rows="2" placeholder="Optional remarks"></textarea>
      </div>
      <div class="alert" style="margin-top:1rem;padding:0.75rem 1rem;background:var(--bg-info-subtle);border-radius:6px;font-size:0.85rem;color:var(--text-muted)">
        ℹ️ This will calculate salaries for all active employees based on attendance records and salary components. The run will be created in <strong>draft</strong> status for review.
      </div>
    `,
  });
  _modal.open();
  _modal.setFooter(`
    <button class="btn btn-secondary" id="pr-cancel">Cancel</button>
    <button class="btn btn-primary" id="pr-run">Generate Payroll</button>
  `);
  _modal.getFooter().querySelector('#pr-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#pr-run').addEventListener('click', generatePayroll);
}

async function generatePayroll() {
  const month = parseInt(document.getElementById('pr-month')?.value);
  const year  = parseInt(document.getElementById('pr-year')?.value);
  const date  = document.getElementById('pr-date')?.value;
  const notes = document.getElementById('pr-notes')?.value?.trim() || undefined;

  if (!month || !year || !date) { Toast.warning('Month, year, and run date are required.'); return; }

  const btn = _modal.getFooter().querySelector('#pr-run');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    const r = await HrApi.payroll.run({ month, year, run_date: date, notes });
    Toast.success(`Payroll run created: ${r.number}`);
    _modal.close();
    await loadTable();
  } catch (e) {
    Toast.error(e.message);
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

async function showDetail(runId) {
  let data;
  try { data = await HrApi.payroll.get(runId); }
  catch { Toast.error('Failed to load payroll run.'); return; }

  const lines = data.lines || [];
  const m = new Modal({
    title: `Payroll ${data.run_number} — ${MONTHS[data.month - 1]} ${data.year}`,
    size: 'xl',
    content: `
      <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Run Date</span><br>${formatDate(data.run_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Gross Total</span><br><strong>${formatAmount(data.total_gross || 0, 'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Net Total</span><br><strong style="color:var(--success)">${formatAmount(data.total_net || 0, 'PKR')}</strong></div>
      </div>
      <div style="overflow-x:auto">
        <table class="table" style="font-size:0.82rem;min-width:700px">
          <thead>
            <tr>
              <th>Code</th><th>Employee</th><th style="text-align:center">Present</th><th style="text-align:center">Absent</th><th style="text-align:center">OT Hrs</th>
              <th style="text-align:right">Gross</th><th style="text-align:right">Deductions</th><th style="text-align:right">Tax</th><th style="text-align:right">Net</th>
            </tr>
          </thead>
          <tbody>
            ${lines.map(l => `
              <tr>
                <td>${l.emp_code}</td>
                <td>${l.full_name}</td>
                <td style="text-align:center">${l.present_days}/${l.working_days}</td>
                <td style="text-align:center">${l.absent_days}</td>
                <td style="text-align:center">${l.overtime_hours || 0}</td>
                <td style="text-align:right">${formatAmount(l.gross_salary, 'PKR')}</td>
                <td style="text-align:right">${formatAmount(l.total_deductions, 'PKR')}</td>
                <td style="text-align:right">${formatAmount(l.income_tax, 'PKR')}</td>
                <td style="text-align:right;font-weight:600">${formatAmount(l.net_salary, 'PKR')}</td>
              </tr>
            `).join('')}
          </tbody>
          <tfoot>
            <tr style="border-top:2px solid var(--border);font-weight:700">
              <td colspan="5">Totals</td>
              <td style="text-align:right">${formatAmount(data.total_gross||0,'PKR')}</td>
              <td style="text-align:right">${formatAmount(data.total_deductions||0,'PKR')}</td>
              <td style="text-align:right">${formatAmount(data.total_income_tax||0,'PKR')}</td>
              <td style="text-align:right">${formatAmount(data.total_net||0,'PKR')}</td>
            </tr>
          </tfoot>
        </table>
      </div>
    `,
  });
  m.open();

  const isDraft    = data.status === 'draft';
  const isApproved = data.status === 'approved';
  m.setFooter(`
    <button class="btn btn-secondary" id="pr-det-close">Close</button>
    ${isDraft    ? `<button class="btn btn-warning"  id="pr-det-approve">Approve</button>` : ''}
    ${isApproved ? `<button class="btn btn-primary"  id="pr-det-post">Post Payroll</button>` : ''}
    ${isDraft    ? `<button class="btn btn-ghost-danger" id="pr-det-cancel">Cancel Run</button>` : ''}
  `);
  m.getFooter().querySelector('#pr-det-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#pr-det-approve')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Approve Payroll?', message: 'Approve this payroll run?', type: 'warning', confirmText: 'Approve' });
    if (ok) {
      try { await HrApi.payroll.approve(data.id); Toast.success('Payroll approved.'); m.close(); await loadTable(); }
      catch (e) { Toast.error(e.message); }
    }
  });
  m.getFooter().querySelector('#pr-det-post')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Post Payroll?', message: 'Creates JV and finalizes payroll. Cannot be undone.', type: 'primary', confirmText: 'Post' });
    if (ok) {
      try { await HrApi.payroll.post(data.id); Toast.success('Payroll posted. Journal voucher created.'); m.close(); await loadTable(); }
      catch (e) { Toast.error(e.message); }
    }
  });
  m.getFooter().querySelector('#pr-det-cancel')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Cancel Run?', message: 'Cancel this payroll run?', type: 'danger', confirmText: 'Cancel' });
    if (ok) {
      try { await HrApi.payroll.cancel(data.id); Toast.success('Cancelled.'); m.close(); await loadTable(); }
      catch (e) { Toast.error(e.message); }
    }
  });
}

async function loadTable() {
  try {
    const r = await HrApi.payroll.list({ limit: 100 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load payroll runs.');
  }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Payroll</h1>
        <p class="page-subtitle">Monthly payroll runs with attendance-based computation</p>
      </div>
      <button class="btn btn-primary" id="add-pr-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        Generate Payroll
      </button>
    </div>
    <div id="pr-table"></div>
  `;

  _table = new DataTable({
    el: '#pr-table',
    columns: [
      { key: 'run_number', label: 'Run #',   width: '110px' },
      {
        key: 'month', label: 'Period', width: '130px',
        render: (v, row) => `${MONTHS[(v || 1) - 1]} ${row.year}`,
      },
      { key: 'run_date',    label: 'Date',    width: '100px', render: formatDate },
      { key: 'total_gross', label: 'Gross',   align: 'right', width: '140px', render: v => formatAmount(v || 0, 'PKR') },
      { key: 'total_net',   label: 'Net Pay', align: 'right', width: '140px', render: v => formatAmount(v || 0, 'PKR') },
      { key: 'status',      label: 'Status',  width: '90px',  render: v => StatusBadge(v) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            ${row.status === 'draft' ? `<button class="btn btn-sm btn-warning" data-action="approve" style="font-size:0.78rem;padding:3px 10px">Approve</button>` : ''}
            ${row.status === 'approved' ? `<button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button>` : ''}
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view')    { await showDetail(row.id); return; }
      if (action === 'approve') {
        const ok = await Confirm({ title: 'Approve?', message: 'Approve this payroll run?', type: 'warning', confirmText: 'Approve' });
        if (ok) { try { await HrApi.payroll.approve(row.id); Toast.success('Approved.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
        return;
      }
      if (action === 'post') {
        const ok = await Confirm({ title: 'Post Payroll?', message: 'Creates JV and finalizes payroll.', type: 'primary', confirmText: 'Post' });
        if (ok) { try { await HrApi.payroll.post(row.id); Toast.success('Payroll posted.'); await loadTable(); } catch(e) { Toast.error(e.message); } }
      }
    },
  });

  document.getElementById('add-pr-btn').addEventListener('click', openRunModal);
  await loadTable();
}

export default { init };
