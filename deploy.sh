#!/bin/bash
# Git Worktree 部署脚本

set -e

REPO_DIR="/Users/yangkai/projects/sec-gateway"
DEPLOY_DIR="/Users/yangkai/projects/sec-gateway-deploy"

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Privacy Gateway 部署工具 ===${NC}"

# 显示当前 worktree 状态
show_status() {
    echo -e "\n${GREEN}当前 Worktree 状态:${NC}"
    git worktree list
}

# 部署到 staging
deploy_staging() {
    echo -e "\n${GREEN}部署到 Staging 环境...${NC}"
    cd "$REPO_DIR"
    git checkout dev
    git add .
    git commit -m "Update: $(date '+%Y-%m-%d %H:%M:%S')" || echo "No changes to commit"
    git checkout staging
    git merge dev --no-edit
    echo -e "${GREEN}✓ Staging 部署完成${NC}"
}

# 部署到 production
deploy_production() {
    echo -e "\n${GREEN}部署到 Production 环境...${NC}"
    cd "$REPO_DIR"
    git checkout main
    git merge staging --no-edit
    echo -e "${GREEN}✓ Production 部署完成${NC}"
    git checkout dev
}

# 回滚 staging
rollback_staging() {
    echo -e "\n${RED}回滚 Staging 到上一个版本...${NC}"
    cd "$REPO_DIR"
    git checkout staging
    git reset --hard HEAD~1
    git checkout dev
    echo -e "${GREEN}✓ Staging 回滚完成${NC}"
}

# 回滚 production
rollback_production() {
    echo -e "\n${RED}回滚 Production 到上一个版本...${NC}"
    cd "$REPO_DIR"
    git checkout main
    git reset --hard HEAD~1
    git checkout dev
    echo -e "${GREEN}✓ Production 回滚完成${NC}"
}

# 命令处理
case "$1" in
    status)
        show_status
        ;;
    staging)
        deploy_staging
        show_status
        ;;
    production|prod)
        deploy_production
        show_status
        ;;
    rollback-staging)
        rollback_staging
        show_status
        ;;
    rollback-production|rollback-prod)
        rollback_production
        show_status
        ;;
    *)
        echo "用法: $0 {status|staging|production|rollback-staging|rollback-production}"
        echo ""
        echo "命令:"
        echo "  status              - 显示 worktree 状态"
        echo "  staging             - 部署到 Staging (dev → staging)"
        echo "  production          - 部署到 Production (staging → main)"
        echo "  rollback-staging    - 回滚 Staging 到上一版本"
        echo "  rollback-production - 回滚 Production 到上一版本"
        exit 1
        ;;
esac
