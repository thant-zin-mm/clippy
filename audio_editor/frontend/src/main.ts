import './styles.css';

import { createApiClient, type ApiClient } from './api/client';

const LOGO_SVG = `
  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M1 8h2l2-4 2 8 2-9 2 6 2-3h2" />
  </svg>`;

function requireElement<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error(`missing ${selector} element`);
  }
  return element;
}

const app = requireElement<HTMLDivElement>('#app');

app.innerHTML = `
  <div class="app-shell">
    <header class="toolbar">
      <div class="brand">
        <span class="logo">${LOGO_SVG}</span>
        <span class="brand-name">Silence Cutter</span>
      </div>
      <div class="toolbar-spacer"></div>
      <button id="health-badge" type="button" class="badge badge--warning" title="Retry backend check" aria-live="polite">
        checking backend…
      </button>
    </header>
    <div class="body">
      <aside class="sidebar">
        <section class="panel">
          <h2 class="panel-title">Silence detection</h2>
          <p class="muted">Panel arrives in task P1-T4.</p>
        </section>
      </aside>
      <main class="stage">
        <div class="stage-placeholder">
          <p class="muted">Waveform arrives in task P1-T2. Import starts there.</p>
        </div>
      </main>
    </div>
    <footer class="status-bar">
      <span class="muted">candidates <span class="mono">0</span> · cut <span class="mono">0</span> · kept <span class="mono">0</span></span>
      <span class="mono muted">ready</span>
    </footer>
  </div>`;

const badge = requireElement<HTMLButtonElement>('#health-badge');

async function refreshHealth(client: ApiClient): Promise<void> {
  badge.className = 'badge badge--warning';
  badge.textContent = 'checking backend…';
  try {
    const health = await client.health();
    badge.className = 'badge badge--success';
    badge.textContent = health.ok ? `backend connected · v${health.version}` : 'backend not ok';
    badge.title = `API base: ${client.apiBase} — click to recheck`;
  } catch (error) {
    const message = error instanceof Error ? error.message : 'backend unreachable';
    badge.className = 'badge badge--danger';
    badge.textContent = message;
    // Full hint in the tooltip: the badge text is CSS-truncated (ui-guide §5).
    badge.title = `${message} — click to recheck`;
  }
}

const client = createApiClient();
void refreshHealth(client);
badge.addEventListener('click', () => void refreshHealth(client));
