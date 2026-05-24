/**
 * customers.js — Customers master module (CRUD)
 * DataTable + Modal form + validation + field-level encrypted CNIC/IBAN
 */

'use strict';

import { MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, StatusBadge, Validator,
  formatAmount, FormGroup, SelectGroup,
} from '../../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────────────────

let _table   = null;
let _modal   = null;
let _editId  = null;
let _payTerms= [];
let _currencies = [];
let _coaAccounts = [];

// ─────────────────────────────────────────────────────────────────────────────
// Load reference data
// ─────────────────────────────────────────────────────────────────────────────

async function loadRefData() {
  const [pt, cur, coa] = await Promise.allSettled([
    MastersApi.paymentTerms.list(),
    MastersApi.currencies.list(),
    MastersApi.coa.list({ flat_dropdown: true, account_type: 'asset' }),
  ]);
  _payTerms    = pt.value  || [];
  _currencies  = cur.value || [];
  _coaAccounts = coa.value || [];
}

// ─────────────────────────────────────────────────────────────────────────────
// Form HTML
// ─────────────────────────────────────────────────────────────────────────────

function buildForm(data = {}) {
  const ptOpts   = _payTerms.map(p  => ({ value: p.id,   label: p.name }));
  const curOpts  = _currencies.map(c => ({ value: c.id,   label: `${c.code} — ${c.name}` }));
  const coaOpts  = _coaAccounts.map(a => ({ value: a.id,  label: a.display_name || a.name }));

  return `
    <div class="tabs" id="cust-tabs">
      <button class="tab-btn active" data-tab="general">General</button>
      <button class="tab-btn" data-tab="financial">Financial</button>
      <button class="tab-btn" data-tab="bank">Bank</button>
    </div>

    <!-- General Tab -->
    <div class="tab-panel active" data-tab="general" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Customer Code', id:'cust-code', required:true, value: data.code||'', placeholder:'e.g. CUST-001', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Customer Name', id:'cust-name', required:true, value: data.name||'', placeholder:'Full name or company name' })}
        ${FormGroup({ label:'NTN', id:'cust-ntn', value: data.ntn||'', placeholder:'e.g. 1234567-8' })}
        ${FormGroup({ label:'STRN', id:'cust-strn', value: data.strn||'', placeholder:'Sales Tax Reg. Number' })}
        ${FormGroup({ label:'CNIC', id:'cust-cnic', value: data.cnic||'', placeholder:'12345-6789012-3', hint:'Stored encrypted' })}
        ${FormGroup({ label:'Phone', id:'cust-phone', value: data.phone||'', placeholder:'+92 300 1234567' })}
        ${FormGroup({ label:'Email', id:'cust-email', type:'email', value: data.email||'', placeholder:'customer@example.com' })}
        ${FormGroup({ label:'City', id:'cust-city', value: data.city||'', placeholder:'Karachi' })}
      </div>
      <div class="form-group">
        <label class="form-label" for="cust-address">Address</label>
        <textarea class="input" id="cust-address" rows="2" placeholder="Full address">${data.address||''}</textarea>
      </div>
    </div>

    <!-- Financial Tab -->
    <div class="tab-panel" data-tab="financial" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Credit Limit (PKR)', id:'cust-credit-limit', type:'number', value: data.credit_limit||0, placeholder:'0.00' })}
        ${FormGroup({ label:'Credit Days', id:'cust-credit-days', type:'number', value: data.credit_days||30 })}
        ${SelectGroup({ label:'Payment Terms', id:'cust-payment-term', value: data.payment_term_id||'', options: ptOpts })}
        ${SelectGroup({ label:'Currency', id:'cust-currency', value: data.currency_id||'', options: curOpts })}
        ${SelectGroup({ label:'Receivable Account', id:'cust-account', value: data.account_id||'', options: coaOpts })}
      </div>
    </div>

    <!-- Bank Tab -->
    <div class="tab-panel" data-tab="bank" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Bank IBAN', id:'cust-iban', value: data.bank_iban||'', placeholder:'PK36SCBL0000001123456702', hint:'Stored encrypted' })}
      </div>
    </div>
  `;
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation schema
// ─────────────────────────────────────────────────────────────────────────────

const SCHEMA = {
  'cust-code':  { required: true, minLength: 1, maxLength: 20 },
  'cust-name':  { required: true, minLength: 2, maxLength: 150 },
  'cust-email': { type: 'email' },
  'cust-cnic':  { type: 'cnic' },
  'cust-iban':  { type: 'iban' },
  'cust-credit-limit': { type: 'amount', min: 0 },
};

// ─────────────────────────────────────────────────────────────────────────────
// Collect form data
// ─────────────────────────────────────────────────────────────────────────────

function collectForm() {
  const g = (id) => document.getElementById(id)?.value?.trim() || null;
  const gn = (id) => parseFloat(document.getElementById(id)?.value) || 0;
  const gi = (id) => parseInt(document.getElementById(id)?.value) || 0;

  return {
    code:            g('cust-code'),
    name:            g('cust-name'),
    ntn:             g('cust-ntn')   || undefined,
    strn:            g('cust-strn')  || undefined,
    cnic:            g('cust-cnic')  || undefined,
    phone:           g('cust-phone') || undefined,
    email:           g('cust-email') || undefined,
    address:         g('cust-address') || undefined,
    city:            g('cust-city')  || undefined,
    bank_iban:       g('cust-iban')  || undefined,
    credit_limit:    gn('cust-credit-limit'),
    credit_days:     gi('cust-credit-days') || 30,
    payment_term_id: g('cust-payment-term') || undefined,
    currency_id:     g('cust-currency')     || undefined,
    account_id:      g('cust-account')      || undefined,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Open modal
// ─────────────────────────────────────────────────────────────────────────────

async function openModal(editData = null) {
  _editId = editData?.id || null;
  const title = _editId ? 'Edit Customer' : 'New Customer';

  if (_modal) _modal.close();
  _modal = new Modal({ title, size: 'xl', content: buildForm(editData || {}) });
  _modal.open();

  // Init tabs
  _modal.getBody().querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      _modal.getBody().querySelectorAll('.tab-btn, .tab-panel').forEach(el => el.classList.remove('active'));
      btn.classList.add('active');
      _modal.getBody().querySelector(`.tab-panel[data-tab="${btn.dataset.tab}"]`)?.classList.add('active');
    });
  });

  _modal.setFooter(`
    <button class="btn btn-secondary" id="cust-cancel">Cancel</button>
    <button class="btn btn-primary" id="cust-save">
      ${_editId ? 'Save Changes' : 'Create Customer'}
    </button>
  `);

  _modal.getFooter().querySelector('#cust-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#cust-save').addEventListener('click', saveCustomer);
}

// ─────────────────────────────────────────────────────────────────────────────
// Save
// ─────────────────────────────────────────────────────────────────────────────

async function saveCustomer() {
  const data = collectForm();

  // Validate
  let hasError = false;
  for (const [fieldId, rules] of Object.entries(SCHEMA)) {
    const el = document.getElementById(fieldId);
    if (!el) continue;
    const result = Validator.validate(el.value, rules);
    if (!result.valid) {
      Validator.showError(el, result.message);
      hasError = true;
    } else {
      Validator.clearError(el);
    }
  }
  if (hasError) return;

  const btn = document.getElementById('cust-save');
  btn.classList.add('loading');
  btn.disabled = true;

  try {
    if (_editId) {
      await MastersApi.customers.update(_editId, data);
      Toast.success('Customer updated successfully.');
    } else {
      await MastersApi.customers.create(data);
      Toast.success('Customer created successfully.');
    }
    _modal.close();
    await loadTable();
  } catch (err) {
    Toast.error(err.message || 'Save failed.');
  } finally {
    btn.classList.remove('loading');
    btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Load table
// ─────────────────────────────────────────────────────────────────────────────

async function loadTable() {
  try {
    const res = await MastersApi.customers.list({ active_only: false, limit: 500 });
    _table?.setData(res.items || []);
  } catch (err) {
    Toast.error('Failed to load customers.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Customers</h1>
        <p class="page-subtitle">Manage customer master data</p>
      </div>
      <button class="btn btn-primary" id="add-customer-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Customer
      </button>
    </div>
    <div id="customers-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#customers-table',
    columns: [
      { key: 'code',         label: 'Code',         width: '100px' },
      { key: 'name',         label: 'Customer Name' },
      { key: 'phone',        label: 'Phone',        sortable: false },
      { key: 'city',         label: 'City' },
      { key: 'credit_limit', label: 'Credit Limit', align: 'right',
        render: (v) => formatAmount(v, 'PKR') },
      { key: 'credit_days',  label: 'Days', width: '60px', align: 'right' },
      { key: 'is_active',    label: 'Status', width: '80px',
        render: (v) => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <button class="btn btn-sm btn-secondary btn-icon" data-action="edit" title="Edit">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg>
          </button>
          <button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete" title="Delete">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/></svg>
          </button>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'edit') {
        try {
          const detail = await MastersApi.customers.get(row.id);
          openModal(detail);
        } catch { Toast.error('Failed to load customer.'); }
      }
      if (action === 'delete') {
        const ok = await Confirm({
          title: 'Delete Customer?',
          message: `"${row.name}" will be permanently deleted.`,
          type: 'danger',
          confirmText: 'Delete',
        });
        if (ok) {
          try {
            await MastersApi.customers.delete(row.id);
            Toast.success('Customer deleted.');
            await loadTable();
          } catch (err) { Toast.error(err.message); }
        }
      }
    },
  });

  document.getElementById('add-customer-btn').addEventListener('click', () => openModal());
  await loadTable();
}

export default { init };
