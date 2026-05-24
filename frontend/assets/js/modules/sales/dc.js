/**
 * dc.js — Delivery Challans module
 */

'use strict';

import { SalesApi, MastersApi } from '../../core/api.js';
import {
  Toast, Modal, Confirm, DataTable,
  StatusBadge, formatAmount, formatDate,
} from '../../components/components.js';

let _table = null;
let _modal = null;
let _locs  = [];
let _dcLines = [];

async function loadRefData() {
  const [locs] = await Promise.allSettled([MastersApi.locations.list()]);
  _locs = locs.value || [];
}

function locOpts() {
  return `<option value="">Default</option>` + _locs.map(l=>`<option value="${l.id}">${l.name}</option>`).join('');
}

async function openSOSelector() {
  try {
    const res = await SalesApi.so.list({ status: 'approved', limit: 100 });
    const sos  = res.items || [];
    if (!sos.length) { Toast.info('No approved Sale Orders available.'); return; }
    const sel = new Modal({
      title: 'Select Sale Order', size: 'md',
      content: `
        <p style="font-size:0.85rem;color:var(--text-muted);margin-bottom:0.75rem">Select approved SO to create a Delivery Challan against:</p>
        <table class="table" style="font-size:0.85rem">
          <thead><tr><th>SO #</th><th>Customer</th><th>Date</th><th style="text-align:right">Total</th><th></th></tr></thead>
          <tbody>
            ${sos.map(s=>`<tr><td>${s.so_number}</td><td>${s.customer_name}</td><td>${formatDate(s.so_date)}</td><td style="text-align:right">${formatAmount(s.grand_total,'PKR')}</td><td><button class="btn btn-sm btn-primary" data-so-id="${s.id}">Select</button></td></tr>`).join('')}
          </tbody>
        </table>
      `,
    });
    sel.open();
    sel.setFooter(`<button class="btn btn-secondary" id="so-sel-cancel">Cancel</button>`);
    sel.getFooter().querySelector('#so-sel-cancel').addEventListener('click', () => sel.close());
    sel.getBody().querySelectorAll('[data-so-id]').forEach(btn => {
      btn.addEventListener('click', async () => {
        sel.close();
        try { const so = await SalesApi.so.get(btn.dataset.soId); so.id=btn.dataset.soId; openCreateModal(so); }
        catch { Toast.error('Failed to load SO.'); }
      });
    });
  } catch { Toast.error('Failed to load SOs.'); }
}

async function openCreateModal(so) {
  _dcLines = so.lines.map(l=>({
    so_line_id: l.id, item_id: l.item_id, item_code: l.item_code,
    item_name: l.item_name, uom_code: l.uom_code,
    qty_ordered: l.qty, qty_remaining: l.qty-(l.qty_delivered||0),
    qty_delivered: 0, location_id: '',
  }));
  const today = new Date().toISOString().split('T')[0];
  if (_modal) _modal.close();
  _modal = new Modal({
    title: `New DC — SO ${so.so_number}`, size: 'xl',
    content: `
      <div class="grid-2" style="padding-top:0.5rem">
        <div class="form-group"><label class="form-label required">DC Date</label><input class="input" type="date" id="dc-date" value="${today}"></div>
        <div class="form-group"><label class="form-label">Default Location</label><select class="input" id="dc-location">${locOpts()}</select></div>
      </div>
      <div class="form-group"><label class="form-label">Notes</label><textarea class="input" id="dc-notes" rows="2" placeholder="Optional"></textarea></div>
      <div style="overflow-x:auto;margin-top:1rem">
        <table class="table line-table" style="min-width:600px">
          <thead><tr><th>Item</th><th style="width:60px">UOM</th><th style="width:80px;text-align:right">Ordered</th><th style="width:80px;text-align:right">Remaining</th><th style="width:100px">Deliver Qty</th></tr></thead>
          <tbody id="dc-lines-body"></tbody>
        </table>
      </div>
    `,
  });
  _modal.open();
  renderDCLines();
  _modal.setFooter(`<button class="btn btn-secondary" id="dc-cancel">Cancel</button><button class="btn btn-primary" id="dc-save">Create DC</button>`);
  _modal.getFooter().querySelector('#dc-cancel').addEventListener('click',()=>_modal.close());
  _modal.getFooter().querySelector('#dc-save').addEventListener('click',()=>saveDC(so.id));
}

function renderDCLines() {
  const tbody=document.getElementById('dc-lines-body');
  if (!tbody) return;
  tbody.innerHTML=_dcLines.map((ln,i)=>`
    <tr>
      <td>${ln.item_code} — ${ln.item_name}</td>
      <td>${ln.uom_code||'—'}</td>
      <td style="text-align:right">${ln.qty_ordered}</td>
      <td style="text-align:right">${ln.qty_remaining.toFixed(2)}</td>
      <td><input class="input input-sm" type="number" min="0" step="0.01" max="${ln.qty_remaining}" value="${ln.qty_delivered}" data-idx="${i}"></td>
    </tr>
  `).join('');
  tbody.querySelectorAll('[data-idx]').forEach(inp=>{
    inp.addEventListener('change',()=>{ _dcLines[parseInt(inp.dataset.idx)].qty_delivered=parseFloat(inp.value)||0; });
  });
}

async function saveDC(soId) {
  const dcDate=document.getElementById('dc-date')?.value;
  const locId=document.getElementById('dc-location')?.value||null;
  const notes=document.getElementById('dc-notes')?.value?.trim()||null;
  if (!dcDate) { Toast.warning('DC date required.'); return; }
  const lines=_dcLines.filter(l=>l.qty_delivered>0).map(l=>({ so_line_id:l.so_line_id, item_id:l.item_id, qty_delivered:l.qty_delivered, location_id:locId||undefined }));
  if (!lines.length) { Toast.warning('Enter delivered quantity for at least one line.'); return; }
  const btn=document.getElementById('dc-save'); btn.classList.add('loading'); btn.disabled=true;
  try {
    const r=await SalesApi.dc.create({so_id:soId,dc_date:dcDate,location_id:locId,notes,lines});
    Toast.success(`DC created: ${r.number}`); _modal.close(); await loadTable();
  } catch(e){ Toast.error(e.message); } finally { btn.classList.remove('loading'); btn.disabled=false; }
}

function showDetail(data) {
  const m=new Modal({title:`DC ${data.dc_number} — ${data.customer_name}`,size:'xl',content:`
    <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:1rem;margin-bottom:1.5rem">
      <div><span style="font-size:0.8rem;color:var(--text-muted)">SO #</span><br>${data.so_number||'—'}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">DC Date</span><br>${formatDate(data.dc_date)}</div>
      <div><span style="font-size:0.8rem;color:var(--text-muted)">Status</span><br>${StatusBadge(data.status)}</div>
    </div>
    <table class="table" style="font-size:0.85rem">
      <thead><tr><th>#</th><th>Item</th><th>UOM</th><th style="text-align:right">Ordered</th><th style="text-align:right">Delivered</th></tr></thead>
      <tbody>${(data.lines||[]).map((l,i)=>`<tr><td>${i+1}</td><td>${l.item_code} — ${l.item_name}</td><td>${l.uom_code||'—'}</td><td style="text-align:right">${l.qty_ordered||0}</td><td style="text-align:right;font-weight:600">${l.qty_delivered||0}</td></tr>`).join('')}</tbody>
    </table>
    ${data.notes?`<p style="margin-top:1rem;font-size:0.85rem;color:var(--text-muted)"><strong>Notes:</strong> ${data.notes}</p>`:''}
  `});
  m.open();
  const footer=`<button class="btn btn-secondary" id="dc-det-close">Close</button>${data.status==='draft'?`<button class="btn btn-primary" id="dc-det-post">Post (Deduct Stock)</button>`:''}`;
  m.setFooter(footer);
  m.getFooter().querySelector('#dc-det-close').addEventListener('click',()=>m.close());
  m.getFooter().querySelector('#dc-det-post')?.addEventListener('click',async()=>{
    const ok=await Confirm({title:'Post DC?',message:'Stock will be deducted. Cannot be undone.',type:'primary',confirmText:'Post'});
    if(ok){ try{await SalesApi.dc.post(data.id); Toast.success('DC posted. Stock deducted.'); m.close(); await loadTable();}catch(e){Toast.error(e.message);} }
  });
}

async function loadTable() {
  try { const r=await SalesApi.dc.list({limit:200}); _table?.setData(r.items||[]); } catch { Toast.error('Failed to load DCs.'); }
}

export async function init(container) {
  container.innerHTML=`
    <div class="page-header">
      <div><h1 class="page-title">Delivery Challans</h1><p class="page-subtitle">Record goods delivered to customers</p></div>
      <button class="btn btn-primary" id="add-dc-btn"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg> New DC</button>
    </div>
    <div id="dc-table"></div>
  `;
  await loadRefData();
  _table=new DataTable({
    el:'#dc-table',
    columns:[
      {key:'dc_number',label:'DC #',width:'120px'},
      {key:'so_number',label:'SO #',width:'110px',render:v=>v||'—'},
      {key:'customer_name',label:'Customer'},
      {key:'dc_date',label:'Date',width:'100px',render:formatDate},
      {key:'status',label:'Status',width:'100px',render:v=>StatusBadge(v)},
      {key:'_actions',label:'',sortable:false,render:(_,row)=>`
        <div class="row-actions">
          <button class="btn btn-sm btn-secondary btn-icon" data-action="view"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg></button>
          ${row.status==='draft'?`<button class="btn btn-sm btn-primary" data-action="post" style="font-size:0.78rem;padding:3px 10px">Post</button><button class="btn btn-sm btn-ghost-danger" data-action="cancel" style="font-size:0.78rem;padding:3px 10px">Cancel</button>`:''}
        </div>
      `},
    ],
    onRowAction:async(action,row)=>{
      if (action==='view'){ try{const d=await SalesApi.dc.get(row.id); showDetail(d);}catch{Toast.error('Failed to load DC.');} return; }
      if (action==='post'){ const ok=await Confirm({title:'Post DC?',message:'Stock will be deducted.',type:'primary',confirmText:'Post'}); if(ok){try{await SalesApi.dc.post(row.id); Toast.success('DC posted.'); await loadTable();}catch(e){Toast.error(e.message);}} return; }
      if (action==='cancel'){ const ok=await Confirm({title:'Cancel DC?',message:`Cancel "${row.dc_number}"?`,type:'danger',confirmText:'Cancel'}); if(ok){try{await SalesApi.dc.cancel(row.id); Toast.success('Cancelled.'); await loadTable();}catch(e){Toast.error(e.message);}} }
    },
  });
  document.getElementById('add-dc-btn').addEventListener('click',openSOSelector);
  await loadTable();
}

export default { init };
