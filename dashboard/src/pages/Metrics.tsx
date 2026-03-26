import { useEffect, useState } from 'preact/hooks';
import { api } from '../api/client';

interface ParsedMetrics {
  total: number;
  blocked: number;
  pii: number;
  backend: string;
}

export function Metrics() {
  const [rawMetrics, setRawMetrics] = useState<string>('');
  const [parsed, setParsed] = useState<ParsedMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadMetrics = () => {
    setLoading(true);
    api.metrics()
      .then((text) => {
        setRawMetrics(text);
        parseMetrics(text);
        setError(null);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  };

  const parseMetrics = (text: string) => {
    const lines = text.split('\n');
    const metrics: ParsedMetrics = { total: 0, blocked: 0, pii: 0, backend: 'unknown' };

    for (const line of lines) {
      if (line.startsWith('sec_gateway_requests_total')) {
        const match = line.match(/(\d+)/);
        if (match) metrics.total = parseInt(match[1]);
      } else if (line.startsWith('sec_gateway_blocked_requests_total')) {
        const match = line.match(/(\d+)/);
        if (match) metrics.blocked = parseInt(match[1]);
      } else if (line.startsWith('sec_gateway_pii_detected_requests_total')) {
        const match = line.match(/(\d+)/);
        if (match) metrics.pii = parseInt(match[1]);
      } else if (line.startsWith('sec_gateway_fpe_backend')) {
        const match = line.match(/backend="([^"]+)"/);
        if (match) metrics.backend = match[1];
      }
    }

    setParsed(metrics);
  };

  useEffect(() => {
    loadMetrics();
    const interval = setInterval(loadMetrics, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !parsed) return <div class="page loading">Loading...</div>;

  return (
    <div class="page metrics">
      <h1>Metrics</h1>

      {error && (
        <div class="error-banner">
          Error: {error}
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-label">Total Requests</div>
          <div class="stat-value large">{parsed?.total ?? 0}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Blocked Requests</div>
          <div class="stat-value large">{parsed?.blocked ?? 0}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">PII Detected</div>
          <div class="stat-value large">{parsed?.pii ?? 0}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">FPE Backend</div>
          <div class="stat-value backend">{parsed?.backend ?? 'unknown'}</div>
        </div>
      </div>

      <div class="section">
        <div class="section-header">
          <h2>Prometheus Format</h2>
          <button class="refresh-btn" onClick={loadMetrics}>Refresh</button>
        </div>
        <pre class="metrics-raw">{rawMetrics}</pre>
      </div>
    </div>
  );
}
