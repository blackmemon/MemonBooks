/**
 * vendors.js — Vendors master module (CRUD + WHT settings)
 */

'use strict';

import { MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, Validator, formatAmount, FormGroup, SelectGroup,
} from '../../components/components.js';

let _table  = null;
let _modal  = null;
let _editId = null;
let _payTerms   = [];
let _currencies = [];
let _coaAccounts= [];
let _taxConfig  = [];

async function loadRefData() {
  const [pt, cur, coa, tax] = await Promise.allSettled([
    MastersApi.paymentTerms.list(),
    MastersApi.currencies.list(),
    MastersApi.coa.list({ flat_dropdown: true }),
    fetch('/api/masters/tax?applies_to=purchases').then(r => r.json()).catch(() => []),
  ]);
  _payTerms    = pt.value  || [];
  _currencies  = cur.value || [];
  _coaAccounts = coa.value || [];
  _taxConfig   = tax.value || [];
}

function buildForm(data = {}) {
  const ptOpts  = _payTerms.map(p   => ({ value: p.id,  label: p.name }));
  const curOpts = _currencies.map(c  => ({ value: c.id,  label: `${c.code} — ${c.name}` }));
  const coaOpts = _coaAccounts.map(a => ({ value: a.id,  label: a.display_name || a.name }));

  return `
    <div class="tabs">
      <button class="tab-btn active" data-tab="general">General</button>
      <button class="tab-btn" data-tab="financial">Financial</button>
      <button class="tab-btn" data-tab="bank">Bank & WHT</button>
    </div>

    <div class="tab-panel active" data-tab="general" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Vendor Code', id:'vnd-code', required:true, value: data.code||'', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Vendor Name', id:'vnd-name', required:true, value: data.name||'' })}
        ${FormGroup({ label:'NTN', id:'vnd-ntn', value: data.ntn||'', placeholder:'1234567-8' })}
        ${FormGroup({ label:'STRN', id:'vnd-strn', value: data.strn||'' })}
        ${FormGroup({ label:'CNIC', id:'vnd-cnic', value: data.cnic||'', placeholder:'12345-6789012-3', hint:'Stored encrypted' })}
        ${FormGroup({ label:'Phone', id:'vnd-phone', value: data.phone||'' })}
        ${FormGroup({ label:'Email', id:'vnd-email', type:'email', value: data.email||'' })}
        ${FormGroup({ label:'City', id:'vnd-city', value: data.city||'' })}
      </div>
      <div class="form-group">
        <label class="form-label" for="vnd-address">Address</label>
        <textarea class="input" id="vnd-address" rows="2">${data.address||''}</textarea>
      </div>
    </div>

    <div class="tab-panel" data-tab="financial" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Credit Limit (PKR)', id:'vnd-credit-limit', type:'number', value: data.credit_limit||0 })}
        ${FormGroup({ label:'Credit Days', id:'vnd-credit-days', type:'number', value: data.credit_days||30 })}
        ${SelectGroup({ label:'Payment Terms', id:'vnd-payment-term', value: data.payment_term_id||'', options: ptOpts })}
        ${SelectGroup({ label:'Currency', id:'vnd-currency', value: data.currency_id||'', options: curOpts })}
        ${SelectGroup({ label:'Payable Account', id:'vnd-account', value: data.account_id||'', options: coaOpts })}
      </div>
    </div>

    <div class="tab-panel" data-tab="bank" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Bank Name', id:'vnd-bank-name', value: data.bank_name||'' })}
        ${FormGroup({ label:'Bank IBAN', id:'vnd-iban', value: data.bank_iban||'', placeholder:'PK36SCBL0000001123456702', hint:'Stored encrypted' })}
        ${FormGroup({ label:'Bank Account #', id:'vnd-bank-acc', value: data.bank_account||'' })}
        ${FormGroup({ label:'WHT Rate (%)', id:'vnd-wht-rate', type:'number', value: data.wht_rate||0, placeholder:'0.00' })}
        ${SelectGroup({ label:'WHT Account', id:'vnd-wht-account', value: data.wht_account_id||'', options: coaOpts })}
      </div>
    </div>
  `;
}

const SCHEMA = {
  'vnd-code': { required: true },
  'vnd-name': { required: true },
  'vnd-email': { type: 'email' },
  'vnd-cnic':  { type: 'cnic' },
  'vnd-iban':  { type: 'iban' },
  'vnd-wht-rate': { type: 'percent' },
};

function collectForm() {
  const g  = (id) => document.getElementById(id)?.value?.trim() || null;
  const gn = (id) => parseFloat(document.getElementById(id)?.value) || 0;
  const gi = (id) => parseInt(document.getElementById(id)?.value)   || 0;
  return {
    code: g('vnd-code'), name: g('vnd-name'),
    ntn: g('vnd-ntn')||undefined, strn: g('vnd-strn')||undefined,
    cnic: g('vnd-cnic')||undefined, phone: g('vnd-phone')||undefined,
    email: g('vnd-email')||undefined, address: g('vnd-address')||undefined,
    city: g('vnd-city')||undefined,
    bank_name: g('vnd-bank-name')||undefined,
    bank_iban: g('vnd-iban')||undefined, bank_account: g('vnd-bank-acc')||undefined,
    wht_rate: gn('vnd-wht-rate'), wht_account_id: g('vnd-wht-account')||undefined,
    credit_limit: gn('vnd-credit-limit'), credit_days: gi('vnd-credit-days')||30,
    payment_term_id: g('vnd-payment-term')||undefined,
    currency_id: g('vnd-currency')||undefined, account_id: g('vnd-account')||undefined,
  };
}

async function openModal(editData = null) {
  _editId = editData?.id || null;
  if (_modal) _modal.close();
  _modal = new Modal({ title: _editId ? 'Edit Vendor' : 'New Vendor', size: 'xl', content: buildForm(editData || {}) });
  _modal.open();
  _modal.getBody().querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      _modal.getBody().querySelectorAll('.tab-btn,.tab-panel').forEach(el => el.classList.remove('active'));
      btn.classList.add('active');
      _modal.getBody().querySelector(`.tab-panel[data-tab="${btn.dataset.tab}"]`)?.classList.add('active');
    });
  });
  _modal.setFooter(`
    <button class="btn btn-secondary" id="vnd-cancel">Cancel</button>
    <button class="btn btn-primary" id="vnd-save">${_editId ? 'Save Changes' : 'Create Vendor'}</button>
  `);
  _modal.getFooter().querySelector('#vnd-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#vnd-save').addEventListener('click', save);
}

async function save() {
  const data = collectForm();
  let hasError = false;
  for (const [id, rules] of Object.entries(SCHEMA)) {
    const el = document.getElementById(id);
    if (!el) continue;
    const r = Validator.validate(el.value, rules);
    if (!r.valid) { Validator.showError(el, r.message); hasError = true; }
    else Validator.clearError(el);
  }
  if (hasError) return;
  const btn = document.getElementById('vnd-save');
  btn.classList.add('loading'); btn.disabled = true;
  try {
    if (_editId) { await MastersApi.vendors.update(_editId, data); Toast.success('Vendor updated.'); }
    else { await MastersApi.vendors.create(data); Toast.success('Vendor created.'); }
    _modal.close(); await loadTable();
  } catch (err) { Toast.error(err.message); }
  finally { btn.classList.remove('loading'); btn.disabled = false; }
}

async function loadTable() {
  try {
    const res = await MastersApi.vendors.list({ active_only: false, limit: 500 });
    _table?.setData(res.items || []);
  } catch { Toast.error('Failed to load vendors.'); }
}

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div><h1 class="page-title">Vendors</h1><p class="page-subtitle">Manage supplier master data</p></div>
      <button class="btn btn-primary" id="add-vendor-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Vendor
      </button>
    </div>
    <div id="vendors-table"></div>
  `;
  await loadRefData();
  _table = new DataTable({
    el: '#vendors-table',
    columns: [
      { key: 'code',         label: 'Code', width: '100px' },
      { key: 'name',         label: 'Vendor Name' },
      { key: 'phone',        label: 'Phone', sortable: false },
      { key: 'city',         label: 'City' },
      { key: 'wht_rate',     label: 'WHT%', align: 'right', render: v => `${v||0}%` },
      { key: 'credit_limit', label: 'Credit Limit', align: 'right', render: v => formatAmount(v,'PKR') },
      { key: 'is_active',    label: 'Status', width: '80px', render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
      {
        key: '_actions', label: '', sortable: false,
        render: (_, row) => `
          <button class="btn btn-sm btn-secondary btn-icon" data-action="edit"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg></button>
          <button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/></svg></button>
        `,
      },
    ],
    onRowAction: async (action, row) => {
      if (action === 'edit') { const d = await MastersApi.vendors.get(row.id); openModal(d); }
      if (action === 'delete') {
        const ok = await Confirm({ title:'Delete Vendor?', message:`"${row.name}" will be deleted.`, type:'danger', confirmText:'Delete' });
        if (ok) { try { await MastersApi.vendors.delete(row.id); Toast.success('Deleted.'); await loadTable(); } catch(e){ Toast.error(e.message); } }
      }
    },
  });
  document.getElementById('add-vendor-btn').addEventListener('click', () => openModal());
  await loadTable();
}

export default { init };
