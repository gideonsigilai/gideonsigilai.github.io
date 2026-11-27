export const API_BASE = (import.meta.env.PUBLIC_API_URL || process.env.API_URL || 'http://127.0.0.1:8052').replace(/\/$/, '');

export function absolutize(url: string): string {
  if (!url) return url;
  if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) return url;
  return `${API_BASE}${url.startsWith('/') ? '' : '/'}${url}`;
}

// Images are served by the API with long-lived immutable cache headers, so we
// reference them directly instead of inlining base64 data URIs. This keeps the
// HTML small and lets the browser cache and decode images independently.
export async function imageUrl(url: string): Promise<string> {
  return absolutize(url);
}
