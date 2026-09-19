import { AppItem, AppClass, AppDeploy, AppUser, GitlabBranch, GitlabPipeline } from '../types/app';

export const createAppService = (authFetch: (url: string, options?: RequestInit) => Promise<Response>) => {
  return {
    // ===== 应用基础接口 =====
    async getAppList(params: {
      page?: number;
      limit?: number;
      status?: number;
      class_no?: string;
      ownerId?: string;
      search?: string;
    }): Promise<{ items: AppItem[]; total: number; page: number; limit: number }> {
      const query = new URLSearchParams();
      if (params.page) query.set('curPage', params.page.toString());
      if (params.limit) query.set('pageSize', params.limit.toString());
      if (params.status !== undefined && params.status !== -1) query.set('status', params.status.toString());
      if (params.class_no) query.set('class_no', params.class_no);
      if (params.ownerId) query.set('ownerId', params.ownerId);
      if (params.search) query.set('search', params.search);

      const res = await authFetch(`/api/app/page?${query.toString()}`);
      if (!res.ok) throw new Error(`Failed to load app list: ${res.statusText}`);
      const json = await res.json();
      return json.data || { items: [], total: 0, page: 1, limit: 10 };
    },

    async getApp(appno: string): Promise<AppItem> {
      const res = await authFetch(`/api/app/${appno}`);
      if (!res.ok) throw new Error(`Failed to load app: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async createApp(payload: Partial<AppItem>): Promise<AppItem> {
      const res = await authFetch('/api/app/create', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to create app: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async saveAppBasic(appno: string, payload: Partial<AppItem>): Promise<AppItem> {
      const res = await authFetch(`/api/app/${appno}/save/basic`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to save basic app info: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async saveAppCharge(appno: string, payload: { createby_id: string; createby_name: string }): Promise<AppItem> {
      const res = await authFetch(`/api/app/${appno}/save/charge`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to save charge: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async deleteApp(appno: string): Promise<void> {
      const res = await authFetch(`/api/app/${appno}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to delete app: ${res.statusText}`);
    },

    // ===== 应用分类接口 =====
    async getClassList(params: { page?: number; limit?: number; name?: string } = {}): Promise<{
      items: AppClass[];
      total: number;
    }> {
      const query = new URLSearchParams();
      if (params.page) query.set('curPage', params.page.toString());
      if (params.limit) query.set('pageSize', params.limit.toString());
      if (params.name) query.set('name', params.name);

      const res = await authFetch(`/api/app/class/pages?${query.toString()}`);
      if (!res.ok) throw new Error(`Failed to load classes: ${res.statusText}`);
      const json = await res.json();
      return json.data || { items: [], total: 0 };
    },

    async saveClass(payload: Partial<AppClass>): Promise<AppClass> {
      const res = await authFetch('/api/app/class', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to save class: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async deleteClass(id: number): Promise<void> {
      const res = await authFetch(`/api/app/class?id=${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to delete class: ${res.statusText}`);
    },

    // ===== 项目成员接口 =====
    async getAppUsers(appno: string): Promise<AppUser[]> {
      const res = await authFetch(`/api/app/${appno}/user/list`);
      if (!res.ok) throw new Error(`Failed to load app users: ${res.statusText}`);
      const json = await res.json();
      return json.data || [];
    },

    async createAppUser(appno: string, payload: { uid: number; uname: string; key?: string }): Promise<AppUser> {
      const res = await authFetch(`/api/app/${appno}/user/create`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to add user: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async deleteAppUser(appno: string, id: number): Promise<void> {
      const res = await authFetch(`/api/app/${appno}/user?id=${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to remove user: ${res.statusText}`);
    },

    // ===== 部署环境接口 =====
    async getDeployList(appno: string): Promise<AppDeploy[]> {
      const res = await authFetch(`/api/app/${appno}/deploy/list`);
      if (!res.ok) throw new Error(`Failed to load deploys: ${res.statusText}`);
      const json = await res.json();
      return json.data || [];
    },

    async createDeploy(appno: string, payload: Partial<AppDeploy>): Promise<AppDeploy> {
      const res = await authFetch(`/api/app/${appno}/deploy/create`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to create deploy: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async saveDeploy(appno: string, payload: Partial<AppDeploy>): Promise<AppDeploy> {
      const res = await authFetch(`/api/app/${appno}/deploy/save`, {
        method: 'POST',
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(`Failed to save deploy: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async copyDeploy(appno: string, id: number): Promise<AppDeploy> {
      const res = await authFetch(`/api/app/${appno}/deploy/copy?id=${id}`);
      if (!res.ok) throw new Error(`Failed to copy deploy: ${res.statusText}`);
      const json = await res.json();
      return json.data;
    },

    async deleteDeploy(appno: string, id: number): Promise<void> {
      const res = await authFetch(`/api/app/${appno}/deploy?id=${id}`, {
        method: 'DELETE',
      });
      if (!res.ok) throw new Error(`Failed to delete deploy: ${res.statusText}`);
    },

    // ===== GitLab CI/CD 接口 =====
    async getBranches(appno: string): Promise<GitlabBranch[]> {
      const res = await authFetch(`/api/gitlab/${appno}/branches`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.message || '获取分支失败');
      }
      const json = await res.json();
      return json.data || [];
    },

    async getCiFile(appno: string, deployno?: string): Promise<string> {
      const query = deployno ? `?deployno=${deployno}` : '';
      const res = await authFetch(`/api/gitlab/${appno}/cifile${query}`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.message || '获取 CI 文件失败');
      }
      const json = await res.json();
      return json.data || '';
    },

    async getPipelineHistory(appno: string, page = 1, limit = 10): Promise<GitlabPipeline[]> {
      const res = await authFetch(`/api/gitlab/${appno}/pipeline/history?pageIndex=${page}&pageSize=${limit}`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.message || '获取流水线历史失败');
      }
      const json = await res.json();
      return json.data || [];
    },

    async triggerCi(appno: string, deployno: string): Promise<any> {
      const res = await authFetch(`/api/gitlab/${appno}/triggerci?deployno=${deployno}`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.message || '触发流水线失败');
      }
      const json = await res.json();
      return json.data;
    },

    async syncCi(appno: string): Promise<void> {
      const res = await authFetch(`/api/gitlab/${appno}/ci/synch`);
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.message || '同步 CI 模板失败');
      }
    },
  };
};
