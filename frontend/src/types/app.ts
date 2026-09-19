export interface AppItem {
  id: number;
  number: string;
  name: string;
  desc: string;
  class_no: string;
  class_name: string;
  create_time: string;
  doc_path: string;
  status: number; // 0:删除, 1:下线, 2:正常
  gitlab_id: string;
  git_url: string;
  is_del: number;
  createby_name: string;
  createby_id: string;
  lastupdate_time: string;
}

export interface AppClass {
  id: number;
  number: string;
  name: string;
  desc: string;
  is_del: number;
}

export interface AppDeploy {
  id: number;
  number: string;
  app_no: string;
  name: string;
  branch_name: string;
  build_tag: string;
  auto_pub: number; // 1:是, 0:否
  api_uri: string;
  api_key_id: number;
  envs: string;
  create_time: string;
  is_del: number;
}

export interface AppUser {
  id: number;
  uid: string | number;
  uname: string;

  app_no: string;
  key: string;
}

export interface AppConfig {
  id: number;
  number: string;
  name: string;
  key: string;
  value: string;
  create_time: string;
  remark: string;
  is_del: number;
}

export interface GitlabBranch {
  name: string;
  merged?: boolean;
  protected?: boolean;
  default?: boolean;
}

export interface GitlabPipeline {
  id: number;
  iid?: number;
  project_id?: number;
  status: string; // 'success' | 'failed' | 'running' | 'pending' | 'canceled' | 'skipped' | 'manual'
  ref: string;
  sha?: string;
  web_url: string;
  created_at: string;
  updated_at?: string;
}
