/**
 * items.js — Items master module (CRUD + Price Lists + Stock View)
 */

'use strict';

import { MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  Badge, Validator, formatAmount, FormGroup, SelectGroup,
} from '../../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// State
// ─────────────────────────────────────────────────────────────────────────────

let _table    = null;
let _modal    = null;
let _editId   = null;
let _uomList  = [];
let _coaList  = [];
let _taxList  = [];
let _priceRows = [];   // editable price list rows in modal

// ─────────────────────────────────────────────────────────────────────────────
// Reference data
// ─────────────────────────────────────────────────────────────────────────────

async function loadRefData() {
  const [uom, coa, tax] = await Promise.allSettled([
    MastersApi.uom.list(),
    MastersApi.coa.list({ flat_dropdown: true }),
    fetch('/api/masters/tax').then(r => r.json()).catch(() => []),
  ]);
  _uomList  = uom.value  || [];
  _coaList  = coa.value  || [];
  _taxList  = tax.value  || [];
}

// ─────────────────────────────────────────────────────────────────────────────
// Form HTML
// ─────────────────────────────────────────────────────────────────────────────

function buildForm(data = {}) {
  const uomOpts = _uomList.map(u => ({ value: u.id, label: `${u.code} — ${u.name}` }));
  const coaOpts = _coaList.map(a => ({ value: a.id, label: a.display_name || a.name }));
  const taxOpts = _taxList.map(t => ({ value: t.id, label: `${t.name} (${t.rate}%)` }));

  const itemTypeOpts = [
    { value: 'product',      label: 'Product (Finished Goods)' },
    { value: 'raw_material', label: 'Raw Material' },
    { value: 'service',      label: 'Service' },
  ];

  return `
    <div class="tabs" id="item-tabs">
      <button class="tab-btn active" data-tab="general">General</button>
      <button class="tab-btn" data-tab="accounts">Accounts & Tax</button>
      <button class="tab-btn" data-tab="pricing">Pricing & Stock</button>
      <button class="tab-btn" data-tab="prices">Price List</button>
    </div>

    <!-- General -->
    <div class="tab-panel active" data-tab="general" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Item Code', id:'itm-code', required:true, value: data.code||'', placeholder:'e.g. ITEM-001', extra: _editId ? 'readonly' : '' })}
        ${FormGroup({ label:'Item Name', id:'itm-name', required:true, value: data.name||'', placeholder:'Full item name' })}
        ${SelectGroup({ label:'Item Type', id:'itm-type', value: data.item_type||'product', options: itemTypeOpts })}
        ${SelectGroup({ label:'Base UOM', id:'itm-uom', value: data.uom_id||'', options: uomOpts })}
        ${SelectGroup({ label:'Purchase UOM', id:'itm-purchase-uom', value: data.purchase_uom_id||'', options: uomOpts })}
        ${SelectGroup({ label:'Sale UOM', id:'itm-sale-uom', value: data.sale_uom_id||'', options: uomOpts })}
      </div>
      <div class="form-group">
        <label class="form-label" for="itm-desc">Description</label>
        <textarea class="input" id="itm-desc" rows="2" placeholder="Optional description">${data.description||''}</textarea>
      </div>
    </div>

    <!-- Accounts & Tax -->
    <div class="tab-panel" data-tab="accounts" style="padding-top:1rem">
      <div class="grid-2">
        ${SelectGroup({ label:'Purchase Account', id:'itm-purchase-acc', value: data.purchase_account_id||'', options: coaOpts })}
        ${SelectGroup({ label:'Sale Account', id:'itm-sale-acc', value: data.sale_account_id||'', options: coaOpts })}
        ${SelectGroup({ label:'Inventory Account', id:'itm-inventory-acc', value: data.inventory_account_id||'', options: coaOpts })}
        ${SelectGroup({ label:'Purchase Tax', id:'itm-purchase-tax', value: data.purchase_tax_id||'', options: taxOpts })}
        ${SelectGroup({ label:'Sale Tax', id:'itm-sale-tax', value: data.sale_tax_id||'', options: taxOpts })}
      </div>
    </div>

    <!-- Pricing & Stock -->
    <div class="tab-panel" data-tab="pricing" style="padding-top:1rem">
      <div class="grid-2">
        ${FormGroup({ label:'Standard Cost (PKR)', id:'itm-std-cost', type:'number', value: data.standard_cost||0, placeholder:'0.00' })}
        ${FormGroup({ label:'Standard Price (PKR)', id:'itm-std-price', type:'number', value: data.standard_price||0, placeholder:'0.00' })}
        ${FormGroup({ label:'Min Stock Level', id:'itm-min-stock', type:'number', value: data.min_stock||0 })}
        ${FormGroup({ label:'Max Stock Level', id:'itm-max-stock', type:'number', value: data.max_stock||0 })}
      </div>
      ${_editId ? `
        <div style="margin-top:1rem">
          <div class="card-header" style="padding:0.5rem 0; border-bottom: 1px solid var(--border)">
            <span class="form-label" style="margin:0">Stock by Location</span>
          </div>
          <div id="stock-by-location" style="margin-top:0.5rem">
            <div class="skeleton" style="height:80px;border-radius:8px"></div>
          </div>
        </div>
      ` : ''}
    </div>

    <!-- Price List -->
    <div class="tab-panel" data-tab="prices" style="padding-top:1rem">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:0.75rem">
        <span class="form-label" style="margin:0">Customer Price List</span>
        <button class="btn btn-sm btn-secondary" id="add-price-row-btn" type="button">+ Add Row</button>
      </div>
      <table class="table line-table" id="price-list-table">
        <thead>
          <tr>
            <th>Customer</th>
            <th style="width:100px">Min Qty</th>
            <th style="width:120px">Price (PKR)</th>
            <th style="width:100px">Valid From</th>
            <th style="width:100px">Valid To</th>
            <th style="width:40px"></th>
          </tr>
        </thead>
        <tbody id="price-list-body"></tbody>
      </table>
    </div>
  `;
}

// ─────────────────────────────────────────────────────────────────────────────
// Price list row management
// ─────────────────────────────────────────────────────────────────────────────

function renderPriceRows() {
  const tbody = document.getElementById('price-list-body');
  if (!tbody) return;
  if (_priceRows.length === 0) {
    tbody.innerHTML = `<tr><td colspan="6" style="text-align:center;color:var(--text-muted);padding:1rem">No price rules — add row to set customer-specific pricing</td></tr>`;
    return;
  }
  tbody.innerHTML = _priceRows.map((row, i) => `
    <tr data-idx="${i}">
      <td><input class="input input-sm" value="${row.customer_name||''}" placeholder="All customers" readonly style="background:var(--surface-alt);cursor:default"></td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" value="${row.min_qty||1}" data-field="min_qty" data-idx="${i}"></td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" value="${row.price||0}" data-field="price" data-idx="${i}"></td>
      <td><input class="input input-sm" type="date" value="${row.valid_from||''}" data-field="valid_from" data-idx="${i}"></td>
      <td><input class="input input-sm" type="date" value="${row.valid_to||''}" data-field="valid_to" data-idx="${i}"></td>
      <td>
        <button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}" title="Remove">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/></svg>
        </button>
      </td>
    </tr>
  `).join('');

  tbody.querySelectorAll('[data-field]').forEach(inp => {
    inp.addEventListener('change', () => {
      const idx   = parseInt(inp.dataset.idx);
      const field = inp.dataset.field;
      _priceRows[idx][field] = inp.type === 'number' ? parseFloat(inp.value) || 0 : inp.value;
    });
  });

  tbody.querySelectorAll('[data-remove]').forEach(btn => {
    btn.addEventListener('click', () => {
      _priceRows.splice(parseInt(btn.dataset.remove), 1);
      renderPriceRows();
    });
  });
}

function addPriceRow() {
  _priceRows.push({ min_qty: 1, price: 0, valid_from: '', valid_to: '', customer_id: null, customer_name: '' });
  renderPriceRows();
}

// ─────────────────────────────────────────────────────────────────────────────
// Load stock by location (edit mode only)
// ─────────────────────────────────────────────────────────────────────────────

async function loadStockByLocation(itemId) {
  const el = document.getElementById('stock-by-location');
  if (!el) return;
  try {
    const res = await fetch(`/api/masters/items/${itemId}/stock`).then(r => r.json());
    if (!res || res.length === 0) {
      el.innerHTML = `<p style="color:var(--text-muted);font-size:0.85rem">No stock recorded yet.</p>`;
      return;
    }
    el.innerHTML = `
      <table class="table" style="font-size:0.85rem">
        <thead><tr><th>Location</th><th style="text-align:right">Qty On Hand</th><th style="text-align:right">Reserved</th><th style="text-align:right">Available</th></tr></thead>
        <tbody>
          ${res.map(r => `
            <tr>
              <td>${r.location_name || r.location_id}</td>
              <td style="text-align:right">${(r.qty_on_hand||0).toFixed(2)}</td>
              <td style="text-align:right">${(r.qty_reserved||0).toFixed(2)}</td>
              <td style="text-align:right;font-weight:600">${((r.qty_on_hand||0)-(r.qty_reserved||0)).toFixed(2)}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    `;
  } catch {
    el.innerHTML = `<p style="color:var(--danger);font-size:0.85rem">Failed to load stock data.</p>`;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation schema
// ─────────────────────────────────────────────────────────────────────────────

const SCHEMA = {
  'itm-code': { required: true, minLength: 1, maxLength: 30 },
  'itm-name': { required: true, minLength: 2, maxLength: 150 },
};

// ─────────────────────────────────────────────────────────────────────────────
// Collect form
// ─────────────────────────────────────────────────────────────────────────────

function collectForm() {
  const g  = id => document.getElementById(id)?.value?.trim() || null;
  const gn = id => parseFloat(document.getElementById(id)?.value) || 0;
  return {
    code:                 g('itm-code'),
    name:                 g('itm-name'),
    description:          g('itm-desc') || undefined,
    item_type:            g('itm-type') || 'product',
    uom_id:               g('itm-uom')  || undefined,
    purchase_uom_id:      g('itm-purchase-uom')  || undefined,
    sale_uom_id:          g('itm-sale-uom')       || undefined,
    purchase_account_id:  g('itm-purchase-acc')   || undefined,
    sale_account_id:      g('itm-sale-acc')        || undefined,
    inventory_account_id: g('itm-inventory-acc')  || undefined,
    purchase_tax_id:      g('itm-purchase-tax')   || undefined,
    sale_tax_id:          g('itm-sale-tax')        || undefined,
    standard_cost:        gn('itm-std-cost'),
    standard_price:       gn('itm-std-price'),
    min_stock:            gn('itm-min-stock'),
    max_stock:            gn('itm-max-stock'),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Open modal
// ─────────────────────────────────────────────────────────────────────────────

async function openModal(editData = null) {
  _editId    = editData?.id || null;
  _priceRows = [];

  if (_modal) _modal.close();
  _modal = new Modal({
    title:   _editId ? 'Edit Item' : 'New Item',
    size:    'xl',
    content: buildForm(editData || {}),
  });
  _modal.open();

  // Tabs
  _modal.getBody().querySelectorAll('.tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      _modal.getBody().querySelectorAll('.tab-btn,.tab-panel').forEach(el => el.classList.remove('active'));
      btn.classList.add('active');
      _modal.getBody().querySelector(`.tab-panel[data-tab="${btn.dataset.tab}"]`)?.classList.add('active');
    });
  });

  // Load price list if editing
  if (_editId) {
    loadStockByLocation(_editId);
    try {
      const prices = await fetch(`/api/masters/items/${_editId}/prices`).then(r => r.json());
      _priceRows = prices || [];
    } catch { _priceRows = []; }
  }
  renderPriceRows();

  document.getElementById('add-price-row-btn')?.addEventListener('click', addPriceRow);

  _modal.setFooter(`
    <button class="btn btn-secondary" id="itm-cancel">Cancel</button>
    <button class="btn btn-primary" id="itm-save">${_editId ? 'Save Changes' : 'Create Item'}</button>
  `);
  _modal.getFooter().querySelector('#itm-cancel').addEventListener('click', () => _modal.close());
  _modal.getFooter().querySelector('#itm-save').addEventListener('click', saveItem);
}

// ─────────────────────────────────────────────────────────────────────────────
// Save
// ─────────────────────────────────────────────────────────────────────────────

async function saveItem() {
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

  const btn = document.getElementById('itm-save');
  btn.classList.add('loading'); btn.disabled = true;

  try {
    let itemId = _editId;
    if (_editId) {
      await MastersApi.items.update(_editId, data);
      Toast.success('Item updated.');
    } else {
      const res = await MastersApi.items.create(data);
      itemId = res.id;
      Toast.success('Item created.');
    }

    // Save price rows (add/update each)
    for (const row of _priceRows) {
      if (row.id) {
        // existing — skip for now (no update endpoint), handled by backend upsert
      } else {
        const pricePayload = {
          item_id:     itemId,
          customer_id: row.customer_id || null,
          min_qty:     row.min_qty || 1,
          price:       row.price   || 0,
          valid_from:  row.valid_from || null,
          valid_to:    row.valid_to   || null,
        };
        await fetch('/api/masters/items/prices', {
          method:  'POST',
          headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${sessionStorage.getItem('token')}` },
          body:    JSON.stringify(pricePayload),
        }).catch(() => null);
      }
    }

    _modal.close();
    await loadTable();
  } catch (err) {
    Toast.error(err.message || 'Save failed.');
  } finally {
    btn.classList.remove('loading'); btn.disabled = false;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Load table
// ─────────────────────────────────────────────────────────────────────────────

async function loadTable() {
  try {
    const res = await MastersApi.items.list({ active_only: false, limit: 500 });
    _table?.setData(res.items || []);
  } catch {
    Toast.error('Failed to load items.');
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Items</h1>
        <p class="page-subtitle">Manage products, raw materials and services</p>
      </div>
      <button class="btn btn-primary" id="add-item-btn">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        New Item
      </button>
    </div>
    <div id="items-table"></div>
  `;

  await loadRefData();

  _table = new DataTable({
    el: '#items-table',
    columns: [
      { key: 'code',           label: 'Code',         width: '110px' },
      { key: 'name',           label: 'Item Name' },
      { key: 'item_type',      label: 'Type',         width: '110px',
        render: v => {
          const map = { product:'Product', raw_material:'Raw Material', service:'Service' };
          const col = { product:'info', raw_material:'warning', service:'default' };
          return Badge(map[v]||v, col[v]||'default', true);
        }
      },
      { key: 'uom_code',       label: 'UOM',          width: '70px' },
      { key: 'standard_cost',  label: 'Std Cost',     align: 'right', render: v => formatAmount(v, 'PKR') },
      { key: 'standard_price', label: 'Std Price',    align: 'right', render: v => formatAmount(v, 'PKR') },
      { key: 'min_stock',      label: 'Min Stock',    align: 'right', render: v => (v||0).toFixed(2) },
      { key: 'is_active',      label: 'Status',       width: '80px',
        render: v => v ? Badge('Active','success',true) : Badge('Inactive','default',true) },
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
          const detail = await MastersApi.items.get(row.id);
          openModal(detail);
        } catch { Toast.error('Failed to load item.'); }
      }
      if (action === 'delete') {
        const ok = await Confirm({
          title:       'Delete Item?',
          message:     `"${row.name}" will be deactivated.`,
          type:        'danger',
          confirmText: 'Delete',
        });
        if (ok) {
          try {
            await MastersApi.items.delete(row.id);
            Toast.success('Item deleted.');
            await loadTable();
          } catch (e) { Toast.error(e.message); }
        }
      }
    },
  });

  document.getElementById('add-item-btn').addEventListener('click', () => openModal());
  await loadTable();
}

export default { init };
