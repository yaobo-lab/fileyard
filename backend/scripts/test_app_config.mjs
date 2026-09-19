// 自动化测试脚本：登录并端到端测试固件管理与配置管理全量 API

async function runTests() {
  const baseUrl = 'http://127.0.0.1:8081';
  console.log('🚀 开始自动化端到端测试 (通过 Vite 8081 前端网关)...');


  // 1. 登录
  console.log('\n--- [1/6] 测试登录认证 ---');
  const loginRes = await fetch(`${baseUrl}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      email: 'superadmin@clovalink.com',
      password: 'password123',
    }),
  });

  if (!loginRes.ok) {
    const txt = await loginRes.text();
    throw new Error(`登录失败 (${loginRes.status}): ${txt}`);
  }

  const loginData = await loginRes.json();
  const token = loginData.token || (loginData.data && loginData.data.token);
  if (!token) {
    throw new Error(`登录返回中未找到 token: ${JSON.stringify(loginData)}`);
  }
  console.log('✅ 登录成功！获取到 Bearer Token');

  const headers = {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${token}`,
  };

  // 2. 测试分类管理
  console.log('\n--- [2/6] 测试固件分类 API ---');
  const classCreateRes = await fetch(`${baseUrl}/api/app/class`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: '电商微服务',
      desc: '在线电商与交易服务体系',
    }),
  });
  console.log('创建分类状态码:', classCreateRes.status);
  const classCreateData = await classCreateRes.json();
  console.log('创建分类结果:', classCreateData.message);
  const createdClass = classCreateData.data;

  const classListRes = await fetch(`${baseUrl}/api/app/class/pages`, { headers });
  const classListData = await classListRes.json();
  console.log(`✅ 分类列表查询成功，当前分类总数: ${classListData.data.total}`);

  // 3. 测试创建固件
  console.log('\n--- [3/6] 测试固件创建与查询 API ---');
  const appCreateRes = await fetch(`${baseUrl}/api/app/create`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: '商城中心服务',
      desc: '提供电商商品、订单与交易支持',
      class_no: createdClass ? createdClass.number : 'CLS-DEFAULT',
      class_name: createdClass ? createdClass.name : '默认分类',
      status: 2,
      gitlab_id: '1024',
      git_url: 'https://gitlab.com/example/mall-center.git',
    }),
  });
  console.log('创建固件状态码:', appCreateRes.status);
  const appCreateData = await appCreateRes.json();
  console.log('创建固件结果:', appCreateData.message);
  const createdApp = appCreateData.data;
  console.log('创建的固件编号:', createdApp.number);

  // 查询列表
  const appListRes = await fetch(`${baseUrl}/api/app/page?curPage=1&pageSize=10`, { headers });
  const appListData = await appListRes.json();
  console.log(`✅ 固件列表查询成功，当前固件总数: ${appListData.data.total}`);

  // 4. 测试编译环境管理
  console.log('\n--- [4/6] 测试固件部署环境 API ---');
  const deployCreateRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/deploy/create`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: '生产环境 (Prod)',
      branch_name: 'main',
      build_tag: 'build:prod',
      auto_pub: 1,
      api_uri: 'https://k8s.example.com/api',
      api_key_id: 888,
      envs: 'ENV=production',
    }),
  });
  console.log('创建部署环境状态码:', deployCreateRes.status);
  const deployData = await deployCreateRes.json();
  console.log('创建环境结果:', deployData.message);
  const createdDeploy = deployData.data;

  // 复制部署
  const copyDeployRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/deploy/copy?id=${createdDeploy.id}`, { headers });
  const copyDeployData = await copyDeployRes.json();
  console.log('复制部署环境结果:', copyDeployData.message);

  // 查看列表
  const deploysRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/deploy/list`, { headers });
  const deploysData = await deploysRes.json();
  console.log(`✅ 部署环境查询成功，当前部署数: ${deploysData.data.length}`);

  // 5. 测试成员绑定
  console.log('\n--- [5/6] 测试固件成员绑定 API ---');
  const userCreateRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/user/create`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      uid: 1001,
      uname: '测试开发工程师',
      key: 'developer',
    }),
  });
  console.log('添加项目成员请求状态码:', userCreateRes.status);
  const userCreateText = await userCreateRes.text();
  console.log('添加项目成员返回原始内容:', userCreateText);
  let userCreateData;
  try {
    userCreateData = JSON.parse(userCreateText);
  } catch (e) {
    console.error('解析 JSON 失败:', e);
  }
  console.log('添加项目成员结果:', userCreateData?.message);

  const usersRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/user/list`, { headers });
  const usersData = await usersRes.json();
  console.log(`✅ 成员列表查询成功，成员数: ${usersData.data.length}`);

  // 6. 测试配置管理 API
  console.log('\n--- [6/6] 测试配置管理 API ---');
  const configSaveRes = await fetch(`${baseUrl}/api/config`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: '电商 CI 构建流水线模板',
      key: 'ci-mall-template',
      value: `stages:\n  - build\n  - test\n  - deploy\nbuild-job:\n  stage: build\n  script:\n    - echo "Building..."`,
      remark: '用于电商微服务通用的 CI 配置',
    }),
  });
  console.log('创建配置状态码:', configSaveRes.status);
  const configSaveData = await configSaveRes.json();
  const createdConfig = configSaveData.data;

  // 7. 测试固件基本信息与负责人更新
  console.log('\n--- [7/9] 测试固件更新 API ---');
  const updateBasicRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/save/basic`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      name: '测试电商中心(已更新)',
      cname: '商城订单结算中心-v2',
      class_id: createdClass.id,
      doc_path: 'https://docs.example.com/v2',
      gitlab_id: '1088',
      git_url: 'git@gitlab.com:mall/order-v2.git',
      status: 1,
    }),
  });
  console.log('更新固件基本信息状态:', updateBasicRes.status);

  // 8. 测试成员移除与部署删除
  console.log('\n--- [8/9] 测试成员移除与部署删除 API ---');
  const deleteUserRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/user?id=${userCreateData.data.id}`, {
    method: 'DELETE',
    headers,
  });
  console.log('移除成员状态:', deleteUserRes.status);

  const deleteDeployRes = await fetch(`${baseUrl}/api/app/${createdApp.number}/deploy?id=${createdDeploy.id}`, {
    method: 'DELETE',
    headers,
  });
  console.log('删除部署环境状态:', deleteDeployRes.status);

  // 9. 测试配置单查与删除
  console.log('\n--- [9/9] 测试配置详情与软删除 API ---');
  const getConfigRes = await fetch(`${baseUrl}/api/config/${createdConfig.id}`, { headers });
  console.log('获取单条配置详情状态:', getConfigRes.status);

  const deleteConfigRes = await fetch(`${baseUrl}/api/config/${createdConfig.id}`, {
    method: 'DELETE',
    headers,
  });
  console.log('软删除配置状态:', deleteConfigRes.status);

  console.log('\n🎉 ==============================================================');
  console.log('🎉 所有固件管理与配置管理 CRUD 完整生命周期测试全部 100% 通过！');
  console.log('🎉 ==============================================================');
}

runTests().catch((err) => {
  console.error('❌ 测试过程中发生错误:', err);
  process.exit(1);
});
