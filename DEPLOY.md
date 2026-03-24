# Git Worktree 部署架构

## 目录结构

```
/Users/yangkai/projects/
├── sec-gateway/              # 开发目录 (dev 分支)
│   ├── .git/                 # Git 仓库
│   ├── deploy.sh             # 部署脚本
│   └── ...                   # 项目文件
│
└── sec-gateway-deploy/       # 部署目录
    ├── main/                 # Production (main 分支)
    └── staging/              # Staging (staging 分支)
```

## 工作流程

### 1. 日常开发
```bash
cd /Users/yangkai/projects/sec-gateway
# 在 dev 分支上开发
git add .
git commit -m "Feature: ..."
```

### 2. 部署到 Staging
```bash
./deploy.sh staging
# 自动: dev → staging
```

### 3. 部署到 Production
```bash
./deploy.sh production
# 自动: staging → main
```

### 4. 回滚操作
```bash
./deploy.sh rollback-staging      # 回滚 Staging
./deploy.sh rollback-production   # 回滚 Production
```

### 5. 查看状态
```bash
./deploy.sh status
```

## 分支策略

- **dev**: 开发分支，日常开发在此进行
- **staging**: 预发布分支，测试环境部署
- **main**: 生产分支，生产环境部署

## 优势

✅ **隔离环境**: 每个环境独立目录，互不影响
✅ **快速切换**: Worktree 机制，无需 checkout 切换
✅ **安全部署**: 单向流动 (dev → staging → main)
✅ **便捷回滚**: 一键回滚到上一版本

## 注意事项

- 只在 `dev` 分支进行开发
- `staging` 和 `main` 分支只接受 merge，不直接修改
- 回滚操作会丢失最新一次提交，请谨慎使用
