/**
 * app.js — MemonBook ERP Application Bootstrap.
 * Initializes theme, auth guard, sidebar, router.
 * Entry point for all authenticated pages.
 */

'use strict';

import Auth   from './auth.js';
import Router from './router.js';

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar navigation tree (role-based visibility applied at render time)
// ─────────────────────────────────────────────────────────────────────────────

const NAV_TREE = [
  {
    label: 'MAIN',
    items: [
      { label: 'Dashboard', route: '/',          icon: 'home',       roles: null },
    ],
  },
  {
    label: 'MASTERS',
    module: 'masters',
    items: [
      {
        label: 'Masters', icon: 'layers', children: [
          { label: 'Chart of Accounts', route: '/masters/coa' },
          { label: 'Customers',         route: '/masters/customers' },
          { label: 'Vendors',           route: '/masters/vendors' },
          { label: 'Items',             route: '/masters/items' },
          { label: 'UOM',              route: '/masters/uom' },
          { label: 'Currencies',        route: '/masters/currencies' },
          { label: 'Tax Config',        route: '/masters/tax' },
          { label: 'Locations',         route: '/masters/locations' },
        ],
      },
    ],
  },
  {
    label: 'PURCHASES',
    module: 'purchases',
    items: [
      {
        label: 'Purchases', icon: 'shopping-cart', children: [
          { label: 'Purchase Orders', route: '/purchases/po' },
          { label: 'GRN',             route: '/purchases/grn' },
          { label: 'Purchase Invoices', route: '/purchases/pi' },
          { label: 'Purchase Returns',  route: '/purchases/pr' },
        ],
      },
    ],
  },
  {
    label: 'SALES',
    module: 'sales',
    items: [
      {
        label: 'Sales', icon: 'trending-up', children: [
          { label: 'Sale Orders',       route: '/sales/so' },
          { label: 'Delivery Challans', route: '/sales/dc' },
          { label: 'Sale Invoices',     route: '/sales/si' },
          { label: 'Sale Returns',      route: '/sales/sr' },
        ],
      },
    ],
  },
  {
    label: 'PAYMENTS',
    module: 'payments',
    items: [
      {
        label: 'Payments', icon: 'credit-card', children: [
          { label: 'Cash/Bank Receipts', route: '/payments/receipts' },
          { label: 'Cash/Bank Payments', route: '/payments/payments' },
        ],
      },
    ],
  },
  {
    label: 'ACCOUNTS',
    module: 'accounts',
    items: [
      {
        label: 'Accounts', icon: 'book-open', children: [
          { label: 'Journal Vouchers', route: '/accounts/jv' },
          { label: 'Ledger View',      route: '/accounts/ledger' },
          { label: 'Trial Balance',    route: '/accounts/trial-balance' },
        ],
      },
    ],
  },
  {
    label: 'HR & PAYROLL',
    module: 'hr',
    items: [
      {
        label: 'HR & Payroll', icon: 'users', children: [
          { label: 'Employees',  route: '/hr/employees' },
          { label: 'Attendance', route: '/hr/attendance' },
          { label: 'Payroll',    route: '/hr/payroll' },
        ],
      },
    ],
  },
  {
    label: 'REPORTS',
    module: 'reports',
    items: [
      { label: 'Reports', route: '/reports', icon: 'bar-chart-2', roles: null },
    ],
  },
  {
    label: 'SYSTEM',
    roles: ['admin'],
    items: [
      {
        label: 'System', icon: 'settings', roles: ['admin'], children: [
          { label: 'Settings', route: '/system/settings' },
          { label: 'Users',    route: '/system/users',   roles: ['admin'] },
          { label: 'Devices',  route: '/system/devices', roles: ['admin'] },
        ],
      },
    ],
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// SVG icons for nav (inline, no external dep)
// ─────────────────────────────────────────────────────────────────────────────

const NAV_ICONS = {
  home:         `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>`,
  layers:       `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>`,
  'shopping-cart': `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="21" r="1"/><circle cx="20" cy="21" r="1"/><path d="M1 1h4l2.68 13.39a2 2 0 002 1.61h9.72a2 2 0 002-1.61L23 6H6"/></svg>`,
  'trending-up':   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>`,
  'credit-card':   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>`,
  'book-open':     `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z"/><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z"/></svg>`,
  users:           `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>`,
  'bar-chart-2':   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>`,
  settings:        `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>`,
  sun:             `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`,
  moon:            `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/></svg>`,
  chevronRight:    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>`,
  menu:            `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/></svg>`,
  'panel-left':    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/></svg>`,
  'log-out':       `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>`,
  bell:            `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 01-3.46 0"/></svg>`,
  logo:            `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h20v4H2zM4 7v14M20 7v14M4 14h16"/></svg>`,
};

// ─────────────────────────────────────────────────────────────────────────────
// Sidebar renderer
// ─────────────────────────────────────────────────────────────────────────────

function renderSidebar() {
  const role = Auth.getRole();
  const user = Auth.getUser();
  const permissions = Auth.getPermissions();

  const canSeeModule = (module) => {
    if (!module) return true;
    const perms = permissions[module] || [];
    return perms.includes('view');
  };

  const canSeeItem = (item) => {
    if (!item.roles) return true;
    return item.roles.includes(role);
  };

  let navHtml = '';
  for (const section of NAV_TREE) {
    if (section.roles && !section.roles.includes(role)) continue;
    if (section.module && !canSeeModule(section.module)) continue;

    navHtml += `<div class="nav-section-label">${section.label}</div>`;

    for (const item of section.items) {
      if (!canSeeItem(item)) continue;

      const icon = NAV_ICONS[item.icon] || '';

      if (item.children) {
        const visibleChildren = item.children.filter(canSeeItem);
        if (!visibleChildren.length) continue;

        navHtml += `
          <div class="nav-item" id="nav-${item.label.replace(/\s/g, '-').toLowerCase()}">
            <button class="nav-link" aria-haspopup="true" aria-expanded="false">
              <span class="nav-icon">${icon}</span>
              <span class="nav-label">${item.label}</span>
              <span class="nav-chevron">${NAV_ICONS.chevronRight}</span>
              <span class="nav-tooltip">${item.label}</span>
            </button>
            <div class="nav-submenu">
              ${visibleChildren.map(child => `
                <div class="nav-item">
                  <a class="nav-link" href="#${child.route}" data-route="${child.route}">
                    <span class="nav-icon">${NAV_ICONS[child.icon || 'chevronRight'] || ''}</span>
                    <span class="nav-label">${child.label}</span>
                    <span class="nav-tooltip">${child.label}</span>
                  </a>
                </div>
              `).join('')}
            </div>
          </div>
        `;
      } else {
        navHtml += `
          <div class="nav-item">
            <a class="nav-link" href="#${item.route}" data-route="${item.route}">
              <span class="nav-icon">${icon}</span>
              <span class="nav-label">${item.label}</span>
              <span class="nav-tooltip">${item.label}</span>
            </a>
          </div>
        `;
      }
    }
  }

  const initials = (user?.username || 'U').slice(0, 2).toUpperCase();
  const theme = Auth.getTheme();

  document.getElementById('sidebar').innerHTML = `
    <a class="sidebar-logo" href="#/">
      <div class="sidebar-logo-icon">${NAV_ICONS.logo}</div>
      <div class="sidebar-logo-text">
        <div class="app-name">MemonBook ERP</div>
        <div class="app-version">v1.0.0</div>
      </div>
    </a>

    <div class="sidebar-search">
      <div class="sidebar-search-inner">
        ${NAV_ICONS.search || ''}
        <input class="sidebar-search-input" id="nav-search" type="search" placeholder="Search menu...">
      </div>
    </div>

    <nav class="sidebar-nav" id="sidebar-nav">${navHtml}</nav>

    <div class="sidebar-bottom">
      <div class="sidebar-user" id="user-menu-trigger">
        <div class="user-avatar">${initials}</div>
        <div class="user-info">
          <div class="user-name">${user?.username || 'Admin'}</div>
          <div class="user-role">${user?.role || 'admin'}</div>
        </div>
      </div>
      <div class="sidebar-controls">
        <button class="theme-toggle" id="theme-toggle" title="Toggle theme">
          ${theme === 'dark' ? NAV_ICONS.sun : NAV_ICONS.moon}
        </button>
        <button class="btn btn-sm btn-ghost-danger" id="logout-btn" title="Logout">
          ${NAV_ICONS['log-out']}
        </button>
        <button class="sidebar-collapse-btn" id="sidebar-collapse" title="Collapse sidebar">
          ${NAV_ICONS['panel-left']}
        </button>
      </div>
    </div>
  `;

  _bindSidebarEvents();
}

function _bindSidebarEvents() {
  const sidebar = document.getElementById('sidebar');

  // Submenu toggles
  sidebar.querySelectorAll('.nav-item > .nav-link[aria-haspopup]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if (sidebar.classList.contains('collapsed')) return;
      const navItem = btn.closest('.nav-item');
      const isOpen = navItem.classList.contains('open');
      // Close all
      sidebar.querySelectorAll('.nav-item.open').forEach(el => el.classList.remove('open'));
      if (!isOpen) navItem.classList.add('open');
      btn.setAttribute('aria-expanded', !isOpen);
    });
  });

  // Theme toggle
  document.getElementById('theme-toggle')?.addEventListener('click', () => {
    Auth.toggleTheme();
    const theme = Auth.getTheme();
    const toggleBtn = document.getElementById('theme-toggle');
    if (toggleBtn) toggleBtn.innerHTML = theme === 'dark' ? NAV_ICONS.sun : NAV_ICONS.moon;
  });

  // Collapse
  document.getElementById('sidebar-collapse')?.addEventListener('click', () => {
    sidebar.classList.toggle('collapsed');
    const mainContent = document.querySelector('.main-content');
    if (mainContent) mainContent.classList.toggle('sidebar-collapsed');
    localStorage.setItem('mb_sidebar_collapsed', sidebar.classList.contains('collapsed') ? '1' : '0');
  });

  // Restore collapsed state
  if (localStorage.getItem('mb_sidebar_collapsed') === '1') {
    sidebar.classList.add('collapsed');
    document.querySelector('.main-content')?.classList.add('sidebar-collapsed');
  }

  // Logout
  document.getElementById('logout-btn')?.addEventListener('click', async () => {
    const { Confirm } = await import('../components/components.js');
    const ok = await Confirm({ title: 'Logout?', message: 'You will be returned to the login screen.', type: 'warning', confirmText: 'Logout' });
    if (ok) {
      Auth.logout();
      window.location.href = 'login.html';
    }
  });

  // Nav search filter
  document.getElementById('nav-search')?.addEventListener('input', (e) => {
    const term = e.target.value.toLowerCase();
    sidebar.querySelectorAll('.nav-item').forEach(item => {
      const label = item.querySelector('.nav-label')?.textContent.toLowerCase() || '';
      const visible = !term || label.includes(term) || item.querySelector('.nav-submenu');
      item.style.display = visible ? '' : 'none';
    });
    if (term) {
      sidebar.querySelectorAll('.nav-item:has(.nav-submenu)').forEach(item => {
        item.classList.toggle('open', true);
      });
    }
  });

  // Mobile sidebar toggle
  document.getElementById('mobile-menu-btn')?.addEventListener('click', () => {
    sidebar.classList.toggle('mobile-open');
    document.getElementById('sidebar-overlay')?.classList.toggle('visible');
  });

  document.getElementById('sidebar-overlay')?.addEventListener('click', () => {
    sidebar.classList.remove('mobile-open');
    document.getElementById('sidebar-overlay')?.classList.remove('visible');
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// App Bootstrap
// ─────────────────────────────────────────────────────────────────────────────

async function initApp() {
  // 1. Apply saved theme immediately
  Auth.applyTheme();

  // 2. Check auth
  if (!Auth.isLoggedIn()) {
    window.location.href = 'login.html';
    return;
  }

  // 3. Render sidebar + header
  renderSidebar();

  // 4. Auth guard
  Router.beforeEach((route) => {
    if (route.public) return true;
    if (!Auth.isLoggedIn()) return '/login';
    if (route.role && !Auth.getRole() === route.role) return false;
    return true;
  });

  // 5. Init router
  Router.init('#page-content');
}

// Run on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initApp);
} else {
  initApp();
}

export default { initApp, renderSidebar };
