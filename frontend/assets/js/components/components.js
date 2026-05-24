/**
 * components.js — MemonBook ERP Single-File Component Library
 * Button, Input, Badge, Table, Modal, Toast, Validator, Confirm, Tabs,
 * Select, DateInput, AmountInput, KpiCard, Skeleton, Dropdown
 *
 * ES2020+ — no external dependencies
 * Import everything from this one file.
 */

'use strict';

// ─────────────────────────────────────────────────────────────────────────────
// ICONS — SVG strings (Lucide-style, 18×18)
// ─────────────────────────────────────────────────────────────────────────────

const Icons = {
  check:    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>`,
  x:        `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`,
  info:     `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>`,
  warn:     `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,
  chevronRight: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>`,
  plus:     `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>`,
  trash:    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4h6v2"/></svg>`,
  edit:     `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>`,
  eye:      `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>`,
  sortAsc:  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 5h10M11 9h7M11 13h4M3 17l3 3 3-3M6 20V4"/></svg>`,
  sortDesc: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 5h10M11 9h7M11 13h4M3 7l3-3 3 3M6 4v16"/></svg>`,
  search:   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>`,
  calendar: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>`,
  download: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>`,
  filter:   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/></svg>`,
};


// ─────────────────────────────────────────────────────────────────────────────
// VALIDATOR
// ─────────────────────────────────────────────────────────────────────────────

const Validator = {
  /**
   * Validate a single value against a rule object.
   * Returns { valid: bool, message: string }
   */
  validate(value, rules = {}) {
    const v = (value ?? '').toString().trim();

    if (rules.required && !v) {
      return { valid: false, message: rules.requiredMsg || 'This field is required.' };
    }
    if (!v) return { valid: true, message: '' };

    if (rules.minLength && v.length < rules.minLength) {
      return { valid: false, message: `Minimum ${rules.minLength} characters required.` };
    }
    if (rules.maxLength && v.length > rules.maxLength) {
      return { valid: false, message: `Maximum ${rules.maxLength} characters allowed.` };
    }
    if (rules.min !== undefined && parseFloat(v) < rules.min) {
      return { valid: false, message: `Minimum value is ${rules.min}.` };
    }
    if (rules.max !== undefined && parseFloat(v) > rules.max) {
      return { valid: false, message: `Maximum value is ${rules.max}.` };
    }
    if (rules.type === 'email' && !Validator.isEmail(v)) {
      return { valid: false, message: 'Enter a valid email address.' };
    }
    if (rules.type === 'cnic' && !Validator.isCNIC(v)) {
      return { valid: false, message: 'CNIC format: 12345-6789012-3' };
    }
    if (rules.type === 'iban' && !Validator.isIBAN(v)) {
      return { valid: false, message: 'Enter a valid IBAN (e.g. PK36SCBL0000001123456702).' };
    }
    if (rules.type === 'phone' && !Validator.isPhone(v)) {
      return { valid: false, message: 'Enter a valid phone number.' };
    }
    if (rules.type === 'number' && isNaN(parseFloat(v))) {
      return { valid: false, message: 'Enter a valid number.' };
    }
    if (rules.type === 'amount' && (isNaN(parseFloat(v)) || parseFloat(v) < 0)) {
      return { valid: false, message: 'Enter a valid positive amount.' };
    }
    if (rules.type === 'percent' && (isNaN(parseFloat(v)) || parseFloat(v) < 0 || parseFloat(v) > 100)) {
      return { valid: false, message: 'Enter a value between 0 and 100.' };
    }
    if (rules.pattern && !new RegExp(rules.pattern).test(v)) {
      return { valid: false, message: rules.patternMsg || 'Invalid format.' };
    }
    if (rules.custom) {
      const msg = rules.custom(v);
      if (msg) return { valid: false, message: msg };
    }

    return { valid: true, message: '' };
  },

  isEmail:  (v) => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v),
  isCNIC:   (v) => /^\d{5}-\d{7}-\d$/.test(v),
  isIBAN:   (v) => /^[A-Z]{2}\d{2}[A-Z0-9]{1,30}$/.test(v.replace(/\s/g, '')),
  isPhone:  (v) => /^[+]?[\d\s\-().]{7,20}$/.test(v),
  isNTN:    (v) => /^\d{7}-\d$/.test(v),

  /**
   * Validate an entire form object.
   * @param {Object} data - key: value pairs
   * @param {Object} schema - key: rules pairs
   * @returns { valid: bool, errors: { field: message } }
   */
  validateForm(data, schema) {
    const errors = {};
    for (const [field, rules] of Object.entries(schema)) {
      const result = Validator.validate(data[field], rules);
      if (!result.valid) errors[field] = result.message;
    }
    return { valid: Object.keys(errors).length === 0, errors };
  },

  /** Show error under an input element */
  showError(inputEl, message) {
    inputEl.classList.add('error');
    let el = inputEl.parentElement.querySelector('.form-error');
    if (!el) {
      el = document.createElement('div');
      el.className = 'form-error';
      inputEl.insertAdjacentElement('afterend', el);
    }
    el.innerHTML = `${Icons.info} <span>${message}</span>`;
    el.style.display = 'flex';
  },

  clearError(inputEl) {
    inputEl.classList.remove('error');
    const el = inputEl.parentElement.querySelector('.form-error');
    if (el) el.style.display = 'none';
  },

  /** Attach live validation to an input */
  attach(inputEl, rules) {
    const validate = () => {
      const result = Validator.validate(inputEl.value, rules);
      if (!result.valid) Validator.showError(inputEl, result.message);
      else Validator.clearError(inputEl);
      return result.valid;
    };
    inputEl.addEventListener('blur', validate);
    inputEl.addEventListener('input', () => {
      if (inputEl.classList.contains('error')) validate();
    });
    return validate;
  },
};


// ─────────────────────────────────────────────────────────────────────────────
// TOAST
// ─────────────────────────────────────────────────────────────────────────────

class ToastManager {
  constructor() {
    this.container = this._getOrCreateContainer();
  }

  _getOrCreateContainer() {
    let el = document.getElementById('toast-container');
    if (!el) {
      el = document.createElement('div');
      el.id = 'toast-container';
      el.className = 'toast-container';
      document.body.appendChild(el);
    }
    return el;
  }

  _show({ type = 'info', title, message, duration = 4000 }) {
    const iconMap = {
      success: Icons.check,
      danger:  Icons.x,
      warning: Icons.warn,
      info:    Icons.info,
    };

    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.setAttribute('role', 'alert');
    toast.innerHTML = `
      <div class="toast-icon">${iconMap[type] || Icons.info}</div>
      <div class="toast-body">
        ${title ? `<div class="toast-title">${title}</div>` : ''}
        ${message ? `<div class="toast-message">${message}</div>` : ''}
      </div>
      <button class="toast-close" aria-label="Close">${Icons.x}</button>
      <div class="toast-progress" style="animation-duration:${duration}ms"></div>
    `;

    const close = () => {
      toast.classList.add('toast-out');
      toast.addEventListener('animationend', () => toast.remove(), { once: true });
      clearTimeout(timer);
    };

    toast.querySelector('.toast-close').addEventListener('click', close);
    const timer = setTimeout(close, duration);
    this.container.appendChild(toast);
    return close;
  }

  success(message, title = 'Success')   { return this._show({ type: 'success', title, message }); }
  danger(message, title = 'Error')      { return this._show({ type: 'danger',  title, message }); }
  warning(message, title = 'Warning')   { return this._show({ type: 'warning', title, message }); }
  info(message, title = 'Info')         { return this._show({ type: 'info',    title, message }); }
  error(message, title = 'Error')       { return this.danger(message, title); }
}

const Toast = new ToastManager();


// ─────────────────────────────────────────────────────────────────────────────
// MODAL
// ─────────────────────────────────────────────────────────────────────────────

class Modal {
  constructor({ id, title, size = 'md', content = '', footer = '', onClose } = {}) {
    this.id      = id || `modal-${Date.now()}`;
    this.onClose = onClose;
    this._el     = null;
    this._overlay = null;
    this._build(title, size, content, footer);
  }

  _build(title, size, content, footer) {
    this._overlay = document.createElement('div');
    this._overlay.className = 'modal-overlay';
    this._overlay.setAttribute('role', 'dialog');
    this._overlay.setAttribute('aria-modal', 'true');

    this._el = document.createElement('div');
    this._el.className = `modal modal-${size}`;
    this._el.id = this.id;

    this._el.innerHTML = `
      <div class="modal-header">
        <h2 class="modal-title">${title || ''}</h2>
        <button class="modal-close" aria-label="Close modal">${Icons.x}</button>
      </div>
      <div class="modal-body">${content}</div>
      ${footer ? `<div class="modal-footer">${footer}</div>` : ''}
    `;

    this._overlay.appendChild(this._el);

    // Close on overlay click (outside modal)
    this._overlay.addEventListener('click', (e) => {
      if (e.target === this._overlay) this.close();
    });

    // Close button
    this._el.querySelector('.modal-close').addEventListener('click', () => this.close());

    // ESC key
    this._escHandler = (e) => { if (e.key === 'Escape') this.close(); };
  }

  open() {
    document.body.appendChild(this._overlay);
    document.addEventListener('keydown', this._escHandler);
    document.body.style.overflow = 'hidden';
    setTimeout(() => this._overlay.classList.add('visible'), 0);
    this._el.querySelector('.modal-body')?.focus?.();
    return this;
  }

  close() {
    this._overlay.remove();
    document.removeEventListener('keydown', this._escHandler);
    document.body.style.overflow = '';
    this.onClose?.();
  }

  setTitle(t) { this._el.querySelector('.modal-title').textContent = t; return this; }
  setContent(html) { this._el.querySelector('.modal-body').innerHTML = html; return this; }
  setFooter(html) {
    let footer = this._el.querySelector('.modal-footer');
    if (!footer) {
      footer = document.createElement('div');
      footer.className = 'modal-footer';
      this._el.appendChild(footer);
    }
    footer.innerHTML = html;
    return this;
  }

  getBody() { return this._el.querySelector('.modal-body'); }
  getFooter() { return this._el.querySelector('.modal-footer'); }
}


// ─────────────────────────────────────────────────────────────────────────────
// CONFIRM DIALOG
// ─────────────────────────────────────────────────────────────────────────────

function Confirm({
  title = 'Are you sure?',
  message = '',
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  type = 'danger',
} = {}) {
  return new Promise((resolve) => {
    const modal = new Modal({
      size: 'sm',
      title: '',
      onClose: () => resolve(false),
    });

    modal.getBody().innerHTML = `
      <div style="display:flex;gap:1rem;align-items:flex-start">
        <div class="confirm-icon ${type}">
          ${type === 'danger' ? Icons.trash : Icons.warn}
        </div>
        <div>
          <div class="confirm-title">${title}</div>
          <div class="confirm-message">${message}</div>
        </div>
      </div>
    `;

    modal.setFooter(`
      <button class="btn btn-secondary btn-sm" id="confirm-cancel">${cancelText}</button>
      <button class="btn btn-${type} btn-sm" id="confirm-ok">${confirmText}</button>
    `);

    modal.open();

    modal.getFooter().querySelector('#confirm-cancel').addEventListener('click', () => {
      modal.close(); resolve(false);
    });
    modal.getFooter().querySelector('#confirm-ok').addEventListener('click', () => {
      modal.close(); resolve(true);
    });
  });
}


// ─────────────────────────────────────────────────────────────────────────────
// DATA TABLE
// ─────────────────────────────────────────────────────────────────────────────

class DataTable {
  constructor({ el, columns, data = [], pageSize = 20, searchable = true, onRowAction }) {
    this._el = typeof el === 'string' ? document.querySelector(el) : el;
    this._columns = columns;
    this._data = data;
    this._filtered = [...data];
    this._pageSize = pageSize;
    this._page = 1;
    this._sortCol = null;
    this._sortDir = 'asc';
    this._searchTerm = '';
    this._searchable = searchable;
    this._onRowAction = onRowAction;
    this._render();
  }

  setData(data) {
    this._data = data;
    this._search(this._searchTerm);
  }

  _search(term) {
    this._searchTerm = term.toLowerCase();
    this._filtered = this._data.filter(row =>
      this._columns.some(col => {
        const v = this._getValue(row, col.key) ?? '';
        return v.toString().toLowerCase().includes(this._searchTerm);
      })
    );
    this._page = 1;
    this._sort();
    this._renderBody();
    this._renderPagination();
  }

  _getValue(row, key) {
    return key.split('.').reduce((o, k) => o?.[k], row);
  }

  _sort() {
    if (!this._sortCol) return;
    this._filtered.sort((a, b) => {
      const va = this._getValue(a, this._sortCol) ?? '';
      const vb = this._getValue(b, this._sortCol) ?? '';
      const cmp = typeof va === 'number' ? va - vb : va.toString().localeCompare(vb.toString());
      return this._sortDir === 'asc' ? cmp : -cmp;
    });
  }

  _currentPage() {
    const start = (this._page - 1) * this._pageSize;
    return this._filtered.slice(start, start + this._pageSize);
  }

  _totalPages() {
    return Math.max(1, Math.ceil(this._filtered.length / this._pageSize));
  }

  _render() {
    this._el.innerHTML = `
      <div class="table-toolbar">
        ${this._searchable ? `
          <div class="input-wrapper table-search">
            <span class="input-icon">${Icons.search}</span>
            <input class="input dt-search" type="search" placeholder="Search...">
          </div>
        ` : '<div></div>'}
        <div class="table-actions" id="dt-actions-${this._uid}"></div>
      </div>
      <div class="table-wrapper" style="border-radius:0;border-top:none">
        <table class="table">
          <thead>${this._renderHead()}</thead>
          <tbody class="dt-body"></tbody>
        </table>
      </div>
      <div class="dt-pagination"></div>
    `;
    this._uid = Date.now();
    this._bindEvents();
    this._renderBody();
    this._renderPagination();
  }

  _renderHead() {
    return '<tr>' + this._columns.map(col => {
      const sortClass = this._sortCol === col.key
        ? (this._sortDir === 'asc' ? 'sorted-asc' : 'sorted-desc')
        : '';
      return `
        <th class="${col.sortable !== false ? 'sortable' : ''} ${sortClass}"
            data-key="${col.key}" style="${col.width ? `width:${col.width}` : ''}">
          ${col.label}
          ${col.sortable !== false ? `<span class="sort-icon">${this._sortCol === col.key ? (this._sortDir === 'asc' ? Icons.sortAsc : Icons.sortDesc) : Icons.sortAsc}</span>` : ''}
        </th>
      `;
    }).join('') + '</tr>';
  }

  _renderBody() {
    const tbody = this._el.querySelector('.dt-body');
    const rows = this._currentPage();

    if (!rows.length) {
      tbody.innerHTML = `
        <tr><td colspan="${this._columns.length}">
          <div class="empty-state" style="padding:3rem">
            ${Icons.search}<p>No records found</p>
          </div>
        </td></tr>
      `;
      return;
    }

    tbody.innerHTML = rows.map((row, i) => `
      <tr data-idx="${(this._page - 1) * this._pageSize + i}">
        ${this._columns.map(col => {
          if (col.key === '_actions') return `<td class="col-actions"><div class="row-actions">${col.render ? col.render(row) : ''}</div></td>`;
          const val = this._getValue(row, col.key);
          const cell = col.render ? col.render(val, row) : (val ?? '—');
          const cls = col.align === 'right' ? 'col-number' : col.class || '';
          return `<td class="${cls}">${cell}</td>`;
        }).join('')}
      </tr>
    `).join('');

    // Row click
    tbody.querySelectorAll('tr').forEach(tr => {
      tr.addEventListener('click', (e) => {
        if (e.target.closest('button, a')) return;
        const idx = parseInt(tr.dataset.idx);
        this._onRowAction?.('click', this._filtered[idx], idx);
      });
    });

    // Action button clicks
    tbody.querySelectorAll('[data-action]').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const tr = btn.closest('tr');
        const idx = parseInt(tr.dataset.idx);
        this._onRowAction?.(btn.dataset.action, this._filtered[idx], idx);
      });
    });
  }

  _renderPagination() {
    const el = this._el.querySelector('.dt-pagination');
    const total = this._filtered.length;
    const pages = this._totalPages();
    const start = Math.min((this._page - 1) * this._pageSize + 1, total);
    const end   = Math.min(this._page * this._pageSize, total);

    const pageBtn = (p, label, disabled = false, active = false) =>
      `<button class="page-btn ${active ? 'active' : ''}" data-page="${p}" ${disabled ? 'disabled' : ''}>${label}</button>`;

    let btns = pageBtn(1, '«', this._page === 1);
    btns += pageBtn(this._page - 1, '‹', this._page === 1);
    for (let p = Math.max(1, this._page - 2); p <= Math.min(pages, this._page + 2); p++) {
      btns += pageBtn(p, p, false, p === this._page);
    }
    btns += pageBtn(this._page + 1, '›', this._page === pages);
    btns += pageBtn(pages, '»', this._page === pages);

    el.innerHTML = `
      <div class="pagination">
        <div class="pagination-info">Showing ${start}–${end} of ${total} records</div>
        <div class="pagination-btns">${btns}</div>
      </div>
    `;

    el.querySelectorAll('.page-btn:not([disabled])').forEach(btn => {
      btn.addEventListener('click', () => {
        this._page = parseInt(btn.dataset.page);
        this._renderBody();
        this._renderPagination();
      });
    });
  }

  _bindEvents() {
    const searchInput = this._el.querySelector('.dt-search');
    if (searchInput) {
      let timer;
      searchInput.addEventListener('input', () => {
        clearTimeout(timer);
        timer = setTimeout(() => this._search(searchInput.value), 250);
      });
    }

    this._el.querySelector('thead').addEventListener('click', (e) => {
      const th = e.target.closest('th.sortable');
      if (!th) return;
      const key = th.dataset.key;
      if (this._sortCol === key) {
        this._sortDir = this._sortDir === 'asc' ? 'desc' : 'asc';
      } else {
        this._sortCol = key;
        this._sortDir = 'asc';
      }
      this._el.querySelector('thead').innerHTML = this._renderHead();
      this._el.querySelector('thead').addEventListener('click', this._thClick);
      this._sort();
      this._renderBody();
    });
  }

  addAction(html) {
    const el = this._el.querySelector('.table-actions');
    if (el) el.insertAdjacentHTML('beforeend', html);
  }

  refresh() { this._renderBody(); this._renderPagination(); }
}


// ─────────────────────────────────────────────────────────────────────────────
// BADGE
// ─────────────────────────────────────────────────────────────────────────────

function Badge(text, type = 'default', dot = false) {
  const dotHtml = dot ? '<span class="badge-dot"></span>' : '';
  return `<span class="badge badge-${type}">${dotHtml}${text}</span>`;
}

function StatusBadge(status) {
  const map = {
    draft:              ['Draft',       'draft'],
    submitted:          ['Submitted',   'submitted'],
    approved:           ['Approved',    'approved'],
    posted:             ['Posted',      'posted'],
    closed:             ['Closed',      'closed'],
    cancelled:          ['Cancelled',   'cancelled'],
    pending:            ['Pending',     'pending'],
    paid:               ['Paid',        'paid'],
    partially_paid:     ['Partial',     'partial'],
    partially_accepted: ['Partial',     'warning'],
    accepted:           ['Accepted',    'approved'],
    rejected:           ['Rejected',    'danger'],
    delivered:          ['Delivered',   'success'],
    active:             ['Active',      'success'],
    inactive:           ['Inactive',    'default'],
    present:            ['Present',     'success'],
    absent:             ['Absent',      'danger'],
    half_day:           ['Half Day',    'warning'],
    leave:              ['Leave',       'info'],
    processed:          ['Processed',   'primary'],
  };
  const [label, type] = map[status] || [status, 'default'];
  return Badge(label, type, true);
}


// ─────────────────────────────────────────────────────────────────────────────
// AMOUNT FORMATTER
// ─────────────────────────────────────────────────────────────────────────────

function formatAmount(amount, currency = 'PKR', decimals = 2) {
  const n = parseFloat(amount) || 0;
  const formatted = n.toLocaleString('en-PK', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
  return currency ? `${currency} ${formatted}` : formatted;
}

function formatDate(isoStr) {
  if (!isoStr) return '—';
  const d = new Date(isoStr);
  return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' });
}

function formatDateTime(isoStr) {
  if (!isoStr) return '—';
  const d = new Date(isoStr);
  return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' })
    + ' ' + d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
}


// ─────────────────────────────────────────────────────────────────────────────
// TABS
// ─────────────────────────────────────────────────────────────────────────────

class Tabs {
  constructor(containerEl) {
    this._el = typeof containerEl === 'string' ? document.querySelector(containerEl) : containerEl;
    this._btns = this._el.querySelectorAll('.tab-btn');
    this._panels = this._el.querySelectorAll('.tab-panel');
    this._init();
  }

  _init() {
    this._btns.forEach(btn => {
      btn.addEventListener('click', () => this.activate(btn.dataset.tab));
    });
    // activate first by default
    if (this._btns.length) this.activate(this._btns[0].dataset.tab);
  }

  activate(tabId) {
    this._btns.forEach(b => b.classList.toggle('active', b.dataset.tab === tabId));
    this._panels.forEach(p => p.classList.toggle('active', p.dataset.tab === tabId));
  }
}


// ─────────────────────────────────────────────────────────────────────────────
// DROPDOWN
// ─────────────────────────────────────────────────────────────────────────────

class Dropdown {
  constructor(triggerEl, menuEl) {
    this._trigger = typeof triggerEl === 'string' ? document.querySelector(triggerEl) : triggerEl;
    this._menu    = typeof menuEl === 'string' ? document.querySelector(menuEl) : menuEl;
    this._open = false;
    this._init();
  }

  _init() {
    this._trigger.addEventListener('click', (e) => {
      e.stopPropagation();
      this._open ? this.hide() : this.show();
    });
    document.addEventListener('click', () => this.hide());
  }

  show() {
    this._menu.style.display = 'block';
    this._open = true;
  }

  hide() {
    this._menu.style.display = 'none';
    this._open = false;
  }
}


// ─────────────────────────────────────────────────────────────────────────────
// SKELETON
// ─────────────────────────────────────────────────────────────────────────────

function SkeletonRows(cols, rows = 5) {
  return Array.from({ length: rows }, () =>
    `<tr>${Array.from({ length: cols }, () =>
      `<td><div class="skeleton skeleton-text w-3-4"></div></td>`
    ).join('')}</tr>`
  ).join('');
}


// ─────────────────────────────────────────────────────────────────────────────
// FORM BUILDER HELPER
// ─────────────────────────────────────────────────────────────────────────────

function FormGroup({ label, id, required = false, hint, type = 'text', placeholder = '', value = '', extra = '' } = {}) {
  const reqMark = required ? '<span class="required">*</span>' : '';
  const hintHtml = hint ? `<div class="form-hint">${hint}</div>` : '';
  return `
    <div class="form-group">
      <label class="form-label" for="${id}">${label} ${reqMark}</label>
      <input class="input" id="${id}" name="${id}" type="${type}" placeholder="${placeholder}"
             value="${value}" ${required ? 'required' : ''} ${extra}>
      ${hintHtml}
    </div>
  `;
}

function SelectGroup({ label, id, required = false, hint, options = [], value = '' } = {}) {
  const reqMark = required ? '<span class="required">*</span>' : '';
  const hintHtml = hint ? `<div class="form-hint">${hint}</div>` : '';
  const opts = options.map(o => {
    const v = typeof o === 'object' ? o.value : o;
    const l = typeof o === 'object' ? o.label : o;
    return `<option value="${v}" ${v == value ? 'selected' : ''}>${l}</option>`;
  }).join('');
  return `
    <div class="form-group">
      <label class="form-label" for="${id}">${label} ${reqMark}</label>
      <select class="input" id="${id}" name="${id}" ${required ? 'required' : ''}>
        <option value="">— Select —</option>
        ${opts}
      </select>
      ${hintHtml}
    </div>
  `;
}


// ─────────────────────────────────────────────────────────────────────────────
// KPI CARD BUILDER
// ─────────────────────────────────────────────────────────────────────────────

function KpiCard({ label, value, sub, trend, trendDir = 'up', icon, type = 'primary' } = {}) {
  const trendHtml = trend
    ? `<div class="kpi-trend ${trendDir}">
        ${trendDir === 'up' ? '▲' : '▼'} ${trend}
       </div>`
    : '';
  const iconHtml = icon
    ? `<div class="kpi-icon">${icon}</div>`
    : '';
  return `
    <div class="kpi-card ${type}">
      ${iconHtml}
      <div class="kpi-label">${label}</div>
      <div class="kpi-value">${value}</div>
      ${sub ? `<div class="kpi-sub">${sub}</div>` : ''}
      ${trendHtml}
    </div>
  `;
}


// ─────────────────────────────────────────────────────────────────────────────
// EXPORTS
// ─────────────────────────────────────────────────────────────────────────────

export {
  // Core
  Icons,
  Validator,
  Toast,
  Modal,
  Confirm,
  DataTable,
  Tabs,
  Dropdown,

  // Helpers / Builders
  Badge,
  StatusBadge,
  FormGroup,
  SelectGroup,
  KpiCard,
  SkeletonRows,

  // Formatters
  formatAmount,
  formatDate,
  formatDateTime,
};
