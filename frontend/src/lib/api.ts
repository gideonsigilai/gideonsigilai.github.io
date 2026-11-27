export const API_BASE = (import.meta.env.PUBLIC_API_URL || process.env.API_URL || 'http://127.0.0.1:8052').replace(/\/$/, '');

export async function apiFetch<T = any>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${path}`);
  return res.json() as Promise<T>;
}
