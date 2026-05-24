/**
 * router.js — SPA client-side router for MemonBook ERP.
 * Hash-based (#/path) routing for Tauri compatibility.
 * Supports route guards, lazy module loading, and breadcrumbs.
 */

'use strict';

// ─────────────────────────────────────────────────────────────────────────────
// Route definitions
// ─────────────────────────────────────────────────────────────────────────────

const ROUTES = [
  // Public
  { path: '/login',     module: null, public: true,  title: 'Login' },

  // Dashboard
  { path: '/',          module: () => import('../modules/dashboard.js'),    title: 'Dashboard',     icon: 'home' },
  { path: '/dashboard', module: () => import('../modules/dashboard.js'),    title: 'Dashboard',     icon: 'home' },

  // Masters
  { path: '/masters/coa',           module: () => import('../modules/masters/coa.js'),        title: 'Chart of Accounts', parent: 'Masters' },
  { path: '/masters/customers',     module: () => import('../modules/masters/customers.js'),   title: 'Customers',         parent: 'Masters' },
  { path: '/masters/vendors',       module: () => import('../modules/masters/vendors.js'),     title: 'Vendors',           parent: 'Masters' },
  { path: '/masters/items',         module: () => import('../modules/masters/items.js'),       title: 'Items',             parent: 'Masters' },
  { path: '/masters/uom',           module: () => import('../modules/masters/uom.js'),         title: 'UOM',               parent: 'Masters' },
  { path: '/masters/currencies',    module: () => import('../modules/masters/currencies.js'),  title: 'Currencies',        parent: 'Masters' },
  { path: '/masters/tax',           module: () => import('../modules/masters/tax.js'),         title: 'Tax Config',        parent: 'Masters' },
  { path: '/masters/locations',     module: () => import('../modules/masters/locations.js'),   title: 'Locations',         parent: 'Masters' },

  // Purchases
  { path: '/purchases/po',          module: () => import('../modules/purchases/po.js'),        title: 'Purchase Orders',   parent: 'Purchases' },
  { path: '/purchases/grn',         module: () => import('../modules/purchases/grn.js'),       title: 'GRN',               parent: 'Purchases' },
  { path: '/purchases/pi',          module: () => import('../modules/purchases/pi.js'),        title: 'Purchase Invoices', parent: 'Purchases' },
  { path: '/purchases/pr',          module: () => import('../modules/purchases/pr.js'),        title: 'Purchase Returns',  parent: 'Purchases' },

  // Sales
  { path: '/sales/so',              module: () => import('../modules/sales/so.js'),            title: 'Sale Orders',       parent: 'Sales' },
  { path: '/sales/dc',              module: () => import('../modules/sales/dc.js'),            title: 'Delivery Challans', parent: 'Sales' },
  { path: '/sales/si',              module: () => import('../modules/sales/si.js'),            title: 'Sale Invoices',     parent: 'Sales' },
  { path: '/sales/sr',              module: () => import('../modules/sales/sr.js'),            title: 'Sale Returns',      parent: 'Sales' },

  // Payments
  { path: '/payments/receipts',     module: () => import('../modules/payments/receipts.js'),   title: 'Receipts',          parent: 'Payments' },
  { path: '/payments/payments',     module: () => import('../modules/payments/payments.js'),   title: 'Payments',          parent: 'Payments' },
  { path: '/accounts/jv',           module: () => import('../modules/payments/jv.js'),         title: 'Journal Vouchers',  parent: 'Accounts' },
  { path: '/accounts/ledger',       module: () => import('../modules/accounts/ledger.js'),     title: 'Ledger',            parent: 'Accounts' },
  { path: '/accounts/trial-balance',module: () => import('../modules/accounts/ledger.js'),    title: 'Trial Balance',     parent: 'Accounts' },

  // HR
  { path: '/hr/employees',          module: () => import('../modules/hr/employees.js'),        title: 'Employees',         parent: 'HR' },
  { path: '/hr/attendance',         module: () => import('../modules/hr/attendance.js'),       title: 'Attendance',        parent: 'HR' },
  { path: '/hr/payroll',            module: () => import('../modules/hr/payroll.js'),          title: 'Payroll',           parent: 'HR' },

  // Reports
  { path: '/reports',               module: () => import('../modules/reports/reports.js'),     title: 'Reports',           parent: 'Reports' },

  // System
  { path: '/system/settings',       module: () => import('../modules/system/settings.js'),    title: 'Settings',          parent: 'System' },
  { path: '/system/users',          module: () => import('../modules/system/users.js'),        title: 'Users',             parent: 'System', role: 'admin' },
  { path: '/system/devices',        module: () => import('../modules/system/devices.js'),      title: 'Devices',           parent: 'System', role: 'admin' },

  // 404
  { path: '*', module: null, title: '404 Not Found' },
];

// ─────────────────────────────────────────────────────────────────────────────
// Router
// ─────────────────────────────────────────────────────────────────────────────

const Router = {
  _current: null,
  _container: null,
  _beforeEach: [],

  init(containerSelector = '#page-content') {
    this._container = document.querySelector(containerSelector);
    window.addEventListener('hashchange', () => this._resolve());
    this._resolve();
  },

  navigate(path) {
    window.location.hash = '#' + path;
  },

  replace(path) {
    window.location.replace('#' + path);
  },

  back() {
    window.history.back();
  },

  currentPath() {
    const hash = window.location.hash.slice(1) || '/';
    return hash.split('?')[0];
  },

  currentQuery() {
    const hash = window.location.hash.slice(1) || '';
    const qi = hash.indexOf('?');
    return qi >= 0 ? new URLSearchParams(hash.slice(qi + 1)) : new URLSearchParams();
  },

  beforeEach(fn) {
    this._beforeEach.push(fn);
  },

  async _resolve() {
    const path = this.currentPath();
    const route = this._match(path);

    if (!route) {
      this._render404();
      return;
    }

    // Run guards
    for (const guard of this._beforeEach) {
      const result = await guard(route, this._current);
      if (result === false) return;
      if (typeof result === 'string') { this.navigate(result); return; }
    }

    this._current = route;
    this._updateBreadcrumb(route);
    this._setActive(path);

    if (route.public && !route.module) {
      // Handled by login page directly
      return;
    }

    if (!route.module) {
      this._render404();
      return;
    }

    // Lazy load module
    this._container.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;height:200px">
        <div class="spinner"></div>
      </div>
    `;

    try {
      const mod = await route.module();
      if (typeof mod.init === 'function') {
        await mod.init(this._container, this.currentQuery());
      } else if (typeof mod.default?.init === 'function') {
        await mod.default.init(this._container, this.currentQuery());
      } else {
        this._container.innerHTML = `<div class="page-container"><p>Module loaded but no init() found.</p></div>`;
      }
    } catch (err) {
      console.error('Route load error:', err);
      this._container.innerHTML = `
        <div class="page-container">
          <div class="alert alert-danger">
            <p>Failed to load this page. ${err.message}</p>
          </div>
        </div>
      `;
    }
  },

  _match(path) {
    return ROUTES.find(r => {
      if (r.path === '*') return false;
      if (r.path === path) return true;
      if (r.path.includes(':')) {
        const regex = new RegExp('^' + r.path.replace(/:[\w]+/g, '([\\w-]+)') + '$');
        return regex.test(path);
      }
      return false;
    }) || ROUTES.find(r => r.path === '*');
  },

  _render404() {
    if (this._container) {
      this._container.innerHTML = `
        <div class="page-container" style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:60vh">
          <div style="font-size:4rem;font-weight:800;color:var(--text-muted)">404</div>
          <div style="font-size:1.25rem;color:var(--text-secondary);margin:1rem 0">Page not found</div>
          <button class="btn btn-primary" onclick="Router.navigate('/')">Go Home</button>
        </div>
      `;
    }
  },

  _updateBreadcrumb(route) {
    const el = document.getElementById('breadcrumb');
    if (!el) return;
    const parts = [];
    if (route.parent) parts.push(`<span>${route.parent}</span><span class="breadcrumb-sep">/</span>`);
    parts.push(`<span class="breadcrumb-current">${route.title}</span>`);
    el.innerHTML = parts.join('');
    document.title = `${route.title} — MemonBook ERP`;
  },

  _setActive(path) {
    document.querySelectorAll('.nav-link').forEach(link => {
      const href = link.getAttribute('href') || link.dataset.route || '';
      const active = href === '#' + path || href === path;
      link.classList.toggle('active', active);
    });
  },
};

// Make Router globally available
window.Router = Router;

export default Router;
