import type { ComponentChildren } from 'preact';
import { useNavigation } from '../Navigation';

interface LayoutProps {
  children: ComponentChildren;
}

export function Layout({ children }: LayoutProps) {
  const { currentPage, navigate } = useNavigation();

  return (
    <div class="layout">
      <aside class="sidebar">
        <div class="sidebar-header">
          <h1>sec-gateway</h1>
          <span class="badge">Dashboard</span>
        </div>
        <nav class="sidebar-nav">
          <button
            class={`nav-link ${currentPage === 'overview' ? 'active' : ''}`}
            onClick={() => navigate('overview')}
          >
            Overview
          </button>
          <button
            class={`nav-link ${currentPage === 'sessions' ? 'active' : ''}`}
            onClick={() => navigate('sessions')}
          >
            Sessions
          </button>
          <button
            class={`nav-link ${currentPage === 'metrics' ? 'active' : ''}`}
            onClick={() => navigate('metrics')}
          >
            Metrics
          </button>
        </nav>
        <div class="sidebar-footer">
          <a href="https://github.com/geekheitian/sec-gateway" target="_blank" rel="noopener">
            GitHub
          </a>
        </div>
      </aside>
      <main class="main-content">{children}</main>
    </div>
  );
}
