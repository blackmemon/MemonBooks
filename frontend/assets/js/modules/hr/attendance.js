/**
 * attendance.js — Employee Attendance module
 */

'use strict';

import { HrApi } from '../../core/api.js';
import {
  Toast, Modal, DataTable, formatDate,
} from '../../components/components.js';

let _table     = null;
let _employees = [];

const STATUS_COLORS = {
  present:  'badge-success',
  absent:   'badge-danger',
  half_day: 'badge-warning',
  leave:    'badge-info',
  holiday:  'badge-default',
};

const STATUS_OPTIONS = ['present', 'absent', 'half_day', 'leave', 'holiday'];

async function loadEmployees() {
  try {
    const r = await HrApi.employees.dropdown();
    _employees = r || [];
  } catch { _employees = []; }
}

async function openMarkModal() {
  const today = new Date().toISOString().split('T')[0];
  const m = new Modal({
    title: 'Mark Attendance',
    size:  'md',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group">
          <label class="form-label required">Employee</label>
          <select class="input" id="att-emp">
            <option value="">All Active Employees</option>
            ${_employees.map(e => `<option value="${e.id}">${e.emp_code} — ${e.full_name}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label required">Date</label>
          <input class="input" type="date" id="att-date" value="${today}">
        </div>
        <div class="form-group">
          <label class="form-label required">Status</label>
          <select class="input" id="att-status">
            ${STATUS_OPTIONS.map(s => `<option value="${s}">${s.replace(/_/g, ' ')}</option>`).join('')}
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Overtime Hours</label>
          <input class="input" type="number" id="att-ot" min="0" step="0.5" value="0">
        </div>
        <div class="form-group">
          <label class="form-label">Check In</label>
          <input class="input" type="time" id="att-checkin">
        </div>
        <div class="form-group">
          <label class="form-label">Check Out</label>
          <input class="input" type="time" id="att-checkout">
        </div>
      </div>
      <div class="form-group">
        <label class="form-label">Notes</label>
        <input class="input" type="text" id="att-notes" placeholder="Optional">
      </div>
      <div id="att-bulk-info" style="display:none;padding:0.5rem;background:var(--bg-warning-subtle);border-radius:6px;font-size:0.85rem;color:var(--text-muted);margin-top:0.5rem">
        ⚠️ No employee selected — attendance will be marked for <strong>all ${_employees.length} active employees</strong>.
      </div>
    `,
  });
  m.open();
  m.setFooter(`
    <button class="btn btn-secondary" id="att-cancel">Cancel</button>
    <button class="btn btn-primary" id="att-save">Mark Attendance</button>
  `);
  m.getFooter().querySelector('#att-cancel').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#att-save').addEventListener('click', () => saveAttendance(m));

  document.getElementById('att-emp').addEventListener('change', (e) => {
    const infoEl = document.getElementById('att-bulk-info');
    if (infoEl) infoEl.style.display = e.target.value ? 'none' : 'block';
  });
}

async function saveAttendance(m) {
  const empId   = document.getElementById('att-emp')?.value;
  const date    = document.getElementById('att-date')?.value;
  const status  = document.getElementById('att-status')?.value;
  const ot      = parseFloat(document.getElementById('att-ot')?.value) || 0;
  const checkIn = document.getElementById('att-checkin')?.value || undefined;
  const checkOut= document.getElementById('att-checkout')?.value || undefined;
  const notes   = document.getElementById('att-notes')?.value?.trim() || undefined;

  if (!date)   { Toast.warning('Date is required.'); return; }
  if (!status) { Toast.warning('Status is required.'); return; }

  const btn = m.getFooter().querySelector('#att-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    if (empId) {
      // Single employee
      await HrApi.attendance.mark({
        employee_id: empId, att_date: date, status,
        check_in: checkIn, check_out: checkOut, overtime_hours: ot, notes,
      });
      Toast.success('Attendance marked.');
    } else {
      // Bulk
      await HrApi.attendance.bulk({
        employee_ids: _employees.map(e => e.id),
        att_date: date, status, check_in: checkIn, check_out: checkOut,
      });
      Toast.success(`Attendance marked for all ${_employees.length} employees.`);
    }
    m.close();
    await loadTable();
  } catch (e) {
    Toast.error(e.message);
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

async function loadTable() {
  const params = {};
  const monthEl = document.getElementById('att-filter-month');
  const yearEl  = document.getElementById('att-filter-year');
  if (monthEl?.value) params.month = monthEl.value;
  if (yearEl?.value)  params.year  = yearEl.value;

  try {
    const r = await HrApi.attendance.list({ ...params, limit: 500 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load attendance.');
  }
}

export async function init(container) {
  const now   = new Date();
  const month = now.getMonth() + 1;
  const year  = now.getFullYear();

  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Attendance</h1>
        <p class="page-subtitle">Daily employee attendance records</p>
      </div>
      <button class="btn btn-primary" id="add-att-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        Mark Attendance
      </button>
    </div>

    <div class="card" style="margin-bottom:1rem">
      <div class="card-body">
        <div style="display:flex;gap:1rem;align-items:flex-end;flex-wrap:wrap">
          <div class="form-group" style="margin:0">
            <label class="form-label">Month</label>
            <select class="input" id="att-filter-month" style="width:120px">
              ${Array.from({length:12},(_,i)=>`<option value="${i+1}" ${i+1===month?'selected':''}>${new Date(2000,i,1).toLocaleString('default',{month:'long'})}</option>`).join('')}
            </select>
          </div>
          <div class="form-group" style="margin:0">
            <label class="form-label">Year</label>
            <input class="input" type="number" id="att-filter-year" value="${year}" style="width:90px">
          </div>
          <button class="btn btn-secondary" id="att-filter-btn">Filter</button>
        </div>
      </div>
    </div>

    <div id="att-table"></div>
  `;

  await loadEmployees();

  _table = new DataTable({
    el: '#att-table',
    columns: [
      { key: 'att_date',      label: 'Date',     width: '100px', render: formatDate },
      { key: 'emp_code',      label: 'Code',     width: '90px' },
      { key: 'employee_name', label: 'Employee' },
      {
        key: 'status', label: 'Status', width: '100px',
        render: v => `<span class="badge ${STATUS_COLORS[v] || 'badge-default'}">${v?.replace(/_/g,' ') || ''}</span>`,
      },
      { key: 'check_in',       label: 'In',  width: '80px',  render: v => v || '—' },
      { key: 'check_out',      label: 'Out', width: '80px',  render: v => v || '—' },
      { key: 'overtime_hours', label: 'OT Hrs', width: '80px', align: 'right', render: v => v > 0 ? v : '—' },
      { key: 'notes',          label: 'Notes', render: v => v || '' },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete" title="Delete">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4h6v2"/></svg>
          </button>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'delete') {
        const { Confirm } = await import('../../components/components.js');
        const ok = await Confirm({ title: 'Delete Record?', message: 'Remove this attendance entry?', type: 'danger', confirmText: 'Delete' });
        if (ok) {
          try {
            await HrApi.attendance.delete(row.id);
            Toast.success('Deleted.');
            await loadTable();
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  document.getElementById('add-att-btn').addEventListener('click', openMarkModal);
  document.getElementById('att-filter-btn').addEventListener('click', loadTable);
  await loadTable();
}

export default { init };
