// API client for the super-admin surface (`/api/admin/*`). All calls go
// through the same-origin proxy so the session cookie is forwarded; the
// backend `AdminUser` guard enforces authorization (403 for non-admins).

import type { AppSettings } from './AppSettings';
import type { AdminEquipmentPage } from './AdminEquipmentPage';
import type { EquipmentItemDetail } from './EquipmentItemDetail';
import type { EquipmentSpecsPayload } from './EquipmentSpecsPayload';

const API_BASE: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

type FetchFn = typeof fetch;

export async function fetchSettings(f: FetchFn): Promise<AppSettings> {
  const r = await f(`${API_BASE}/api/admin/settings`, { credentials: 'include' });
  if (!r.ok) throw new Error(`fetchSettings ${r.status}`);
  return (await r.json()) as AppSettings;
}

export async function updateSettings(f: FetchFn, body: AppSettings): Promise<AppSettings> {
  const r = await f(`${API_BASE}/api/admin/settings`, {
    method: 'PUT',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
  if (!r.ok) throw new Error(`updateSettings ${r.status}`);
  return (await r.json()) as AppSettings;
}

export async function fetchEquipment(
  f: FetchFn,
  opts: { kind?: string; q?: string; page?: number } = {}
): Promise<AdminEquipmentPage> {
  const params = new URLSearchParams();
  if (opts.kind) params.set('kind', opts.kind);
  if (opts.q) params.set('q', opts.q);
  if (opts.page) params.set('page', String(opts.page));
  const qs = params.toString();
  const r = await f(`${API_BASE}/api/admin/equipment${qs ? `?${qs}` : ''}`, {
    credentials: 'include'
  });
  if (!r.ok) throw new Error(`fetchEquipment ${r.status}`);
  return (await r.json()) as AdminEquipmentPage;
}

/** Full item incl. specs — reuses the public detail endpoint (read-only). */
export async function fetchEquipmentItem(f: FetchFn, id: string): Promise<EquipmentItemDetail> {
  const r = await f(`${API_BASE}/api/equipment/items/${id}`, { credentials: 'include' });
  if (!r.ok) throw new Error(`fetchEquipmentItem ${r.status}`);
  return (await r.json()) as EquipmentItemDetail;
}

export interface EquipmentEdit {
  brand?: string;
  model?: string;
  variant?: string;
  display_name?: string;
  status?: string;
  /** Replace the per-kind specs row (tagged `{ kind, ...fields }`). */
  specs?: EquipmentSpecsPayload;
}

export async function editEquipment(f: FetchFn, id: string, patch: EquipmentEdit): Promise<void> {
  const r = await f(`${API_BASE}/api/admin/equipment/${id}`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(patch)
  });
  if (!r.ok) {
    const detail = await r.text().catch(() => '');
    throw new Error(`editEquipment ${r.status}${detail ? `: ${detail.slice(0, 200)}` : ''}`);
  }
}

export async function deleteEquipment(f: FetchFn, id: string): Promise<void> {
  const r = await f(`${API_BASE}/api/admin/equipment/${id}`, {
    method: 'DELETE',
    credentials: 'include'
  });
  if (!r.ok) {
    // 409 = item still referenced by photos/setups.
    throw new Error(r.status === 409 ? 'in_use' : `deleteEquipment ${r.status}`);
  }
}
