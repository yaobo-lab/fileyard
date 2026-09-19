CREATE TABLE `app` (
  `id` int NOT NULL AUTO_INCREMENT COMMENT '主键ID',
  `number` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '应用编号',
  `name` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '名称', 
  `desc` varchar(128) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '描述',
  `class_no` varchar(32) NOT NULL COMMENT '所属分类编号',
  `class_name` varchar(64) NOT NULL COMMENT '所属分类名称',
  `create_time` datetime NOT NULL COMMENT '创建时间', 
  `doc_path` varchar(128) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '文档地址', 
  `status` tinyint NOT NULL COMMENT '0：删除 1:下线，2：正常',
  `gitlab_id` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '仓库ID',
  `git_url` varchar(128) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '地址', 
  `is_del` tinyint NOT NULL COMMENT '1:删除，0正常',
  `createby_name` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '创建人姓名',
  `createby_id` varchar(32) NOT NULL COMMENT '创建人ID',
  `lastupdate_time` datetime NOT NULL COMMENT '最后更新时间',
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_no` (`number`) USING BTREE,
  UNIQUE KEY `idx_key` (`key_name`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=163 DEFAULT CHARSET=utf8mb3 COMMENT='应用表（系统&服务）';


CREATE TABLE `app_class` (
  `id` int NOT NULL AUTO_INCREMENT COMMENT '主键ID',
  `number` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '应用编号',
  `name` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '服务分组名称',
  `desc` varchar(128) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '描述',
  `is_del` tinyint NOT NULL COMMENT '1:删除，0正常',
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_no` (`number`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=23 DEFAULT CHARSET=utf8mb3 COMMENT='应用分类';


CREATE TABLE `app_deploy` (
  `id` int NOT NULL AUTO_INCREMENT,
  `number` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '应用编号',
  `app_no` varchar(32) NOT NULL COMMENT '所属app编号',
  `name` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '环境名称',
  `branch_name` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '关联分支名称',
  `build_tag` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '打包编译执行命令得后缀',
  `auto_pub` tinyint NOT NULL COMMENT '是否自动发布', 
  `api_uri` varchar(256) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT 'K8S api 密钥',
  `api_key_id` int NOT NULL COMMENT 'K8S api 密钥ID',
  `is_del` tinyint NOT NULL COMMENT '1:删除，0正常',
  `create_time` datetime NOT NULL COMMENT '创建时间',
  `envs` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '环境变量',
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_no` (`number`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=49 DEFAULT CHARSET=utf8mb3 COMMENT='应用部署环境';



CREATE TABLE `app_user` (
  `id` int NOT NULL AUTO_INCREMENT,
  `uid` int NOT NULL COMMENT '用户编号',
  `uname` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_bin NOT NULL COMMENT '用户名',
  `app_no` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_bin NOT NULL COMMENT '应用编号',
  `key` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_bin NOT NULL COMMENT '权限值',
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_uid_app` (`uid`,`app_no`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=23 DEFAULT CHARSET=utf8mb3 COLLATE=utf8_bin COMMENT='应用与用户关系表';


CREATE TABLE `app_config` (
  `id` int NOT NULL AUTO_INCREMENT COMMENT '主键ID',
  `number` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '唯一标识',
  `name` varchar(64) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '名称',
  `key` varchar(32) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT 'config key',
  `value` text CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '配置value',
  `create_time` datetime NOT NULL COMMENT '创建时间',
  `remark` varchar(255) CHARACTER SET utf8mb3 COLLATE utf8_general_ci NOT NULL COMMENT '备注',
  `is_del` tinyint NOT NULL COMMENT '1:删除，0正常',
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_no` (`number`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=23 DEFAULT CHARSET=utf8mb3 COMMENT='通用配置表：如gitlab ci 模板';