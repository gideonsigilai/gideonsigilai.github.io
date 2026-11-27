export const API_BASE = (import.meta.env.PUBLIC_API_URL || process.env.API_URL || 'http://127.0.0.1:8052').replace(/\/$/, '');

export function absolutize(url: string): string {
  if (!url) return url;
  if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) return url;
  return `${API_BASE}${url.startsWith('/') ? '' : '/'}${url}`;
}

export async function toDataUri(url: string): Promise<string> {
  const u = absolutize(url);
  if (u.startsWith('data:')) return u;
  const res = await fetch(u);
  if (!res.ok) return u;
  const buf = await res.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  const b64 = btoa(binary);
  const type = res.headers.get('content-type') || 'application/octet-stream';
  return `data:${type};base64,${b64}`;
}
