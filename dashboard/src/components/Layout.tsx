import type { ComponentChildren } from 'preact';

interface LayoutProps {
  children: ComponentChildren;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div class="layout">
      <aside class="sidebar">
        <div class="sidebar-header">
          <h1>sec-gateway</h1>
          <span class="badge">Dashboard</span>
        </div>
        <nav class="sidebar-nav">
          <a href="/">Overview</a>
          <a href="/sessions">Sessions</a>
          <a href="/metrics">Metrics</a>
        </nav>
        <div class="sidebar-footer">
          <a href="https://github.com/geekheitian/sec-gateway" target="_blank">
            GitHub
          </a>
        </div>
      </aside>
      <main class="main-content">{children}</main>
    </div>
  );
}
