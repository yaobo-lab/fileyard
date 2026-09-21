-- 010_mqtt_clients.sql
-- MQTT 客户端连接登记表，按 MQTT 账号（username）只保留一条唯一记录

CREATE TABLE IF NOT EXISTS mqtt_clients (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) NOT NULL UNIQUE,
    client_id VARCHAR(255) NOT NULL,
    device_name VARCHAR(255),
    ip_address VARCHAR(100),
    port INTEGER,
    proto_ver SMALLINT DEFAULT 5,
    keepalive INTEGER DEFAULT 60,
    clean_start BOOLEAN DEFAULT true,
    online BOOLEAN NOT NULL DEFAULT true,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    disconnected_at TIMESTAMPTZ,
    disconnected_reason VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_mqtt_clients_username ON mqtt_clients(username);
CREATE INDEX IF NOT EXISTS idx_mqtt_clients_client_id ON mqtt_clients(client_id);
CREATE INDEX IF NOT EXISTS idx_mqtt_clients_online ON mqtt_clients(online);
CREATE INDEX IF NOT EXISTS idx_mqtt_clients_connected_at ON mqtt_clients(connected_at);
