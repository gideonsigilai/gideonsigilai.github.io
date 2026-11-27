import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
  site: process.env.SITE_URL || 'https://gideonsigilai.github.io',
  base: process.env.BASE_PATH || '/',
  trailingSlash: 'never',
  compressHTML: true,
  prefetch: {
    prefetchAll: false,
    defaultStrategy: 'viewport',
  },
  image: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '217.77.8.87',
      },
      {
        protocol: 'http',
        hostname: '127.0.0.1',
      },
    ],
  },
  build: {
    format: 'file',
  },
});
