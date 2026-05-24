/**
 * dashboard.js — MemonBook ERP Dashboard Module
 * KPI cards, sales chart, purchase chart, recent transactions,
 * low-stock alert, payable/receivable summary.
 */

'use strict';

import { Api, APIError } from '../core/api.js';
import { Toast, KpiCard, formatAmount, formatDate, StatusBadge } from '../components/components.js';

// ─────────────────────────────────────────────────────────────────────────────
// Dashboard icons
// ─────────────────────────────────────────────────────────────────────────────

const ICONS = {
  sales:      `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>`,
  purchase:   `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="21" r="1"/><circle cx="20" cy="21" r="1"/><path d="M1 1h4l2.68 13.39a2 2 0 002 1.61h9.72a2 2 0 002-1.61L23 6H6"/></svg>`,
  receivable: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6"/></svg>`,
  payable:    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="1" y="4" width="22" height="16" rx="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>`,
  stock:      `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/></svg>`,
  employees:  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>`,
  alert:      `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,
  refresh:    `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/></svg>`,
};

// ─────────────────────────────────────────────────────────────────────────────
// Dashboard state
// ─────────────────────────────────────────────────────────────────────────────

let _charts = {};

function _destroyCharts() {
  Object.values(_charts).forEach(c => c?.destroy?.());
  _charts = {};
}

// ─────────────────────────────────────────────────────────────────────────────
// Data fetchers
// ─────────────────────────────────────────────────────────────────────────────

async function fetchDashboardData() {
  // Parallel requests
  const [kpi, salesChart, purchaseChart, recentSales, recentPurchases, lowStock] = await Promise.allSettled([
    Api.get('/reports/dashboard/kpi').catch(() => _mockKpi()),
    Api.get('/reports/dashboard/sales-chart').catch(() => _mockSalesChart()),
    Api.get('/reports/dashboard/purchase-chart').catch(() => _mockPurchaseChart()),
    Api.get('/sales/si/', { params: { limit: 8, sort: '-created_at' } }).catch(() => ({ items: [] })),
    Api.get('/purchases/pi/', { params: { limit: 8, sort: '-created_at' } }).catch(() => ({ items: [] })),
    Api.get('/reports/dashboard/low-stock').catch(() => ({ items: [] })),
  ]);

  return {
    kpi:             kpi.value || _mockKpi(),
    salesChart:      salesChart.value || _mockSalesChart(),
    purchaseChart:   purchaseChart.value || _mockPurchaseChart(),
    recentSales:     (recentSales.value?.items || []),
    recentPurchases: (recentPurchases.value?.items || []),
    lowStock:        (lowStock.value?.items || []),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Mock data (shown when backend endpoints not yet implemented)
// ─────────────────────────────────────────────────────────────────────────────

function _mockKpi() {
  return {
    total_sales:       4850000,
    total_purchases:   3120000,
    total_receivable:  980000,
    total_payable:     540000,
    total_employees:   24,
    low_stock_items:   7,
    sales_trend:       '+12.4%',
    purchases_trend:   '+8.1%',
  };
}

function _mockSalesChart() {
  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return {
    labels: months,
    sales:     [320000, 410000, 290000, 520000, 480000, 610000, 590000, 720000, 640000, 810000, 750000, 890000],
    purchases: [210000, 280000, 195000, 340000, 310000, 420000, 390000, 510000, 460000, 580000, 540000, 620000],
  };
}

function _mockPurchaseChart() {
  return {
    labels: ['Electronics', 'Machinery', 'Raw Material', 'Packaging', 'Other'],
    values: [35, 25, 20, 12, 8],
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Render
// ─────────────────────────────────────────────────────────────────────────────

function renderKpiCards(kpi) {
  return `
    <div class="kpi-grid">
      ${KpiCard({ label: 'Total Sales (MTD)',    value: formatAmount(kpi.total_sales),      type: 'primary', icon: ICONS.sales,      trend: kpi.sales_trend,     trendDir: 'up' })}
      ${KpiCard({ label: 'Total Purchases (MTD)',value: formatAmount(kpi.total_purchases),  type: 'warning', icon: ICONS.purchase,   trend: kpi.purchases_trend, trendDir: 'up' })}
      ${KpiCard({ label: 'Receivables',          value: formatAmount(kpi.total_receivable), type: 'success', icon: ICONS.receivable, sub: 'Outstanding from customers' })}
      ${KpiCard({ label: 'Payables',             value: formatAmount(kpi.total_payable),    type: 'danger',  icon: ICONS.payable,    sub: 'Due to vendors' })}
      ${KpiCard({ label: 'Active Employees',     value: kpi.total_employees,               type: 'info',    icon: ICONS.employees,  sub: 'Total headcount' })}
      ${KpiCard({ label: 'Low Stock Items',      value: kpi.low_stock_items,               type: kpi.low_stock_items > 5 ? 'danger' : 'warning', icon: ICONS.stock, sub: 'Below minimum level' })}
    </div>
  `;
}

function renderSalesChart(data) {
  return `
    <div class="chart-card">
      <div class="chart-header">
        <div class="chart-title">Sales vs Purchases — Monthly</div>
        <div class="chart-period" id="chart-year">FY ${new Date().getFullYear()}</div>
      </div>
      <div class="chart-body">
        <canvas id="sales-chart" height="90"></canvas>
      </div>
    </div>
  `;
}

function renderPurchaseChart() {
  return `
    <div class="chart-card">
      <div class="chart-header">
        <div class="chart-title">Purchases by Category</div>
        <div class="chart-period">This Month</div>
      </div>
      <div class="chart-body" style="display:flex;align-items:center;justify-content:center">
        <canvas id="purchase-chart" width="200" height="200" style="max-width:220px;max-height:220px"></canvas>
      </div>
    </div>
  `;
}

function renderRecentSales(items) {
  if (!items.length) {
    return `<div class="empty-state">${ICONS.sales}<p>No recent sale invoices</p></div>`;
  }
  return `
    <table class="table">
      <thead><tr>
        <th>Invoice #</th><th>Customer</th><th>Date</th>
        <th class="col-number">Amount</th><th>Status</th>
      </tr></thead>
      <tbody>
        ${items.map(si => `
          <tr>
            <td><a href="#/sales/si?id=${si.id}" class="font-medium">${si.si_number || '—'}</a></td>
            <td class="truncate" style="max-width:160px">${si.customer_name || '—'}</td>
            <td class="col-date">${formatDate(si.invoice_date)}</td>
            <td class="col-number">${formatAmount(si.total_amount)}</td>
            <td>${StatusBadge(si.status)}</td>
          </tr>
        `).join('')}
      </tbody>
    </table>
  `;
}

function renderRecentPurchases(items) {
  if (!items.length) {
    return `<div class="empty-state">${ICONS.purchase}<p>No recent purchase invoices</p></div>`;
  }
  return `
    <table class="table">
      <thead><tr>
        <th>Invoice #</th><th>Vendor</th><th>Date</th>
        <th class="col-number">Amount</th><th>Status</th>
      </tr></thead>
      <tbody>
        ${items.map(pi => `
          <tr>
            <td><a href="#/purchases/pi?id=${pi.id}" class="font-medium">${pi.pi_number || '—'}</a></td>
            <td class="truncate" style="max-width:160px">${pi.vendor_name || '—'}</td>
            <td class="col-date">${formatDate(pi.invoice_date)}</td>
            <td class="col-number">${formatAmount(pi.total_amount)}</td>
            <td>${StatusBadge(pi.status)}</td>
          </tr>
        `).join('')}
      </tbody>
    </table>
  `;
}

function renderLowStock(items) {
  if (!items.length) {
    return `<div class="empty-state">${ICONS.stock}<p>All items within stock limits</p></div>`;
  }
  return `
    <table class="table">
      <thead><tr>
        <th>Item</th><th class="col-number">Current Stock</th>
        <th class="col-number">Min Level</th><th>UOM</th>
      </tr></thead>
      <tbody>
        ${items.map(item => `
          <tr>
            <td>${item.name}</td>
            <td class="col-number text-danger">${item.qty_balance}</td>
            <td class="col-number">${item.min_stock}</td>
            <td>${item.uom_code || '—'}</td>
          </tr>
        `).join('')}
      </tbody>
    </table>
  `;
}

// ─────────────────────────────────────────────────────────────────────────────
// Chart rendering
// ─────────────────────────────────────────────────────────────────────────────

function initSalesChart(data) {
  const ctx = document.getElementById('sales-chart');
  if (!ctx || !window.Chart) return;

  const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
  const gridColor = isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)';
  const labelColor = isDark ? '#94A3B8' : '#64748B';

  _charts.sales = new Chart(ctx, {
    type: 'bar',
    data: {
      labels: data.labels,
      datasets: [
        {
          label: 'Sales',
          data: data.sales,
          backgroundColor: 'rgba(0, 114, 183, 0.8)',
          borderRadius: 4,
          borderSkipped: false,
        },
        {
          label: 'Purchases',
          data: data.purchases,
          backgroundColor: 'rgba(217, 119, 6, 0.7)',
          borderRadius: 4,
          borderSkipped: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: true,
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: {
          position: 'top',
          labels: {
            color: labelColor,
            usePointStyle: true,
            pointStyleWidth: 8,
            font: { size: 12, family: 'Inter' },
          },
        },
        tooltip: {
          backgroundColor: isDark ? '#1E293B' : '#FFFFFF',
          titleColor: isDark ? '#F1F5F9' : '#0F172A',
          bodyColor: isDark ? '#94A3B8' : '#475569',
          borderColor: isDark ? '#334155' : '#E2E8F0',
          borderWidth: 1,
          callbacks: {
            label: (ctx) => ` ${ctx.dataset.label}: ${formatAmount(ctx.raw)}`,
          },
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: { color: labelColor, font: { size: 11 } },
        },
        y: {
          grid: { color: gridColor },
          ticks: {
            color: labelColor,
            font: { size: 11 },
            callback: (v) => `${(v / 1000).toFixed(0)}K`,
          },
        },
      },
    },
  });
}

function initPurchaseChart(data) {
  const ctx = document.getElementById('purchase-chart');
  if (!ctx || !window.Chart) return;

  const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
  const labelColor = isDark ? '#94A3B8' : '#64748B';

  _charts.purchase = new Chart(ctx, {
    type: 'doughnut',
    data: {
      labels: data.labels,
      datasets: [{
        data: data.values,
        backgroundColor: [
          '#0072B7', '#D97706', '#16A34A', '#0891B2', '#7C3AED',
        ],
        borderColor: isDark ? '#1E293B' : '#FFFFFF',
        borderWidth: 2,
        hoverOffset: 6,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: true,
      cutout: '65%',
      plugins: {
        legend: {
          position: 'bottom',
          labels: {
            color: labelColor,
            padding: 16,
            font: { size: 11 },
            usePointStyle: true,
            pointStyleWidth: 8,
          },
        },
        tooltip: {
          backgroundColor: isDark ? '#1E293B' : '#FFFFFF',
          titleColor: isDark ? '#F1F5F9' : '#0F172A',
          bodyColor: isDark ? '#94A3B8' : '#475569',
          borderColor: isDark ? '#334155' : '#E2E8F0',
          borderWidth: 1,
          callbacks: {
            label: (ctx) => ` ${ctx.label}: ${ctx.raw}%`,
          },
        },
      },
    },
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Main init
// ─────────────────────────────────────────────────────────────────────────────

export async function init(container) {
  _destroyCharts();

  // Skeleton layout
  container.innerHTML = `
    <div class="page-header">
      <div>
        <h1 class="page-title">Dashboard</h1>
        <p class="page-subtitle">Welcome back, <strong id="dash-username">—</strong> — Today is ${new Date().toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })}</p>
      </div>
      <button class="btn btn-secondary btn-sm" id="dash-refresh">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/></svg>
        Refresh
      </button>
    </div>

    <!-- KPI skeleton -->
    <div class="kpi-grid" id="kpi-area">
      ${Array(6).fill(`<div class="kpi-card"><div class="skeleton skeleton-text w-1-2" style="height:12px;margin-bottom:12px"></div><div class="skeleton" style="height:32px;width:70%"></div></div>`).join('')}
    </div>

    <!-- Charts -->
    <div class="grid-2" style="margin-bottom:var(--space-6)" id="charts-area">
      <div class="chart-card"><div class="chart-body"><div class="skeleton skeleton-h32" style="height:180px"></div></div></div>
      <div class="chart-card"><div class="chart-body"><div class="skeleton skeleton-h32" style="height:180px"></div></div></div>
    </div>

    <!-- Tables -->
    <div class="grid-2" id="tables-area">
      <div class="card"><div class="card-header"><div class="card-title">Recent Sales</div></div><div class="card-body" style="padding:0"><div class="skeleton" style="height:200px;border-radius:0"></div></div></div>
      <div class="card"><div class="card-header"><div class="card-title">Recent Purchases</div></div><div class="card-body" style="padding:0"><div class="skeleton" style="height:200px;border-radius:0"></div></div></div>
    </div>
  `;

  // Set username
  const user = window.Auth?.getUser?.();
  const usernameEl = document.getElementById('dash-username');
  if (usernameEl && user) usernameEl.textContent = user.username;

  // Fetch data
  let data;
  try {
    data = await fetchDashboardData();
  } catch (err) {
    Toast.danger('Failed to load dashboard data. ' + (err.message || ''));
    data = {
      kpi: _mockKpi(),
      salesChart: _mockSalesChart(),
      purchaseChart: _mockPurchaseChart(),
      recentSales: [],
      recentPurchases: [],
      lowStock: [],
    };
  }

  // Render KPIs
  document.getElementById('kpi-area').outerHTML = `
    <div class="kpi-grid" id="kpi-area">
      ${renderKpiCards(data.kpi)}
    </div>
  `;
  // Note: outerHTML replaces the element, so re-query the container
  container.querySelector('#kpi-area') || container.insertAdjacentHTML('afterbegin', renderKpiCards(data.kpi));

  // Render charts
  document.getElementById('charts-area').innerHTML = `
    <div>${renderSalesChart(data.salesChart)}</div>
    <div>${renderPurchaseChart()}</div>
  `;

  // Init charts after DOM update
  requestAnimationFrame(() => {
    initSalesChart(data.salesChart);
    initPurchaseChart(data.purchaseChart);
  });

  // Render tables
  document.getElementById('tables-area').innerHTML = `
    <div class="card">
      <div class="card-header">
        <div class="card-title">Recent Sale Invoices</div>
        <a href="#/sales/si" class="btn btn-sm btn-secondary">View All</a>
      </div>
      <div style="overflow:auto">${renderRecentSales(data.recentSales)}</div>
    </div>
    <div class="card">
      <div class="card-header">
        <div class="card-title">Recent Purchase Invoices</div>
        <a href="#/purchases/pi" class="btn btn-sm btn-secondary">View All</a>
      </div>
      <div style="overflow:auto">${renderRecentPurchases(data.recentPurchases)}</div>
    </div>
  `;

  // Low stock section
  if (data.lowStock.length > 0) {
    container.insertAdjacentHTML('beforeend', `
      <div class="card" style="margin-top:var(--space-6)">
        <div class="card-header">
          <div class="card-title" style="color:var(--brand-danger)">
            ${ICONS.alert} Low Stock Alert (${data.lowStock.length} items)
          </div>
          <a href="#/masters/items" class="btn btn-sm btn-danger">Manage Items</a>
        </div>
        <div style="overflow:auto">${renderLowStock(data.lowStock)}</div>
      </div>
    `);
  }

  // Refresh button
  document.getElementById('dash-refresh')?.addEventListener('click', () => {
    init(container);
  });
}

export default { init };
