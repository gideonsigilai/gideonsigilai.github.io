import { apiFetch } from '../lib/api';

function esc(s: string): string {
  return String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export async function GET(context: any) {
  const siteUrl = (context.site?.href || 'https://gideonsigilai.github.io').replace(/\/$/, '');
  const [projectsRes, postsRes] = await Promise.all([
    apiFetch('/api/projects'),
    apiFetch('/api/posts'),
  ]);
  const projects = projectsRes.projects || [];
  const posts = postsRes.posts || [];

  const urls: { loc: string; lastmod?: string; priority?: string }[] = [
    { loc: `${siteUrl}/`, priority: '1.0' },
    { loc: `${siteUrl}/writing`, priority: '0.8' },
  ];
  for (const p of projects) urls.push({ loc: `${siteUrl}/work/${p.slug}`, priority: '0.7' });
  for (const p of posts) urls.push({ loc: `${siteUrl}/writing/${p.slug}`, lastmod: p.published_at, priority: '0.6' });

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((u) => `  <url>
    <loc>${esc(u.loc)}</loc>${u.lastmod ? `\n    <lastmod>${new Date(u.lastmod).toISOString().slice(0, 10)}</lastmod>` : ''}
    <changefreq>weekly</changefreq>
    <priority>${u.priority || '0.5'}</priority>
  </url>`).join('\n')}
</urlset>`;

  return new Response(body, {
    headers: {
      'Content-Type': 'application/xml; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
