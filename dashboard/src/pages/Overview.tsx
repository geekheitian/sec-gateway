import { useEffect, useState } from 'preact/hooks';
import type { Health, Session } from '../api/client';
import { api } from '../api/client';

export function Overview() {
  const [health, setHealth] = useState<Health | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.health(), api.sessions()])
      .then(([h, s]) => {
        setHealth(h);
        setSessions(s);
        setLoading(false);
      })
      .catch((e) => {
        setError(e.message);
        setLoading(false);
      });
  }, []);

  if (loading) return <div class="page loading">Loading...</div>;
  if (error) return <div class="page error">Error: {error}</div>;

  return (
    <div class="page overview">
      <h1>Dashboard Overview</h1>
      
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-label">Service Status</div>
          <div class="stat-value status-ok">{health?.status}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Version</div>
          <div class="stat-value">{health?.version}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Active Sessions</div>
          <div class="stat-value">{sessions.length}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">FPE Backend</div>
          <div class="stat-value">{health?.service}</div>
        </div>
      </div>

      <div class="section">
        <h2>Recent Sessions</h2>
        {sessions.length === 0 ? (
          <p>No active sessions</p>
        ) : (
          <table class="sessions-table">
            <thead>
              <tr>
                <th>Session ID</th>
                <th>Created</th>
                <th>Last Accessed</th>
                <th>Requests</th>
              </tr>
            </thead>
            <tbody>
              {sessions.slice(0, 5).map((s) => (
                <tr key={s.id}>
                  <td class="session-id">{s.id}</td>
                  <td>{new Date(s.created_at).toLocaleString()}</td>
                  <td>{new Date(s.last_accessed).toLocaleString()}</td>
                  <td>{s.request_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
