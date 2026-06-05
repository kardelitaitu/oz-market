import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const docsDir = path.resolve(__dirname, '../../docs');

function docsServePlugin() {
  return {
    name: 'docs-serve',
    configureServer(server) {
      server.middlewares.use('/docs', (req, res, next) => {
        const filePath = path.join(docsDir, req.url || '');
        if (fs.existsSync(filePath) && fs.statSync(filePath).isFile()) {
          const content = fs.readFileSync(filePath, 'utf-8');
          const ext = path.extname(filePath).toLowerCase();
          const mimes = { '.md': 'text/markdown', '.html': 'text/html', '.json': 'application/json', '.yaml': 'text/yaml', '.yml': 'text/yaml', '.png': 'image/png', '.svg': 'image/svg+xml' };
          res.writeHead(200, { 'Content-Type': mimes[ext] || 'text/plain' });
          res.end(content);
        } else {
          next();
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), docsServePlugin()],
  server: {
    historyApiFallback: true,
  },
  preview: {
    host: '127.0.0.1',
  },
})
