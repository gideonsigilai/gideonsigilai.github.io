# Gideon Sigilai — portfolio

A fast, responsive portfolio with light and dark themes, restrained parallax, project case studies, a searchable journal, and contact and sharing controls.

## Run locally

No package installation or build step is required. From this project directory:

```sh
python3 -m http.server 8000 --directory dist
```

Open http://localhost:8000. Use a local web server because links and assets use paths relative to the website root.

## Source files

The files in `dist/` are the original, editable HTML, CSS, and JavaScript. They are not compiled or minified output. `dist/index.html` is the homepage; `dist/projects/` contains case studies and `dist/blog/` contains the journal and articles. Shared styling and behavior live in `dist/style.css`, `dist/script.js`, and `dist/theme.js`.

## Personalize

- Replace the labeled portrait and project image placeholders with your own optimized images and descriptive alternative text.
- Review the three starter journal articles before promoting them as your writing. They are editable static pages with links to relevant official documentation.
- Contact and profile links are in each page. The shared email constant is also in `script.js`.
- Update page titles, descriptions, canonical URLs, JSON-LD, `sitemap.xml`, and `feed.xml` when adding content or changing the domain.

## Contact and sharing

The conversation form validates input and prepares an email draft in the visitor's mail app. The visitor sends that email themselves. No mail server or submission database is configured. Copy-email, copy-draft, and WhatsApp provide alternatives. Sharing controls prepare links for the chosen platform; they do not publish automatically.

## Accessibility and performance

The site uses semantic, server-readable HTML, native modal dialogs, keyboard focus styles, touch-sized controls, and reduced-motion preferences. Parallax uses a single scheduled animation frame and smaller movement on phones. Static content remains readable before JavaScript loads.

## Hosting

Serve `dist/` as the site root. `.openai/hosting.json` contains the existing Sites project configuration. Current sharing remains private; search-engine indexing requires public access. Search metadata alone does not make a private site indexable.
