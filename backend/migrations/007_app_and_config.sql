-- 007_app_and_config.sql
-- 应用管理与配置管理数据表

-- 1. 应用分类表 (app_class)
CREATE TABLE IF NOT EXISTS app_class (
    id SERIAL PRIMARY KEY,
    number VARCHAR(32) NOT NULL,
    name VARCHAR(64) NOT NULL,
    "desc" VARCHAR(128) NOT NULL DEFAULT '',
    is_del SMALLINT NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_class_number ON app_class(number);

-- 2. 应用表 (app)
CREATE TABLE IF NOT EXISTS app (
    id SERIAL PRIMARY KEY,
    number VARCHAR(32) NOT NULL,
    name VARCHAR(64) NOT NULL,
    key_name VARCHAR(64) NOT NULL DEFAULT '',
    "desc" VARCHAR(128) NOT NULL DEFAULT '',
    class_no VARCHAR(32) NOT NULL DEFAULT '',
    class_name VARCHAR(64) NOT NULL DEFAULT '',
    create_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    doc_path VARCHAR(128) NOT NULL DEFAULT '',
    status SMALLINT NOT NULL DEFAULT 2, -- 0:删除 1:下线 2:正常
    gitlab_id VARCHAR(32) NOT NULL DEFAULT '',
    git_url VARCHAR(128) NOT NULL DEFAULT '',
    is_del SMALLINT NOT NULL DEFAULT 0, -- 1:删除 0:正常
    createby_name VARCHAR(32) NOT NULL DEFAULT '',
    createby_id VARCHAR(32) NOT NULL DEFAULT '',
    lastupdate_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_number ON app(number);
CREATE INDEX IF NOT EXISTS idx_app_key_name ON app(key_name);
CREATE INDEX IF NOT EXISTS idx_app_class_no ON app(class_no);

-- 3. 应用部署环境表 (app_deploy)
CREATE TABLE IF NOT EXISTS app_deploy (
    id SERIAL PRIMARY KEY,
    number VARCHAR(32) NOT NULL,
    app_no VARCHAR(32) NOT NULL,
    name VARCHAR(64) NOT NULL,
    branch_name VARCHAR(64) NOT NULL DEFAULT '',
    build_tag VARCHAR(32) NOT NULL DEFAULT '',
    auto_pub SMALLINT NOT NULL DEFAULT 0,
    api_uri VARCHAR(256) NOT NULL DEFAULT '',
    api_key_id INT NOT NULL DEFAULT 0,
    is_del SMALLINT NOT NULL DEFAULT 0,
    create_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    envs VARCHAR(255) NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_deploy_number ON app_deploy(number);
CREATE INDEX IF NOT EXISTS idx_app_deploy_app_no ON app_deploy(app_no);

-- 4. 应用与用户关系表 (app_user)
CREATE TABLE IF NOT EXISTS app_user (
    id SERIAL PRIMARY KEY,
    uid INT NOT NULL,
    uname VARCHAR(64) NOT NULL,
    app_no VARCHAR(32) NOT NULL,
    key VARCHAR(64) NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_user_uid_app ON app_user(uid, app_no);
CREATE INDEX IF NOT EXISTS idx_app_user_app_no ON app_user(app_no);

-- 5. 通用配置表 (app_config)
CREATE TABLE IF NOT EXISTS app_config (
    id SERIAL PRIMARY KEY,
    number VARCHAR(32) NOT NULL,
    name VARCHAR(64) NOT NULL,
    key VARCHAR(32) NOT NULL,
    value TEXT NOT NULL DEFAULT '',
    create_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    remark VARCHAR(255) NOT NULL DEFAULT '',
    is_del SMALLINT NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_app_config_number ON app_config(number);
CREATE INDEX IF NOT EXISTS idx_app_config_key ON app_config(key);

-- 默认种子数据：默认分类
INSERT INTO app_class (number, name, "desc", is_del)
VALUES ('CLS-DEFAULT', '通用服务', '默认通用应用分类', 0)
ON CONFLICT (number) DO NOTHING;
