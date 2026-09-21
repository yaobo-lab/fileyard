-- 009_smart_home_classes.sql
-- 下线原有旧测试分类（如通用服务、电商微服务等），并初始化全屋智能家居专业分类体系

UPDATE app_class 
SET is_del = 1 
WHERE name IN ('通用服务', '电商微服务') OR name LIKE '%微服务%' OR name LIKE '%电商%';

-- 插入标准智能家居分类
INSERT INTO app_class (number, name, "desc", is_del)
VALUES 
  ('CLS-LIGHTING', '照明类别', '智能调光驱动、智能开关、RGBW调色、DALI/DMX驱动等照明控制系统', 0),
  ('CLS-CURTAIN', '窗帘类别', '智能开合帘电机、电动卷帘、百叶帘控制器、智能推窗器等遮阳驱动系统', 0),
  ('CLS-PANEL', '中控类别', '智能中控大屏、智慧语音面板、全屋场景开关、多功能触摸控制屏', 0),
  ('CLS-GATEWAY', '网关类别', '多协议智能网关、KNX/Zigbee/Matter/RS485总线网关、边缘主机', 0),
  ('CLS-HVAC', '暖通类别', '中央空调VRV网关、智能地暖温控器、新风系统控制器、环境温湿度控制', 0),
  ('CLS-SECURITY', '安防类别', '人体移动/微动存在探测器、门窗磁传感器、烟雾报警器、燃气报警器、水浸报警器', 0),
  ('CLS-DOORLOCK', '门锁类别', '3D人脸识别视频锁、指纹密码锁、智能可视门铃、智能猫眼、门禁控制系统', 0),
  ('CLS-SENSOR', '传感类别', '高精度温湿度传感器、环境照度传感器、空气质量PM2.5/CO2传感器、跌倒雷达', 0),
  ('CLS-MEDIA', '影音类别', '背景音乐主机、分布式功放系统、家庭影院控制器、红外万能遥控转发模块', 0),
  ('CLS-POWER', '电工类别', '智能墙面插座、导轨式微型断路器、智能计量电表、配电箱控制模块', 0)
ON CONFLICT (number) DO UPDATE SET 
  name = EXCLUDED.name,
  "desc" = EXCLUDED."desc",
  is_del = 0;

-- 更新已有固件中旧的电商微服务为智能中控类别
UPDATE app 
SET class_no = 'CLS-PANEL', class_name = '中控类别' 
WHERE class_name IN ('通用服务', '电商微服务') OR class_name LIKE '%微服务%' OR class_name LIKE '%电商%';
