/**
 * so.js — Sale Orders module
 */

'use strict';

import { SalesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate, FormGroup, SelectGroup,
} from '../../components/components.js';

let _table     = null;
let _modal     = null;
let _editId    = null;
let _lines     = [];
let _customers = [];
let _items     = [];
let _uoms      = [];
let _taxes     = [];
let _currs     = [];
let _terms     = [];
let _locs      = [];

async function loadRefData() {
  const [custs, items, uoms, taxes, currs, terms, locs] = await Promise.allSettled([
    MastersApi.customers.list({ active_only: true, limit: 500 }),
    MastersApi.items.dropdown(),
    MastersApi.uom.list(),
    fetch('/api/masters/tax?applies_to=sales').then(r => r.json()).catch(() => []),
    MastersApi.currencies.list(),
    MastersApi.paymentTerms.list(),
    MastersApi.locations.list(),
  ]);
  _customers = custs.value?.items || custs.value || [];
  _items     = items.value || [];
  _uoms      = uoms.value  || [];
  _taxes     = taxes.value || [];
  _currs     = currs.value || [];
  _terms     = terms.value || [];
  _locs      = locs.value  || [];
}

function calcLine(ln) {
  const gross = (ln.qty||0)*(ln.unit_price||0);
  const disc  = gross*(ln.discount_pct||0)/100;
  const net   = gross-disc;
  const taxR  = _taxes.find(t => t.id===ln.tax_id);
  return net + (taxR ? net*taxR.rate/100 : 0);
}

function renderLines() {
  const tbody = document.getElementById('so-lines-body');
  if (!tbody) return;
  if (_lines.length === 0) {
    tbody.innerHTML = `<tr><td colspan="7" style="text-align:center;color:var(--text-muted);padding:1rem">Add lines</td></tr>`;
    updateTotals(); return;
  }
  tbody.innerHTML = _lines.map((ln,i) => `
    <tr>
      <td style="min-width:180px">
        <select class="input input-sm" data-field="item_id" data-idx="${i}">
          <option value="">Select…</option>
          ${_items.map(it => `<option value="${it.id}" data-price="${it.standard_price||0}" ${it.id===ln.item_id?'selected':''}>${it.code} — ${it.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:75px"><input class="input input-sm" type="number" min="0.01" step="0.01" value="${ln.qty||1}" data-field="qty" data-idx="${i}"></td>
      <td style="width:100px"><input class="input input-sm" type="number" min="0" step="0.01" value="${ln.unit_price||0}" data-field="unit_price" data-idx="${i}"></td>
      <td style="width:65px"><input class="input input-sm" type="number" min="0" max="100" step="0.01" value="${ln.discount_pct||0}" data-field="discount_pct" data-idx="${i}"></td>
      <td style="width:110px">
        <select class="input input-sm" data-field="tax_id" data-idx="${i}">
          <option value="">No Tax</option>
          ${_taxes.map(t => `<option value="${t.id}" ${t.id===ln.tax_id?'selected':''}>${t.name}</option>`).join('')}
        </select>
      </td>
      <td style="width:100px;text-align:right;font-size:0.82rem">${formatAmount(calcLine(ln),'PKR')}</td>
      <td style="width:36px"><button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button></td>
    </tr>
  `).join('');

  tbody.querySelectorAll('[data-field]').forEach(el => {
    el.addEventListener('change', () => {
      const idx=parseInt(el.dataset.idx); const f=el.dataset.field;
      if (f==='item_id') { const opt=el.selectedOptions[0]; _lines[idx].item_id=el.value; _lines[idx].unit_price=parseFloat(opt?.dataset.price||0); renderLines(); return; }
      _lines[idx][f]=['qty','unit_price','discount_pct'].includes(f)?parseFloat(el.value)||0:el.value;
      renderLines();
    });
  });
  tbody.querySelectorAll('[data-remove]').forEach(btn => { btn.addEventListener('click',()=>{ _lines.splice(parseInt(btn.dataset.remove),1); renderLines(); }); });
  updateTotals();
}

function updateTotals() {
  const subtotal = _lines.reduce((s,ln)=>s+(ln.qty||0)*(ln.unit_price||0),0);
  const discount = _lines.reduce((s,ln)=>s+(ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100,0);
  const tax = _lines.reduce((s,ln)=>{
    const net=(ln.qty||0)*(ln.unit_price||0)-(ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100;
    const r=_taxes.find(t=>t.id===ln.tax_id); return s+(r?net*r.rate/100:0);
  },0);
  const grand=subtotal-discount+tax;
  ['so-subtotal','so-discount','so-tax','so-grand'].forEach((id,i)=>{
    const el=document.getElementById(id); if (el) el.textContent=formatAmount([subtotal,discount,tax,grand][i],'PKR');
  });
}

async function openModal(editData=null) {
  _editId=editData?.id||null;
  _lines=editData?.lines?.map(l=>({item_id:l.item_id,qty:l.qty,unit_price:l.unit_price,discount_pct:l.discount_pct||0,tax_id:l.tax_id||''})) || [];
  const today=new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();
  _modal=new Modal({
    title:_editId?`Edit SO — ${editData.so_number}`:'New Sale Order', size:'xl',
    content:`
      <div class="tabs"><button class="tab-btn active" data-tab="header">Header</button><button class="tab-btn" data-tab="lines">Lines</button></div>
      <div class="tab-panel active" data-tab="header" style="padding-top:1rem">
        <div class="grid-2">
          <div class="form-group"><label class="form-label required">Customer</label>
            <select class="input" id="so-customer" ${_editId?'disabled':''}>
              <option value="">Select…</option>
              ${_customers.map(c=>`<option value="${c.id}" ${c.id===editData?.customer_id?'selected':''}>${c.code} — ${c.name}</option>`).join('')}
            </select>
          </div>
          <div class="form-group"><label class="form-label required">SO Date</label><input class="input" type="date" id="so-date" value="${editData?.so_date||today}"></div>
          <div class="form-group"><label class="form-label">Delivery Date</label><input class="input" type="date" id="so-delivery" value="${editData?.delivery_date||''}"></div>
          <div class="form-group"><label class="form-label">Currency</label>
            <select class="input" id="so-currency"><option value="">PKR (default)</option>${_currs.map(c=>`<option value="${c.id}" ${c.id===editData?.currency_id?'selected':''}>${c.code}</option>`).join('')}</select>
          </div>
          <div class="form-group"><label class="form-label">Payment Terms</label>
            <select class="input" id="so-terms"><option value="">None</option>${_terms.map(t=>`<option value="${t.id}" ${t.id===editData?.payment_term_id?'selected':''}>${t.name}</option>`).join('')}</select>
          </div>
          <div class="form-group"><label class="form-label">Location</label>
            <select class="input" id="so-location"><option value="">Default</option>${_locs.map(l=>`<option value="${l.id}">${l.name}</option>`).join('')}</select>
          </div>
        </div>
        <div class="form-group"><label class="form-label">Notes</label><textarea class="input" id="so-notes" rows="2">${editData?.notes||''}</textarea></div>
      </div>
      <div class="tab-panel" data-tab="lines" style="padding-top:1rem">
        <div style="display:flex;justify-content:flex-end;margin-bottom:0.75rem"><button class="btn btn-sm btn-secondary" id="so-add-line">+ Add Line</button></div>
        <div style="overflow-x:auto">
          <table class="table line-table" style="min-width:680px">
            <thead><tr><th>Item</th><th>Qty</th><th>Unit Price</th><th>Disc%</th><th>Tax</th><th style="text-align:right">Total</th><th></th></tr></thead>
            <tbody id="so-lines-body"></tbody>
          </table>
        </div>
        <div style="display:flex;justify-content:flex-end;margin-top:1rem">
          <table style="font-size:0.88rem;min-width:220px">
            <tr><td style="padding:3px 12px 3px 0;color:var(--text-muted)">Subtotal</td><td style="text-align:right" id="so-subtotal">PKR 0.00</td></tr>
            <tr><td style="color:var(--text-muted)">Discount</td><td style="text-align:right" id="so-discount">PKR 0.00</td></tr>
            <tr><td style="color:var(--text-muted)">Tax</td><td style="text-align:right" id="so-tax">PKR 0.00</td></tr>
            <tr style="border-top:2px solid var(--border);font-weight:700"><td style="padding-top:6px">Grand Total</td><td style="text-align:right;padding-top:6px" id="so-grand">PKR 0.00</td></tr>
          </table>
        </div>
      </div>
    `,
  });
  _modal.open();
  _modal.getBody().querySelectorAll('.tab-btn').forEach(btn=>{
    btn.addEventListener('click',()=>{
      _modal.getBody().querySelectorAll('.tab-btn,.tab-panel').forEach(el=>el.classList.remove('active'));
      btn.classList.add('active');
      _modal.getBody().querySelector(`.tab-panel[data-tab="${btn.dataset.tab}"]`)?.classList.add('active');
    });
  });
  renderLines();
  document.getElementById('so-add-line')?.addEventListener('click',()=>{ _lines.push({item_id:'',qty:1,unit_price:0,discount_pct:0,tax_id:''}); renderLines(); });
  _modal.setFooter(`<button class="btn btn-secondary" id="so-cancel">Cancel</button><button class="btn btn-primary" id="so-save">${_editId?'Save':'Create SO'}</button>`);
  _modal.getFooter().querySelector('#so-cancel').addEventListener('click',()=>_modal.close());
  _modal.getFooter().querySelector('#so-save').addEventListener('click',saveSO);
}

async function saveSO() {
  const g=id=>document.getElementById(id)?.value?.trim()||null;
  if (!g('so-customer')){ Toast.warning('Select a customer.'); return; }
  if (!g('so-date'))    { Toast.warning('SO date required.'); return; }
  if (!_lines.length)   { Toast.warning('Add at least one line.'); return; }
  if (_lines.some(l=>!l.item_id)){ Toast.warning('All lines need an item.'); return; }
  const payload={ customer_id:g('so-customer'), so_date:g('so-date'), delivery_date:g('so-delivery')||undefined, currency_id:g('so-currency')||undefined, payment_term_id:g('so-terms')||undefined, location_id:g('so-location')||undefined, notes:g('so-notes')||undefined,
    lines:_lines.map(l=>({item_id:l.item_id,qty:l.qty,unit_price:l.unit_price,discount_pct:l.discount_pct||0,tax_id:l.tax_id||undefined})) };
  const btn=document.getElementById('so-save'); btn.classList.add('loading'); btn.disabled=true;
  try {
    if (_editId){ await SalesApi.so.update(_editId,payload); Toast.success('SO updated.'); }
    else { const r=await SalesApi.so.create(payload); Toast.success(`SO created: ${r.number}`); }
    _modal.close(); await loadTable();
  } catch(e){ Toast.error(e.message); } finally { btn.classList.remove('loading'); btn.disabled=false; }
}

function actionBtns(row) {
  const b=[]; const s=row.status;
  if (s==='draft') { b.push(`<button class="btn btn-sm btn-secondary btn-icon" data-action="edit" title="Edit"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4z"/></svg></button>`); b.push(`<button class="btn btn-sm btn-primary" data-action="submit" style="font-size:0.78rem;padding:3px 10px">Submit</button>`); b.push(`<button class="btn btn-sm btn-ghost-danger btn-icon" data-action="delete"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/></svg></button>`); }
  if (s==='submitted') { b.push(`<button class="btn btn-sm btn-success" data-action="approve" style="font-size:0.78rem;padding:3px 10px">Approve</button>`); b.push(`<button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>`); }
  if (!b.length) b.push(`<button class="btn btn-sm btn-secondary btn-icon" data-action="view"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg></button>`);
  return `<div class="row-actions">${b.join('')}</div>`;
}

async function loadTable() {
  try { const r=await SalesApi.so.list({limit:200}); _table?.setData(r.items||[]); } catch { Toast.error('Failed to load sale orders.'); }
}

export async function init(container) {
  container.innerHTML=`
    <div class="page-header">
      <div><h1 class="page-title">Sale Orders</h1><p class="page-subtitle">Manage customer sale orders</p></div>
      <button class="btn btn-primary" id="add-so-btn"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg> New SO</button>
    </div>
    <div id="so-table"></div>
  `;
  await loadRefData();
  _table=new DataTable({
    el:'#so-table',
    columns:[
      {key:'so_number',label:'SO #',width:'120px'},
      {key:'customer_name',label:'Customer'},
      {key:'so_date',label:'Date',width:'100px',render:formatDate},
      {key:'delivery_date',label:'Delivery',width:'100px',render:v=>v?formatDate(v):'—'},
      {key:'grand_total',label:'Total',align:'right',render:v=>formatAmount(v,'PKR')},
      {key:'status',label:'Status',width:'100px',render:v=>StatusBadge(v)},
      {key:'_actions',label:'',sortable:false,render:(_,row)=>actionBtns(row)},
    ],
    onRowAction:async(action,row)=>{
      if (action==='edit'||action==='view'){ try { const d=await SalesApi.so.get(row.id); if(action==='edit') openModal(d); else showDetail(d); } catch { Toast.error('Failed to load SO.'); } return; }
      if (action==='submit'){ try{ await SalesApi.so.submit(row.id); Toast.success('SO submitted.'); await loadTable(); }catch(e){Toast.error(e.message);} return; }
      if (action==='approve'){ const ok=await Confirm({title:'Approve SO?',message:`Approve "${row.so_number}"?`,type:'success',confirmText:'Approve'}); if(ok){ try{await SalesApi.so.approve(row.id); Toast.success('SO approved.'); await loadTable();}catch(e){Toast.error(e.message);} } return; }
      if (action==='cancel'){ const ok=await Confirm({title:'Cancel SO?',message:`Cancel "${row.so_number}"?`,type:'danger',confirmText:'Cancel'}); if(ok){ try{await SalesApi.so.cancel(row.id); Toast.success('Cancelled.'); await loadTable();}catch(e){Toast.error(e.message);} } return; }
      if (action==='delete'){ const ok=await Confirm({title:'Delete SO?',message:`Delete "${row.so_number}"?`,type:'danger',confirmText:'Delete'}); if(ok){ try{await SalesApi.so.delete(row.id); Toast.success('Deleted.'); await loadTable();}catch(e){Toast.error(e.message);} } }
    },
  });
  document.getElementById('add-so-btn').addEventListener('click',()=>openModal());
  await loadTable();
}

function showDetail(data) {
  const m=new Modal({title:`SO ${data.so_number} — ${data.customer_name}`,size:'xl',content:`
    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:1rem;margin-bottom:1.5rem">
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Date</span><br>${formatDate(data.so_date)}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Grand Total</span><br><strong>${formatAmount(data.grand_total,'PKR')}</strong></div>
    </div>
    <table class="table" style="font-size:0.85rem">
      <thead><tr><th>#</th><th>Item</th><th style="text-align:right">Qty</th><th style="text-align:right">Price</th><th style="text-align:right">Disc%</th><th style="text-align:right">Total</th><th style="text-align:right">Delivered</th></tr></thead>
      <tbody>${(data.lines||[]).map((l,i)=>`<tr><td>${i+1}</td><td>${l.item_code} — ${l.item_name}</td><td style="text-align:right">${l.qty}</td><td style="text-align:right">${formatAmount(l.unit_price,'PKR')}</td><td style="text-align:right">${l.discount_pct||0}%</td><td style="text-align:right">${formatAmount(l.line_total,'PKR')}</td><td style="text-align:right">${l.qty_delivered||0}</td></tr>`).join('')}</tbody>
    </table>
  `});
  m.open();
  m.setFooter(`<button class="btn btn-secondary" id="so-det-close">Close</button>`);
  m.getFooter().querySelector('#so-det-close').addEventListener('click',()=>m.close());
}

export default { init };
