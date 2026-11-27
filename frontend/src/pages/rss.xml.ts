import { API_BASE, apiFetch } from '../lib/api';

function esc(s: string): string {
  return String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

export async function GET(context: any) {
  const site = await apiFetch('/api/site');
  const postsRes = await apiFetch('/api/posts');
  const posts = postsRes.posts || [];
  const siteUrl = (context.site?.href || "https://gideonsigilai.github.io").replace(/\/$/, "");
  const author = site.profile.name;

  const items = posts.map((p: any) => {
    const link = `${siteUrl}/writing/${p.slug}`;
    const pub = p.published_at ? new Date(p.published_at).toUTCString() : new Date().toUTCString();
    return `    <item>
      <title>${esc(p.title)}</title>
      <link>${esc(link)}</link>
      <guid isPermaLink="true">${esc(link)}</guid>
      <description>${esc(p.excerpt || '')}</description>
      <pubDate>${pub}</pubDate>
      <author>${esc(site.profile.email || '')} (${esc(author)})</author>
      ${(p.tags || []).map((t: string) => `<category>${esc(t)}</category>`).join('')}
    </item>`;
  }).join('\n');

  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${esc(author)} — Journal</title>
    <link>${esc(siteUrl)}/writing</link>
    <description>${esc(site.profile.description || '')}</description>
    <language>en-ke</language>
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <atom:link href="${esc(siteUrl)}/rss.xml" rel="self" type="application/rss+xml" />
${items}
  </channel>
</rss>`;

  return new Response(xml, {
    headers: {
      'Content-Type': 'application/rss+xml; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
