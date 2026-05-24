/**
 * employees.js — Employees module (HR)
 */

'use strict';

import { HrApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table = null;
let _modal = null;

// ─── Helpers ────────────────────────────────────────────────────────────────

const EMPLOYMENT_TYPES = ['permanent', 'contract', 'probation', 'part_time'];
const GENDER_OPTIONS   = ['male', 'female', 'other'];

async function openModal(editData = null) {
  const isEdit = !!editData;
  const today  = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();

  _modal = new Modal({
    title: isEdit ? `Edit Employee — ${editData.emp_code}` : 'New Employee',
    size:  'xl',
    content: `
      <div style="display:flex;gap:0;margin-bottom:1rem;border-bottom:1px solid var(--border)">
        <button class="tab-btn active" data-etab="personal">Personal Info</button>
        <button class="tab-btn" data-etab="job">Job & Salary</button>
        <button class="tab-btn" data-etab="bank">Bank Details</button>
      </div>

      <!-- TAB: Personal Info -->
      <div id="etab-personal">
        <div class="grid-2">
          <div class="form-group">
            <label class="form-label required">Full Name</label>
            <input class="input" type="text" id="emp-name" value="${editData?.full_name || ''}" placeholder="Full legal name">
          </div>
          <div class="form-group">
            <label class="form-label">Father's Name</label>
            <input class="input" type="text" id="emp-father" value="${editData?.father_name || ''}">
          </div>
          <div class="form-group">
            <label class="form-label">Date of Birth</label>
            <input class="input" type="date" id="emp-dob" value="${editData?.dob || ''}">
          </div>
          <div class="form-group">
            <label class="form-label">Gender</label>
            <select class="input" id="emp-gender">
              <option value="">Select…</option>
              ${GENDER_OPTIONS.map(g => `<option value="${g}" ${editData?.gender===g?'selected':''}>${g.charAt(0).toUpperCase()+g.slice(1)}</option>`).join('')}
            </select>
          </div>
          <div class="form-group">
            <label class="form-label">CNIC</label>
            <input class="input" type="text" id="emp-cnic" value="${editData?.cnic || ''}" placeholder="XXXXX-XXXXXXX-X">
          </div>
          <div class="form-group">
            <label class="form-label">Phone</label>
            <input class="input" type="text" id="emp-phone" value="${editData?.phone || ''}" placeholder="03XX-XXXXXXX">
          </div>
          <div class="form-group">
            <label class="form-label">Email</label>
            <input class="input" type="email" id="emp-email" value="${editData?.email || ''}">
          </div>
        </div>
        <div class="form-group">
          <label class="form-label">Address</label>
          <textarea class="input" id="emp-address" rows="2">${editData?.address || ''}</textarea>
        </div>
      </div>

      <!-- TAB: Job & Salary -->
      <div id="etab-job" style="display:none">
        <div class="grid-2">
          <div class="form-group">
            <label class="form-label required">Join Date</label>
            <input class="input" type="date" id="emp-join" value="${editData?.join_date || today}">
          </div>
          <div class="form-group">
            <label class="form-label">Employment Type</label>
            <select class="input" id="emp-type">
              ${EMPLOYMENT_TYPES.map(t => `<option value="${t}" ${editData?.employment_type===t?'selected':''}>${t.replace(/_/g,' ')}</option>`).join('')}
            </select>
          </div>
          <div class="form-group">
            <label class="form-label required">Basic Salary</label>
            <input class="input" type="number" id="emp-salary" min="0" step="0.01" value="${editData?.basic_salary || 0}">
          </div>
        </div>
      </div>

      <!-- TAB: Bank Details -->
      <div id="etab-bank" style="display:none">
        <div class="grid-2">
          <div class="form-group">
            <label class="form-label">Bank Name</label>
            <input class="input" type="text" id="emp-bank-name" value="${editData?.bank_name || ''}">
          </div>
          <div class="form-group">
            <label class="form-label">Account Number</label>
            <input class="input" type="text" id="emp-bank-acc" value="${editData?.bank_account || ''}">
          </div>
          <div class="form-group form-group-full">
            <label class="form-label">IBAN</label>
            <input class="input" type="text" id="emp-bank-iban" value="${editData?.bank_iban || ''}" placeholder="PK36SCBL0000001123456702">
          </div>
        </div>
      </div>
    `,
  });
  _modal.open();
  _modal.setFooter(`
    <button class="btn btn-secondary" id="emp-cancel">Cancel</button>
    <button class="btn btn-primary" id="emp-save">${isEdit ? 'Save Changes' : 'Create Employee'}</button>
  `);
  _modal.getFooter().querySelector('#emp-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#emp-save').addEventListener('click', () => saveEmployee(editData?.id));

  // Tab switching
  _modal.getContent().querySelectorAll('[data-etab]').forEach(btn => {
    btn.addEventListener('click', () => {
      _modal.getContent().querySelectorAll('[data-etab]').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const tab = btn.dataset.etab;
      _modal.getContent().querySelectorAll('[id^="etab-"]').forEach(el => {
        el.style.display = el.id === `etab-${tab}` ? '' : 'none';
      });
    });
  });
}

async function saveEmployee(editId = null) {
  const g = id => document.getElementById(id)?.value?.trim() || null;
  if (!g('emp-name'))  { Toast.warning('Full name is required.'); return; }
  if (!g('emp-join'))  { Toast.warning('Join date is required.'); return; }

  const payload = {
    full_name:       g('emp-name'),
    father_name:     g('emp-father')  || undefined,
    dob:             g('emp-dob')     || undefined,
    gender:          g('emp-gender')  || undefined,
    cnic:            g('emp-cnic')    || undefined,
    phone:           g('emp-phone')   || undefined,
    email:           g('emp-email')   || undefined,
    address:         g('emp-address') || undefined,
    join_date:       g('emp-join'),
    employment_type: document.getElementById('emp-type')?.value || 'permanent',
    basic_salary:    parseFloat(document.getElementById('emp-salary')?.value || '0') || 0,
    bank_name:       g('emp-bank-name')  || undefined,
    bank_account:    g('emp-bank-acc')   || undefined,
    bank_iban:       g('emp-bank-iban')  || undefined,
    salary_components: [],
  };

  const btn = document.getElementById('emp-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    if (editId) {
      await HrApi.employees.update(editId, payload);
      Toast.success('Employee updated.');
    } else {
      const r = await HrApi.employees.create(payload);
      Toast.success(`Employee created: ${r.number}`);
    }
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
    title: `${data.emp_code} — ${data.full_name}`,
    size:  'lg',
    content: `
      <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:1rem;margin-bottom:1.5rem">
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Employee Code</span><br><strong>${data.emp_code}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Employment Type</span><br>${data.employment_type?.replace(/_/g,' ') || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Join Date</span><br>${formatDate(data.join_date)}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Department</span><br>${data.department_name || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Designation</span><br>${data.designation_name || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Basic Salary</span><br><strong>${formatAmount(data.basic_salary || 0, 'PKR')}</strong></div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Phone</span><br>${data.phone || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">Email</span><br>${data.email || '—'}</div>
        <div><span style="font-size:0.8rem;color:var(--text-muted)">CNIC</span><br>${data.cnic || '—'}</div>
      </div>
      ${(data.salary_components || []).length ? `
        <div style="margin-top:1rem">
          <strong style="font-size:0.9rem">Salary Components</strong>
          <table class="table" style="margin-top:0.5rem;font-size:0.85rem">
            <thead><tr><th>Component</th><th>Type</th><th style="text-align:right">Amount</th><th style="text-align:right">%</th></tr></thead>
            <tbody>
              ${data.salary_components.map(c => `
                <tr>
                  <td>${c.name}</td>
                  <td>${c.component_type}</td>
                  <td style="text-align:right">${c.amount > 0 ? formatAmount(c.amount, 'PKR') : '—'}</td>
                  <td style="text-align:right">${c.percentage > 0 ? c.percentage + '%' : '—'}</td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      ` : ''}
    `,
  });
  m.open();
  m.setFooter(`
    <button class="btn btn-secondary" id="emp-det-close">Close</button>
    <button class="btn btn-primary" id="emp-det-edit">Edit</button>
    ${data.is_active ? `<button class="btn btn-ghost-danger" id="emp-det-deactivate">Deactivate</button>` : ''}
  `);
  m.getFooter().querySelector('#emp-det-close').addEventListener('click', () => m.close());
  m.getFooter().querySelector('#emp-det-edit').addEventListener('click', () => { m.close(); openModal(data); });
  m.getFooter().querySelector('#emp-det-deactivate')?.addEventListener('click', async () => {
    const ok = await Confirm({ title: 'Deactivate Employee?', message: `${data.full_name} will be marked inactive.`, type: 'danger', confirmText: 'Deactivate' });
    if (ok) {
      try {
        await HrApi.employees.deactivate(data.id);
        Toast.success('Employee deactivated.');
        m.close();
        await loadTable();
      } catch (e) { Toast.error(e.message); }
    }
  });
}

async function loadTable() {
  try {
    const r = await HrApi.employees.list({ active_only: false, limit: 500 });
    _table?.setData(r.items || []);
  } catch {
    Toast.error('Failed to load employees.');
  }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Employees</h1>
        <p class="page-subtitle">Employee profiles, salaries, and records</p>
      </div>
      <button class="btn btn-primary" id="add-emp-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Employee
      </button>
    </div>
    <div id="emp-table"></div>
  `;

  _table = new DataTable({
    el: '#emp-table',
    columns: [
      { key: 'emp_code',        label: 'Code',       width: '100px' },
      { key: 'full_name',       label: 'Name' },
      { key: 'department_name', label: 'Department', render: v => v || '—' },
      { key: 'employment_type', label: 'Type', width: '110px', render: v => v?.replace(/_/g,' ') || '—' },
      { key: 'join_date',       label: 'Joined', width: '100px', render: formatDate },
      { key: 'basic_salary',    label: 'Basic', align: 'right', width: '130px', render: v => formatAmount(v || 0, 'PKR') },
      { key: 'phone',           label: 'Phone', width: '120px', render: v => v || '—' },
      {
        key: 'is_active', label: 'Status', width: '80px',
        render: v => v ? `<span class="badge badge-success">Active</span>` : `<span class="badge badge-default">Inactive</span>`,
      },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <div class="row-actions">
            <button class="btn btn-sm btn-secondary btn-icon" data-action="view" title="View">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
            <button class="btn btn-sm btn-secondary btn-icon" data-action="edit" title="Edit">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
            </button>
          </div>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'view') {
        try { const d = await HrApi.employees.get(row.id); showDetail(d); } catch { Toast.error('Failed to load employee.'); }
        return;
      }
      if (action === 'edit') {
        try { const d = await HrApi.employees.get(row.id); openModal(d); } catch { Toast.error('Failed.'); }
      }
    },
  });

  document.getElementById('add-emp-btn').addEventListener('click', () => openModal());
  await loadTable();
}

export default { init };
