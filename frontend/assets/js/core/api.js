/**
 * api.js — HTTP client for MemonBook ERP backend.
 * All requests go through here: JWT attach, error handling, token refresh.
 */

'use strict';

const API_BASE = 'http://127.0.0.1:8765/api';

// ─────────────────────────────────────────────────────────────────────────────
// Low-level fetch wrapper
// ─────────────────────────────────────────────────────────────────────────────

async function request(method, path, { body, params, signal } = {}) {
  const url = new URL(API_BASE + path, window.location.origin);

  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      if (v !== null && v !== undefined) url.searchParams.set(k, v);
    });
  }

  const token = Auth.getToken();
  const headers = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;

  let response;
  try {
    response = await fetch(url.toString(), {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      signal,
    });
  } catch (err) {
    if (err.name === 'AbortError') throw err;
    throw new APIError('Network error. Check if the backend is running.', 0);
  }

  // 401 → token expired → redirect to login
  if (response.status === 401) {
    Auth.logout();
    Router.navigate('/login');
    throw new APIError('Session expired. Please log in again.', 401);
  }

  let data;
  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    data = await response.json();
  } else {
    data = await response.text();
  }

  if (!response.ok) {
    const message = data?.detail || data?.message || `HTTP ${response.status}`;
    throw new APIError(message, response.status, data);
  }

  return data;
}

class APIError extends Error {
  constructor(message, status = 0, data = null) {
    super(message);
    this.name    = 'APIError';
    this.status  = status;
    this.data    = data;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP methods
// ─────────────────────────────────────────────────────────────────────────────

const Api = {
  get:    (path, opts = {}) => request('GET',    path, opts),
  post:   (path, body, opts = {}) => request('POST',  path, { ...opts, body }),
  put:    (path, body, opts = {}) => request('PUT',   path, { ...opts, body }),
  patch:  (path, body, opts = {}) => request('PATCH', path, { ...opts, body }),
  delete: (path, opts = {}) => request('DELETE', path, opts),

  // File download
  async download(path, filename) {
    const token = Auth.getToken();
    const headers = {};
    if (token) headers['Authorization'] = `Bearer ${token}`;

    const response = await fetch(API_BASE + path, { headers });
    if (!response.ok) throw new APIError('Download failed', response.status);

    const blob = await response.blob();
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Module-specific API methods
// ─────────────────────────────────────────────────────────────────────────────

const AuthApi = {
  login:          (body) => Api.post('/auth/login', body),
  logout:         ()     => Api.post('/auth/logout', {}),
  me:             ()     => Api.get('/auth/me'),
  changePassword: (body) => Api.post('/auth/change-password', body),
};

const DeviceApi = {
  getHash:        ()     => Api.get('/device/hash'),
  register:       (body) => Api.post('/device/register', body),
  list:           ()     => Api.get('/device/list'),
  revoke:         (id)   => Api.delete(`/device/${id}`),
};

const MastersApi = {
  coa: {
    list:   (p = {}) => Api.get('/masters/coa/', { params: p }),
    get:    (id)     => Api.get(`/masters/coa/${id}`),
    create: (b)      => Api.post('/masters/coa/', b),
    update: (id, b)  => Api.put(`/masters/coa/${id}`, b),
    delete: (id)     => Api.delete(`/masters/coa/${id}`),
  },
  customers: {
    list:   (p = {}) => Api.get('/masters/customers/', { params: p }),
    get:    (id)     => Api.get(`/masters/customers/${id}`),
    create: (b)      => Api.post('/masters/customers/', b),
    update: (id, b)  => Api.put(`/masters/customers/${id}`, b),
    delete: (id)     => Api.delete(`/masters/customers/${id}`),
  },
  vendors: {
    list:   (p = {}) => Api.get('/masters/vendors/', { params: p }),
    get:    (id)     => Api.get(`/masters/vendors/${id}`),
    create: (b)      => Api.post('/masters/vendors/', b),
    update: (id, b)  => Api.put(`/masters/vendors/${id}`, b),
    delete: (id)     => Api.delete(`/masters/vendors/${id}`),
  },
  items: {
    list:     (p = {}) => Api.get('/masters/items/', { params: p }),
    get:      (id)     => Api.get(`/masters/items/${id}`),
    dropdown: ()       => Api.get('/masters/items/dropdown'),
    create:   (b)      => Api.post('/masters/items/', b),
    update:   (id, b)  => Api.put(`/masters/items/${id}`, b),
    delete:   (id)     => Api.delete(`/masters/items/${id}`),
  },
  uom:          { list: (p={}) => Api.get('/masters/uom', { params: p }) },
  currencies:   { list: (p={}) => Api.get('/masters/currencies', { params: p }) },
  paymentTerms: {
    list:   (p={}) => Api.get('/masters/payment-terms', { params: p }),
    create: (b)    => Api.post('/masters/payment-terms', b),
  },
  locations: {
    list:   (p={}) => Api.get('/masters/locations', { params: p }),
    create: (b)    => Api.post('/masters/locations', b),
    update: (id,b) => Api.put(`/masters/locations/${id}`, b),
  },
};

const PurchasesApi = {
  po:  {
    list:    (p = {}) => Api.get('/purchases/po/', { params: p }),
    get:     (id)     => Api.get(`/purchases/po/${id}`),
    create:  (b)      => Api.post('/purchases/po/', b),
    update:  (id, b)  => Api.put(`/purchases/po/${id}`, b),
    submit:  (id)     => Api.post(`/purchases/po/${id}/submit`, {}),
    approve: (id)     => Api.post(`/purchases/po/${id}/approve`, {}),
    cancel:  (id)     => Api.post(`/purchases/po/${id}/cancel`, {}),
    delete:  (id)     => Api.delete(`/purchases/po/${id}`),
  },
  grn: {
    list:   (p = {}) => Api.get('/purchases/grn/', { params: p }),
    get:    (id)     => Api.get(`/purchases/grn/${id}`),
    create: (b)      => Api.post('/purchases/grn/', b),
    accept: (id)     => Api.post(`/purchases/grn/${id}/accept`, {}),
    cancel: (id)     => Api.post(`/purchases/grn/${id}/cancel`, {}),
  },
  pi:  {
    list:   (p = {}) => Api.get('/purchases/pi/', { params: p }),
    get:    (id)     => Api.get(`/purchases/pi/${id}`),
    create: (b)      => Api.post('/purchases/pi/', b),
    update: (id, b)  => Api.put(`/purchases/pi/${id}`, b),
    post:   (id)     => Api.post(`/purchases/pi/${id}/post`, {}),
    cancel: (id)     => Api.post(`/purchases/pi/${id}/cancel`, {}),
  },
  pr:  {
    list:   (p = {}) => Api.get('/purchases/pr/', { params: p }),
    create: (b)      => Api.post('/purchases/pr/', b),
    post:   (id)     => Api.post(`/purchases/pr/${id}/post`, {}),
  },
};

const SalesApi = {
  so: {
    list:   (p = {}) => Api.get('/sales/so/', { params: p }),
    get:    (id)     => Api.get(`/sales/so/${id}`),
    create: (b)      => Api.post('/sales/so/', b),
    update: (id, b)  => Api.put(`/sales/so/${id}`, b),
    submit: (id)     => Api.post(`/sales/so/${id}/submit`, {}),
    approve:(id)     => Api.post(`/sales/so/${id}/approve`, {}),
    cancel: (id)     => Api.post(`/sales/so/${id}/cancel`, {}),
    delete: (id)     => Api.delete(`/sales/so/${id}`),
  },
  dc: {
    list:   (p = {}) => Api.get('/sales/dc/', { params: p }),
    get:    (id)     => Api.get(`/sales/dc/${id}`),
    create: (b)      => Api.post('/sales/dc/', b),
    post:   (id)     => Api.post(`/sales/dc/${id}/post`, {}),
    cancel: (id)     => Api.post(`/sales/dc/${id}/cancel`, {}),
  },
  si: {
    list:   (p = {}) => Api.get('/sales/si/', { params: p }),
    get:    (id)     => Api.get(`/sales/si/${id}`),
    create: (b)      => Api.post('/sales/si/', b),
    update: (id, b)  => Api.put(`/sales/si/${id}`, b),
    post:   (id)     => Api.post(`/sales/si/${id}/post`, {}),
    cancel: (id)     => Api.post(`/sales/si/${id}/cancel`, {}),
  },
  sr: {
    list:   (p = {}) => Api.get('/sales/sr/', { params: p }),
    create: (b)      => Api.post('/sales/sr/', b),
    post:   (id)     => Api.post(`/sales/sr/${id}/post`, {}),
  },
};

const PaymentsApi = {
  receipts: {
    list:   (p = {}) => Api.get('/payments/receipts/', { params: p }),
    get:    (id)     => Api.get(`/payments/receipts/${id}`),
    create: (b)      => Api.post('/payments/receipts/', b),
    post:   (id)     => Api.post(`/payments/receipts/${id}/post`, {}),
    cancel: (id)     => Api.post(`/payments/receipts/${id}/cancel`, {}),
  },
  payments: {
    list:   (p = {}) => Api.get('/payments/payments/', { params: p }),
    get:    (id)     => Api.get(`/payments/payments/${id}`),
    create: (b)      => Api.post('/payments/payments/', b),
    post:   (id)     => Api.post(`/payments/payments/${id}/post`, {}),
    cancel: (id)     => Api.post(`/payments/payments/${id}/cancel`, {}),
  },
  jv: {
    list:    (p = {}) => Api.get('/accounts/jv/', { params: p }),
    get:     (id)     => Api.get(`/accounts/jv/${id}`),
    create:  (b)      => Api.post('/accounts/jv/', b),
    post:    (id)     => Api.post(`/accounts/jv/${id}/post`, {}),
    reverse: (id)     => Api.post(`/accounts/jv/${id}/reverse`, {}),
  },
};

const AccountsApi = {
  ledger:       (p = {})  => Api.get('/accounts/ledger', { params: p }),
  trialBalance: (p = {})  => Api.get('/accounts/trial-balance', { params: p }),
  balance:      (id, p={}) => Api.get(`/accounts/balance/${id}`, { params: p }),
};

const HrApi = {
  employees: {
    list:       (p = {}) => Api.get('/hr/employees/', { params: p }),
    get:        (id)     => Api.get(`/hr/employees/${id}`),
    dropdown:   ()       => Api.get('/hr/employees/dropdown'),
    create:     (b)      => Api.post('/hr/employees/', b),
    update:     (id, b)  => Api.put(`/hr/employees/${id}`, b),
    deactivate: (id)     => Api.post(`/hr/employees/${id}/deactivate`, {}),
  },
  attendance: {
    list:    (p = {}) => Api.get('/hr/attendance/', { params: p }),
    mark:    (b)      => Api.post('/hr/attendance/', b),
    bulk:    (b)      => Api.post('/hr/attendance/bulk', b),
    summary: (id, p={}) => Api.get(`/hr/attendance/summary/${id}`, { params: p }),
    delete:  (id)     => Api.delete(`/hr/attendance/${id}`),
  },
  payroll: {
    list:    (p = {}) => Api.get('/hr/payroll/', { params: p }),
    get:     (id)     => Api.get(`/hr/payroll/${id}`),
    run:     (b)      => Api.post('/hr/payroll/', b),
    approve: (id)     => Api.post(`/hr/payroll/${id}/approve`, {}),
    post:    (id)     => Api.post(`/hr/payroll/${id}/post`, {}),
    cancel:  (id)     => Api.post(`/hr/payroll/${id}/cancel`, {}),
    payslip: (runId, empId) => Api.get(`/hr/payroll/${runId}/payslip/${empId}`),
  },
};

const ReportsApi = {
  list: () => Api.get('/reports/'),
};

export {
  Api,
  APIError,
  AuthApi,
  DeviceApi,
  MastersApi,
  PurchasesApi,
  SalesApi,
  PaymentsApi,
  AccountsApi,
  HrApi,
  ReportsApi,
};
