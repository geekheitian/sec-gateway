const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:8080';

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

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

async function handleTextResponse(res: Response): Promise<string> {
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  return res.text();
}

export const api = {
  health: async (): Promise<Health> => {
    const res = await fetch(`${API_BASE}/health`);
    return handleResponse<Health>(res);
  },

  sessions: async (): Promise<Session[]> => {
    const res = await fetch(`${API_BASE}/sessions`);
    return handleResponse<Session[]>(res);
  },

  deleteSession: async (id: string): Promise<void> => {
    const res = await fetch(`${API_BASE}/sessions/${id}`, { method: 'DELETE' });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${res.statusText}`);
    }
  },

  metrics: async (): Promise<string> => {
    const res = await fetch(`${API_BASE}/metrics`);
    return handleTextResponse(res);
  },
};
