import { useEffect, useState } from 'preact/hooks';
import type { Session } from '../api/client';
import { api } from '../api/client';

export function Sessions() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const loadSessions = () => {
    api.sessions()
      .then(setSessions)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadSessions();
  }, []);

  const handleDelete = async (id: string) => {
    setDeleting(id);
    try {
      await api.deleteSession(id);
      setSessions(sessions.filter((s) => s.id !== id));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeleting(null);
    }
  };

  if (loading) return <div class="page loading">Loading...</div>;

  return (
    <div class="page sessions">
      <h1>Session Management</h1>

      {error && (
        <div class="error-banner">
          Error: {error}
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div class="section">
        <div class="section-header">
          <h2>Active Sessions ({sessions.length})</h2>
          <button class="refresh-btn" onClick={loadSessions}>
            Refresh
          </button>
        </div>

        {sessions.length === 0 ? (
          <p class="empty-state">No active sessions</p>
        ) : (
          <table class="sessions-table">
            <thead>
              <tr>
                <th>Session ID</th>
                <th>Created</th>
                <th>Last Accessed</th>
                <th>Requests</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s) => (
                <tr key={s.id}>
                  <td class="session-id">{s.id}</td>
                  <td>{new Date(s.created_at).toLocaleString()}</td>
                  <td>{new Date(s.last_accessed).toLocaleString()}</td>
                  <td>{s.request_count}</td>
                  <td>
                    <button
                      class="delete-btn"
                      onClick={() => handleDelete(s.id)}
                      disabled={deleting === s.id}
                    >
                      {deleting === s.id ? 'Deleting...' : 'Delete'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
