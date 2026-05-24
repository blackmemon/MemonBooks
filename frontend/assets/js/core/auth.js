/**
 * auth.js — Authentication state management for MemonBook ERP.
 * Handles token storage, user session, device hash, permissions.
 */

'use strict';

const _TOKEN_KEY       = 'mb_token';
const _USER_KEY        = 'mb_user';
const _PERMISSIONS_KEY = 'mb_permissions';
const _THEME_KEY       = 'mb_theme';

const Auth = {
  // ── Token management ──────────────────────────────────────────
  getToken() {
    return sessionStorage.getItem(_TOKEN_KEY) || localStorage.getItem(_TOKEN_KEY);
  },

  setToken(token, persistent = false) {
    sessionStorage.setItem(_TOKEN_KEY, token);
    if (persistent) localStorage.setItem(_TOKEN_KEY, token);
  },

  removeToken() {
    sessionStorage.removeItem(_TOKEN_KEY);
    localStorage.removeItem(_TOKEN_KEY);
  },

  isLoggedIn() {
    const token = this.getToken();
    if (!token) return false;
    try {
      const payload = JSON.parse(atob(token.split('.')[1]));
      return payload.exp * 1000 > Date.now();
    } catch {
      return false;
    }
  },

  getPayload() {
    const token = this.getToken();
    if (!token) return null;
    try {
      return JSON.parse(atob(token.split('.')[1]));
    } catch {
      return null;
    }
  },

  // ── User info ─────────────────────────────────────────────────
  getUser() {
    const raw = sessionStorage.getItem(_USER_KEY);
    return raw ? JSON.parse(raw) : null;
  },

  setUser(user) {
    sessionStorage.setItem(_USER_KEY, JSON.stringify(user));
  },

  removeUser() {
    sessionStorage.removeItem(_USER_KEY);
  },

  getUserId()   { return this.getPayload()?.sub; },
  getUsername() { return this.getPayload()?.username; },
  getRole()     { return this.getPayload()?.role; },

  // ── Permissions ───────────────────────────────────────────────
  setPermissions(perms) {
    sessionStorage.setItem(_PERMISSIONS_KEY, JSON.stringify(perms));
  },

  getPermissions() {
    const raw = sessionStorage.getItem(_PERMISSIONS_KEY);
    return raw ? JSON.parse(raw) : {};
  },

  can(module, action) {
    const perms = this.getPermissions();
    return (perms[module] || []).includes(action);
  },

  isAdmin()   { return this.getRole() === 'admin'; },
  isManager() { return ['admin', 'manager'].includes(this.getRole()); },

  // ── Device hash ───────────────────────────────────────────────
  async getDeviceHash() {
    // Try Tauri command first (desktop)
    if (window.__TAURI__) {
      try {
        const { invoke } = window.__TAURI__.tauri;
        return await invoke('get_desktop_device_hash', {
          backendUrl: 'http://127.0.0.1:8765',
        });
      } catch { /* fallback */ }
    }
    // Fallback: ask backend directly
    try {
      const res = await fetch('http://127.0.0.1:8765/api/device/hash');
      const data = await res.json();
      return data.device_hash;
    } catch {
      // Last resort: generate from browser fingerprint (not secure, dev only)
      return 'DEV_' + navigator.userAgent.replace(/[^a-zA-Z0-9]/g, '').slice(0, 32);
    }
  },

  // ── Login / Logout ────────────────────────────────────────────
  async login(username, password) {
    const deviceHash = await this.getDeviceHash();

    const { AuthApi } = await import('./api.js');
    const result = await AuthApi.login({ username, password, device_hash: deviceHash });

    this.setToken(result.access_token);
    this.setUser({
      user_id:    result.user_id,
      username:   result.username,
      role:       result.role,
      db_context: result.db_context,
    });
    this.setPermissions(result.permissions || {});
    return result;
  },

  logout() {
    this.removeToken();
    this.removeUser();
    sessionStorage.removeItem(_PERMISSIONS_KEY);
  },

  // ── Theme ─────────────────────────────────────────────────────
  getTheme() {
    return localStorage.getItem(_THEME_KEY) ||
      (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  },

  setTheme(theme) {
    localStorage.setItem(_THEME_KEY, theme);
    document.documentElement.setAttribute('data-theme', theme);
  },

  toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme') || this.getTheme();
    this.setTheme(current === 'dark' ? 'light' : 'dark');
  },

  applyTheme() {
    this.setTheme(this.getTheme());
  },
};

// Make Auth globally available (accessed by api.js before import resolves)
window.Auth = Auth;

export default Auth;
