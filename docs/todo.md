
我要在当前项目实现： 应用管理与配置管理 两个模块功能。


下面你要参考下面两个项目，将里面的一些功能，搬到当前项目中（搬的时候，你要保持当前项目的结构，前端风格、保持不要变）

后端go写的rest api 项目：  docs/api
前端vue 写的web 页面 ： docs/web


关于数据的表：

我简化后的MYSQL 脚本：backend\docs\db.sql，你参考这个表结构，在当前PGSQL添加一样的表


关于后端的要求：

你要将：
应用管理api ： docs\api\internal\api\app_api  
配置管理api ： docs\api\internal\api\config_api 

全部抄一份到当前后端rust 后端中，因为 go项目使用的是mysql 作为后端
现在你需要将MYSQL 的表转为PGSQL 。

下面的我简化后的MYSQL 脚本：backend\docs\db.sql
注意：我简化了结构，将一些无用的字段删除，或者新增加的字段添加了（golang代码orm还是旧的表结构，你按我现在新的表结构来）

 
关于前端的要求：

1：保持原项目的风格（shadcn/ui + Radix ），不要使用vue的风格，你参考vue里的页面功能来实现
2: 前端也需要将 docs\web\src\page\pipeline gitlab 的cicd 搬到现有项目中
3：因为有些字段不需要了，你参考 我简化后的MYSQL 脚本：backend\docs\db.sql 来做字段的处理
4：配置管理 docs\web\src\page\config   搬到现有项目中
5：应用管理docs\web\src\page\app  、docs\web\src\page\deploy   搬到现有项目中
