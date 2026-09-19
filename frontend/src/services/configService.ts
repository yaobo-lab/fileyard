import { AppConfig } from '../types/app';

export const createConfigService = (authFetch: (url: string, options?: RequestInit) => Promise<Response>) => {
  return {
    async getConfigList(params: {
      page?: number;
      limit?: number;
      search?: string;
      key?: string;
      name?: string;
    } = {}): Promise<{ items: AppConfig[]; total: number; page: number; limit: number }> {
      const query = new URLSearchParams();
      if (params.page) query.set('curPage', params.page.toString());
      if (params.limit) query.set('pageSize', params.limit.toString());
      if (params.search) query.set('search', params.search);
      if (params.key) query.set('key', params.key);
      if (params.name) query.set('name', params.name);

      const res = await authFetch(`/api/config/page?${query.toString()}`);
      if (!res.ok) throw new Error(`Failed to load config list: ${res.statusText}`);
      const json = await res.json();
      return json.data || { items: [], total: 0, page: 1, limit: 10 };
    },

    async getConfig(id: number): Promise<AppConfig> {
      const res = await authFetch(`/api/config?id=${id}`);
      if (!res.ok) throw new Error(`Failed to load config: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async saveConfig(payload: Partial<AppConfig>): Promise<AppConfig> {
      const res = await authFetch('/api/config', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to save config: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async deleteConfig(id: number): Promise<void> {
      const res = await authFetch(`/api/config?id=${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to delete config: ${res.statusText}`);
    },
  };
};
