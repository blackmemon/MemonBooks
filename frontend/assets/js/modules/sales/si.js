/**
 * si.js — Sale Invoices module
 */

'use strict';

import { SalesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate, FormGroup,
} from '../../components/components.js';

let _table     = null;
let _modal     = null;
let _customers = [];
let _items     = [];
let _taxes     = [];
let _terms     = [];
let _currs     = [];
let _lines     = [];

async function loadRefData() {
  const [custs, items, taxes, terms, currs] = await Promise.allSettled([
    MastersApi.customers.list({ active_only: true, limit: 500 }),
    MastersApi.items.dropdown(),
    fetch('/api/masters/tax?applies_to=sales').then(r=>r.json()).catch(()=>[]),
    MastersApi.paymentTerms.list(),
    MastersApi.currencies.list(),
  ]);
  _customers = custs.value?.items || custs.value || [];
  _items     = items.value  || [];
  _taxes     = taxes.value  || [];
  _terms     = terms.value  || [];
  _currs     = currs.value  || [];
}

function calcLine(ln) {
  const gross=(ln.qty||0)*(ln.unit_price||0);
  const disc=gross*(ln.discount_pct||0)/100;
  const net=gross-disc;
  const taxR=_taxes.find(t=>t.id===ln.tax_id);
  const taxAmt=taxR?net*taxR.rate/100:0;
  return {net, taxAmt, total: net+taxAmt};
}

function renderLines() {
  const tbody=document.getElementById('si-lines-body');
  if (!tbody) return;
  if (!_lines.length) { tbody.innerHTML=`<tr><td colspan="7" style="text-align:center;color:var(--text-muted);padding:1rem">Add lines</td></tr>`; updateTotals(); return; }
  tbody.innerHTML=_lines.map((ln,i)=>`
    <tr>
      <td style="min-width:170px"><select class="input input-sm" data-field="item_id" data-idx="${i}"><option value="">Select…</option>${_items.map(it=>`<option value="${it.id}" data-price="${it.standard_price||0}" ${it.id===ln.item_id?'selected':''}>${it.code} — ${it.name}</option>`).join('')}</select></td>
      <td style="width:75px"><input class="input input-sm" type="number" min="0.01" step="0.01" value="${ln.qty||1}" data-field="qty" data-idx="${i}"></td>
      <td style="width:100px"><input class="input input-sm" type="number" min="0" step="0.01" value="${ln.unit_price||0}" data-field="unit_price" data-idx="${i}"></td>
      <td style="width:65px"><input class="input input-sm" type="number" min="0" max="100" step="0.01" value="${ln.discount_pct||0}" data-field="discount_pct" data-idx="${i}"></td>
      <td style="width:110px"><select class="input input-sm" data-field="tax_id" data-idx="${i}"><option value="">No Tax</option>${_taxes.map(t=>`<option value="${t.id}" ${t.id===ln.tax_id?'selected':''}>${t.name}</option>`).join('')}</select></td>
      <td style="width:100px;text-align:right;font-size:0.82rem">${formatAmount(calcLine(ln).total,'PKR')}</td>
      <td style="width:36px"><button class="btn btn-sm btn-ghost-danger btn-icon" data-remove="${i}"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button></td>
    </tr>
  `).join('');
  tbody.querySelectorAll('[data-field]').forEach(el=>{
    el.addEventListener('change',()=>{
      const idx=parseInt(el.dataset.idx); const f=el.dataset.field;
      if(f==='item_id'){const opt=el.selectedOptions[0]; _lines[idx].item_id=el.value; _lines[idx].unit_price=parseFloat(opt?.dataset.price||0); renderLines(); return;}
      _lines[idx][f]=['qty','unit_price','discount_pct'].includes(f)?parseFloat(el.value)||0:el.value;
      renderLines();
    });
  });
  tbody.querySelectorAll('[data-remove]').forEach(btn=>{btn.addEventListener('click',()=>{_lines.splice(parseInt(btn.dataset.remove),1); renderLines();});});
  updateTotals();
}

function updateTotals() {
  let sub=0,disc=0,tax=0;
  for(const ln of _lines){ const c=calcLine(ln); sub+=(ln.qty||0)*(ln.unit_price||0); disc+=(ln.qty||0)*(ln.unit_price||0)*(ln.discount_pct||0)/100; tax+=c.taxAmt; }
  const grand=sub-disc+tax;
  const s=(id,v)=>{const el=document.getElementById(id); if(el) el.textContent=formatAmount(v,'PKR');};
  s('si-subtotal',sub); s('si-disc',disc); s('si-tax',tax); s('si-grand',grand);
}

async function openModal() {
  _lines=[];
  const today=new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();
  _modal=new Modal({title:'New Sale Invoice',size:'xl',content:`
    <div class="grid-2" style="padding-top:0.5rem">
      <div class="form-group"><label class="form-label required">Customer</label>
        <select class="input" id="si-customer"><option value="">Select…</option>${_customers.map(c=>`<option value="${c.id}">${c.code} — ${c.name}</option>`).join('')}</select>
      </div>
      <div class="form-group"><label class="form-label required">Invoice Date</label><input class="input" type="date" id="si-date" value="${today}"></div>
      <div class="form-group"><label class="form-label">Due Date</label><input class="input" type="date" id="si-due-date"></div>
      <div class="form-group"><label class="form-label">Payment Terms</label>
        <select class="input" id="si-terms"><option value="">None</option>${_terms.map(t=>`<option value="${t.id}">${t.name}</option>`).join('')}</select>
      </div>
      <div class="form-group"><label class="form-label">Currency</label>
        <select class="input" id="si-currency"><option value="">PKR (default)</option>${_currs.map(c=>`<option value="${c.id}">${c.code}</option>`).join('')}</select>
      </div>
    </div>
    <div class="form-group"><label class="form-label">Notes</label><textarea class="input" id="si-notes" rows="2" placeholder="Optional"></textarea></div>
    <div style="display:flex;justify-content:space-between;align-items:center;margin:1rem 0 0.5rem">
      <strong style="font-size:0.9rem">Invoice Lines</strong>
      <button class="btn btn-sm btn-secondary" id="si-add-line">+ Add Line</button>
    </div>
    <div style="overflow-x:auto">
      <table class="table line-table" style="min-width:680px">
        <thead><tr><th>Item</th><th>Qty</th><th>Unit Price</th><th>Disc%</th><th>Tax</th><th style="text-align:right">Total</th><th></th></tr></thead>
        <tbody id="si-lines-body"></tbody>
      </table>
    </div>
    <div style="display:flex;justify-content:flex-end;margin-top:1rem">
      <table style="font-size:0.88rem;min-width:220px">
        <tr><td style="padding:3px 12px 3px 0;color:var(--text-muted)">Subtotal</td><td style="text-align:right" id="si-subtotal">PKR 0.00</td></tr>
        <tr><td style="color:var(--text-muted)">Discount</td><td style="text-align:right" id="si-disc">PKR 0.00</td></tr>
        <tr><td style="color:var(--text-muted)">Tax</td><td style="text-align:right" id="si-tax">PKR 0.00</td></tr>
        <tr style="border-top:2px solid var(--border);font-weight:700"><td style="padding-top:6px">Grand Total</td><td style="text-align:right;padding-top:6px" id="si-grand">PKR 0.00</td></tr>
      </table>
    </div>
  `});
  _modal.open();
  document.getElementById('si-add-line')?.addEventListener('click',()=>{ _lines.push({item_id:'',qty:1,unit_price:0,discount_pct:0,tax_id:''}); renderLines(); });
  renderLines();
  _modal.setFooter(`<button class="btn btn-secondary" id="si-cancel">Cancel</button><button class="btn btn-primary" id="si-save">Create Invoice</button>`);
  _modal.getFooter().querySelector('#si-cancel').addEventListener('click',()=>_modal.close());
  _modal.getFooter().querySelector('#si-save').addEventListener('click',saveSI);
}

async function saveSI() {
  const g=id=>document.getElementById(id)?.value?.trim()||null;
  if(!g('si-customer')){Toast.warning('Select a customer.');return;}
  if(!g('si-date')){Toast.warning('Invoice date required.');return;}
  if(!_lines.length){Toast.warning('Add at least one line.');return;}
  if(_lines.some(l=>!l.item_id)){Toast.warning('All lines need an item.');return;}
  const payload={customer_id:g('si-customer'),invoice_date:g('si-date'),due_date:g('si-due-date')||undefined,payment_term_id:g('si-terms')||undefined,currency_id:g('si-currency')||undefined,notes:g('si-notes')||undefined,
    lines:_lines.map(l=>({item_id:l.item_id,qty:l.qty,unit_price:l.unit_price,discount_pct:l.discount_pct||0,tax_id:l.tax_id||undefined}))};
  const btn=document.getElementById('si-save'); btn.classList.add('loading'); btn.disabled=true;
  try{const r=await SalesApi.si.create(payload); Toast.success(`Invoice created: ${r.number}`); _modal.close(); await loadTable();}
  catch(e){Toast.error(e.message);} finally{btn.classList.remove('loading'); btn.disabled=false;}
}

function showDetail(data) {
  const m=new Modal({title:`SI ${data.si_number} — ${data.customer_name}`,size:'xl',content:`
    <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:1rem;margin-bottom:1.5rem">
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Invoice Date</span><br>${formatDate(data.invoice_date)}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Due Date</span><br>${data.due_date?formatDate(data.due_date):'—'}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Grand Total</span><br><strong>${formatAmount(data.grand_total,'PKR')}</strong></div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Amount Due</span><br><strong style="color:var(--danger)">${formatAmount(data.amount_due,'PKR')}</strong></div>
    </div>
    <table class="table" style="font-size:0.85rem">
      <thead><tr><th>#</th><th>Item</th><th style="text-align:right">Qty</th><th style="text-align:right">Price</th><th style="text-align:right">Disc%</th><th style="text-align:right">Tax</th><th style="text-align:right">Total</th></tr></thead>
      <tbody>${(data.lines||[]).map((l,i)=>`<tr><td>${i+1}</td><td>${l.item_code} — ${l.item_name}</td><td style="text-align:right">${l.qty}</td><td style="text-align:right">${formatAmount(l.unit_price,'PKR')}</td><td style="text-align:right">${l.discount_pct||0}%</td><td style="text-align:right">${formatAmount(l.tax_amt,'PKR')}</td><td style="text-align:right">${formatAmount(l.line_total,'PKR')}</td></tr>`).join('')}</tbody>
    </table>
  `});
  m.open();
  const footer=`<button class="btn btn-secondary" id="si-det-close">Close</button>${data.status==='draft'?`<button class="btn btn-primary" id="si-det-post">Post Invoice</button>`:''}`;
  m.setFooter(footer);
  m.getFooter().querySelector('#si-det-close').addEventListener('click',()=>m.close());
  m.getFooter().querySelector('#si-det-post')?.addEventListener('click',async()=>{
    const ok=await Confirm({title:'Post Invoice?',message:'Creates JV and locks invoice.',type:'primary',confirmText:'Post'});
    if(ok){try{await SalesApi.si.post(data.id); Toast.success('Invoice posted.'); m.close(); await loadTable();}catch(e){Toast.error(e.message);}}
  });
}

async function loadTable() {
  try{const r=await SalesApi.si.list({limit:200}); _table?.setData(r.items||[]);}catch{Toast.error('Failed to load sale invoices.');}
}

export async function init(container) {
  container.innerHTML=`
    <div class="page-header">
      <div><h1 class="page-title">Sale Invoices</h1><p class="page-subtitle">Customer invoices with auto journal voucher</p></div>
      <button class="btn btn-primary" id="add-si-btn"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg> New Invoice</button>
    </div>
    <div id="si-table"></div>
  `;
  await loadRefData();
  _table=new DataTable({
    el:'#si-table',
    columns:[
      {key:'si_number',label:'SI #',width:'120px'},
      {key:'customer_name',label:'Customer'},
      {key:'invoice_date',label:'Date',width:'100px',render:formatDate},
      {key:'due_date',label:'Due',width:'100px',render:v=>v?formatDate(v):'—'},
      {key:'grand_total',label:'Total',align:'right',render:v=>formatAmount(v,'PKR')},
      {key:'amount_due',label:'Due Amt',align:'right',render:v=>formatAmount(v,'PKR')},
      {key:'status',label:'Status',width:'90px',render:v=>StatusBadge(v)},
      {key:'_actions',label:'',sortable:false,render:(_,row)=>`
        <div class="row-actions">
          <button class="btn btn-sm btn-secondary btn-icon" data-action="view"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg></button>
          ${row.status==='draft'?`<button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button><button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>`:''}
        </div>
      `},
    ],
    onRowAction:async(action,row)=>{
      if(action==='view'){try{const d=await SalesApi.si.get(row.id); showDetail(d);}catch{Toast.error('Failed.');} return;}
      if(action==='post'){const ok=await Confirm({title:'Post Invoice?',message:'Creates JV.',type:'primary',confirmText:'Post'}); if(ok){try{await SalesApi.si.post(row.id); Toast.success('Posted.'); await loadTable();}catch(e){Toast.error(e.message);}} return;}
      if(action==='cancel'){const ok=await Confirm({title:'Cancel Invoice?',message:'Cancel this invoice?',type:'danger',confirmText:'Cancel'}); if(ok){try{await SalesApi.si.cancel(row.id); Toast.success('Cancelled.'); await loadTable();}catch(e){Toast.error(e.message);}}}
    },
  });
  document.getElementById('add-si-btn').addEventListener('click',openModal);
  await loadTable();
}

export default { init };
