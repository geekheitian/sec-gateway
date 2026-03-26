const API_BASE = 'http://localhost:8080';

export interface Health {
  status: string;
  service: string;
  version: string;
}

export interface Session {
  id: string;
  created_at: string;
  last_accessed: string;
  request_count: number;
}

export interface Metrics {
  total: number;
  blocked: number;
  pii: number;
  backend: string;
}

export const api = {
  health: async (): Promise<Health> => {
    const res = await fetch(`${API_BASE}/health`);
    return res.json();
  },

  sessions: async (): Promise<Session[]> => {
    const res = await fetch(`${API_BASE}/sessions`);
    return res.json();
  },

  deleteSession: async (id: string): Promise<void> => {
    await fetch(`${API_BASE}/sessions/${id}`, { method: 'DELETE' });
  },

  metrics: async (): Promise<string> => {
    const res = await fetch(`${API_BASE}/metrics`);
    return res.text();
  },
};
