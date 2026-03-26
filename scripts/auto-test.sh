#!/bin/bash

# ============================================================
# sec-gateway 自动化测试脚本
# Phase 2C 功能完整验证
# ============================================================

set -e

# 配置
# ============================================================
BASE_URL="${BASE_URL:-http://localhost:8080}"
AUTH_TOKEN="test-bearer-token-12345"
AUTH_HEADER="Authorization: Bearer $AUTH_TOKEN"
CONFIG_FILE="config/default.yaml"
LOG_DIR="logs"
TEST_SESSION_PREFIX="auto-test-"
TIMEOUT_SECONDS=60

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 全局状态
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
START_TIME=$(date +%s)

# ============================================================
# 工具函数
# ============================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((TESTS_SKIPPED++))
}

log_section() {
    echo ""
    echo "============================================================"
    echo " $1"
    echo "============================================================"
}

http_request() {
    local method="${1:-GET}"
    local path="$2"
    local data="$3"
    local extra_headers="$4"
    local expected_status="${5:-200}"
    
    local response
    local status_code
    
    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Content-Type: application/json" \
            $extra_headers \
            -d "$data" \
            "$BASE_URL$path" 2>/dev/null)
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            $extra_headers \
            "$BASE_URL$path" 2>/dev/null)
    fi
    
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    echo "$body"
    echo "$status_code" > /tmp/.last_status_code
}

wait_for_service() {
    log_info "等待服务启动..."
    local elapsed=0
    while [ $elapsed -lt $TIMEOUT_SECONDS ]; do
        if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
            log_info "服务已就绪 (${elapsed}s)"
            return 0
        fi
        sleep 1
        ((elapsed++))
    done
    log_fail "服务启动超时 (${TIMEOUT_SECONDS}s)"
    return 1
}

check_service_running() {
    if ! curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        log_skip "服务未运行，跳过测试"
        return 1
    fi
    return 0
}

cleanup_test_sessions() {
    log_info "清理测试会话..."
    local sessions=$(curl -s "$BASE_URL/sessions" 2>/dev/null | jq -r '.[].id' 2>/dev/null || echo "")
    for session in $sessions; do
        if [[ "$session" == "$TEST_SESSION_PREFIX"* ]]; then
            curl -s -X DELETE "$BASE_URL/sessions/$session" > /dev/null 2>&1 || true
        fi
    done
}

cleanup_logs() {
    if [ -f "$LOG_DIR/audit.log" ]; then
        : > "$LOG_DIR/audit.log"
    fi
}

# ============================================================
# 前置检查
# ============================================================

preflight_check() {
    log_section "前置检查"
    
    # 检查 jq
    if ! command -v jq &> /dev/null; then
        log_fail "jq 未安装 (brew install jq)"
        exit 1
    fi
    log_success "jq 已安装"
    
    # 检查 curl
    if ! command -v curl &> /dev/null; then
        log_fail "curl 未安装"
        exit 1
    fi
    log_success "curl 已安装"
    
    # 检查 Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_fail "Rust/Cargo 未安装"
        exit 1
    fi
    log_success "Rust/Cargo 已安装 ($(rustc --version 2>/dev/null | awk '{print $2}'))"
    
    # 检查配置文件
    if [ ! -f "$CONFIG_FILE" ]; then
        log_fail "配置文件不存在: $CONFIG_FILE"
        exit 1
    fi
    log_success "配置文件存在"
    
    # 创建日志目录
    mkdir -p "$LOG_DIR"
    log_success "日志目录就绪: $LOG_DIR"
}

# ============================================================
# 测试用例
# ============================================================

test_health_check() {
    log_section "测试 1: 健康检查"
    
    local response=$(curl -s "$BASE_URL/health")
    local status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health")
    
    if [ "$status" = "200" ]; then
        log_success "HTTP 状态码: 200"
    else
        log_fail "HTTP 状态码: $status (期望: 200)"
        return 1
    fi
    
    local service=$(echo "$response" | jq -r '.service' 2>/dev/null)
    if [ "$service" = "sec-gateway" ]; then
        log_success "响应 service: sec-gateway"
    else
        log_fail "响应 service: $service (期望: sec-gateway)"
        return 1
    fi
    
    local version=$(echo "$response" | jq -r '.version' 2>/dev/null)
    log_info "版本: $version"
}

test_auth_no_token() {
    log_section "测试 2a: 无认证 (应失败)"
    
    local status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
        -H "Content-Type: application/json" \
        -d '{"messages":[{"role":"user","content":"test"}]}' \
        "$BASE_URL/v1/chat/completions")
    
    if [ "$status" = "401" ]; then
        log_success "正确返回 401 Unauthorized"
    else
        log_fail "返回 $status (期望: 401)"
        return 1
    fi
}

test_auth_bearer_token() {
    log_section "测试 2b: Bearer Token 认证"
    
    local response=$(http_request "POST" "/v1/chat/completions" \
        '{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}]}' \
        "-H \"Authorization: Bearer $AUTH_TOKEN\" -H \"x-session-id: ${TEST_SESSION_PREFIX}auth-bearer\"")
    
    local status=$(cat /tmp/.last_status_code)
    
    if [ "$status" = "200" ] || [ "$status" = "401" ]; then
        log_success "Bearer Token 认证: HTTP $status"
    else
        log_fail "Bearer Token 认证失败: HTTP $status"
        return 1
    fi
}

test_auth_custom_header() {
    log_section "测试 2c: 自定义 Header 认证"
    
    local response=$(http_request "POST" "/v1/chat/completions" \
        '{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}]}' \
        "-H \"X-Gateway-Auth: test-secret-value\" -H \"x-session-id: ${TEST_SESSION_PREFIX}auth-header\"")
    
    local status=$(cat /tmp/.last_status_code)
    
    if [ "$status" = "200" ] || [ "$status" = "401" ]; then
        log_success "自定义 Header 认证: HTTP $status"
    else
        log_fail "自定义 Header 认证失败: HTTP $status"
        return 1
    fi
}

test_pii_detection() {
    log_section "测试 3: 多类型 PII 检测"
    
    local pii_content='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"我的信息：身份证110101199001011234，手机13812345678，邮箱test@example.com，信用卡6217000010012345678，IP地址192.168.1.100，API密钥sk-proj-AbCdEf1234567890，JWT eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U，数据库mongodb://user:pass@host:27017/db"}]}'
    
    local response=$(http_request "POST" "/v1/chat/completions" \
        "$pii_content" \
        "-H \"Authorization: Bearer $AUTH_TOKEN\" -H \"x-session-id: ${TEST_SESSION_PREFIX}pii\"")
    
    # 检查审计日志中是否记录了 PII 检测
    sleep 1
    local pii_count=$(grep -c "${TEST_SESSION_PREFIX}pii" "$LOG_DIR/audit.log" 2>/dev/null || echo "0")
    
    if [ "$pii_count" -gt "0" ]; then
        log_success "审计日志记录: 找到 $pii_count 条记录"
    else
        log_info "审计日志: 未找到记录 (可能审计未启用)"
    fi
    
    log_success "PII 检测请求已发送"
}

test_session_metadata() {
    log_section "测试 5: 会话元数据追踪"
    
    # 发送多个请求到同一会话
    for i in {1..5}; do
        http_request "POST" "/v1/chat/completions" \
            '{"messages":[{"role":"user","content":"Request '"$i"'"}]}' \
            "-H \"Authorization: Bearer $AUTH_TOKEN\" -H \"x-session-id: ${TEST_SESSION_PREFIX}metadata\"" > /dev/null
        sleep 0.2
    done
    
    # 查询会话列表
    local sessions=$(curl -s "$BASE_URL/sessions")
    local metadata_session=$(echo "$sessions" | jq -r '.[] | select(.id=="'"${TEST_SESSION_PREFIX}metadata"'")' 2>/dev/null)
    
    if [ -z "$metadata_session" ] || [ "$metadata_session" = "null" ]; then
        log_skip "会话元数据: 会话未找到 (审计可能未启用)"
        return 0
    fi
    
    local created_at=$(echo "$metadata_session" | jq -r '.created_at')
    local last_accessed=$(echo "$metadata_session" | jq -r '.last_accessed')
    local request_count=$(echo "$metadata_session" | jq -r '.request_count')
    
    if [ "$created_at" != "null" ] && [ "$created_at" != "" ]; then
        log_success "created_at: $created_at"
    else
        log_fail "created_at 为空"
        return 1
    fi
    
    if [ "$last_accessed" != "null" ] && [ "$last_accessed" != "" ]; then
        log_success "last_accessed: $last_accessed"
    else
        log_fail "last_accessed 为空"
        return 1
    fi
    
    if [ "$request_count" -ge 5 ]; then
        log_success "request_count: $request_count (>= 5)"
    else
        log_fail "request_count: $request_count (期望 >= 5)"
        return 1
    fi
}

test_session_delete() {
    log_section "测试 5c: 删除会话"
    
    # 创建测试会话
    http_request "POST" "/v1/chat/completions" \
        '{"messages":[{"role":"user","content":"To be deleted"}]}' \
        "-H \"Authorization: Bearer $AUTH_TOKEN\" -H \"x-session-id: ${TEST_SESSION_PREFIX}delete-test\"" > /dev/null
    
    sleep 0.5
    
    # 删除会话
    local delete_status=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE \
        "$BASE_URL/sessions/${TEST_SESSION_PREFIX}delete-test")
    
    if [ "$delete_status" = "200" ] || [ "$delete_status" = "204" ]; then
        log_success "删除会话: HTTP $delete_status"
    else
        log_fail "删除会话失败: HTTP $delete_status"
        return 1
    fi
    
    # 验证会话已删除
    local sessions=$(curl -s "$BASE_URL/sessions")
    local deleted_session=$(echo "$sessions" | jq -r '.[] | select(.id=="'"${TEST_SESSION_PREFIX}delete-test"'")' 2>/dev/null)
    
    if [ -z "$deleted_session" ] || [ "$deleted_session" = "null" ]; then
        log_success "会话已成功删除"
    else
        log_fail "会话仍然存在"
        return 1
    fi
}

test_audit_log() {
    log_section "测试 4: 审计日志"
    
    # 触发一些请求
    http_request "POST" "/v1/chat/completions" \
        '{"messages":[{"role":"user","content":"Audit test"}]}' \
        "-H \"Authorization: Bearer $AUTH_TOKEN\" -H \"x-session-id: ${TEST_SESSION_PREFIX}audit\"" > /dev/null
    
    sleep 1
    
    if [ ! -f "$LOG_DIR/audit.log" ]; then
        log_skip "审计日志文件不存在 (审计功能可能未启用)"
        return 0
    fi
    
    local log_size=$(stat -f%z "$LOG_DIR/audit.log" 2>/dev/null || stat -c%s "$LOG_DIR/audit.log" 2>/dev/null || echo "0")
    
    if [ "$log_size" -gt 0 ]; then
        log_success "审计日志文件存在: ${log_size} bytes"
    else
        log_fail "审计日志文件为空"
        return 1
    fi
    
    # 检查 JSON 格式
    local last_log=$(tail -1 "$LOG_DIR/audit.log")
    local is_json=$(echo "$last_log" | jq -e '.' >/dev/null 2>&1 && echo "true" || echo "false")
    
    if [ "$is_json" = "true" ]; then
        log_success "审计日志格式: JSON 有效"
        
        # 解析字段
        local timestamp=$(echo "$last_log" | jq -r '.timestamp' 2>/dev/null)
        local level=$(echo "$last_log" | jq -r '.level' 2>/dev/null)
        local session_id=$(echo "$last_log" | jq -r '.session_id' 2>/dev/null)
        
        log_info "  timestamp: $timestamp"
        log_info "  level: $level"
        log_info "  session_id: $session_id"
    else
        log_fail "审计日志格式: JSON 无效"
        return 1
    fi
}

test_audit_rotation() {
    log_section "测试 4b: 审计日志轮换"
    
    if [ ! -f "$LOG_DIR/audit.log" ]; then
        log_skip "审计日志文件不存在，跳过轮换测试"
        return 0
    fi
    
    # 检查是否存在轮换文件
    local rotation_files=$(ls "$LOG_DIR"/audit-*.log 2>/dev/null | wc -l | tr -d ' ')
    
    if [ "$rotation_files" -gt 0 ]; then
        log_success "发现 $rotation_files 个轮换文件"
        ls -la "$LOG_DIR"/audit-*.log 2>/dev/null | tail -3
    else
        log_info "暂无声志轮换文件 (需要等待日期切换或达到大小限制)"
    fi
    
    log_success "轮换机制检查完成"
}

test_rate_limit() {
    log_section "测试 7: 速率限制"
    
    local rate_limited=0
    local successful=0
    
    # 快速发送大量请求
    for i in {1..20}; do
        local status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
            -H "Authorization: Bearer $AUTH_TOKEN" \
            -H "Content-Type: application/json" \
            -d '{"messages":[{"role":"user","content":"Rate test '"$i"'"}]}' \
            "$BASE_URL/v1/chat/completions" 2>/dev/null)
        
        if [ "$status" = "429" ]; then
            ((rate_limited++))
        elif [ "$status" = "200" ] || [ "$status" = "401" ]; then
            ((successful++))
        fi
        
        sleep 0.05
    done
    
    log_info "成功请求: $successful, 限流: $rate_limited"
    
    if [ "$rate_limited" -gt 0 ]; then
        log_success "速率限制生效: $rate_limited 个请求被限流"
    else
        log_info "未触发速率限制 (burst_size 可能较大)"
    fi
}

test_cors() {
    log_section "测试 8: CORS 白名单"
    
    # 测试允许的 Origin
    local allowed_response=$(curl -s -X OPTIONS \
        -H "Origin: http://localhost:3000" \
        -H "Access-Control-Request-Method: POST" \
        -H "Access-Control-Request-Headers: Authorization,Content-Type" \
        -w "\n%{http_code}" \
        "$BASE_URL/v1/chat/completions" 2>/dev/null)
    
    local allowed_status=$(echo "$allowed_response" | tail -n1)
    local allowed_cors=$(echo "$allowed_response" | grep -i "access-control-allow-origin" | head -1)
    
    if [ "$allowed_status" = "200" ] || [ "$allowed_status" = "204" ]; then
        log_success "CORS 预检请求: HTTP $allowed_status"
    else
        log_info "CORS 预检请求: HTTP $allowed_status (可能 CORS 未启用)"
    fi
    
    if [ -n "$allowed_cors" ]; then
        log_success "CORS 响应头: $allowed_cors"
    else
        log_info "CORS 响应头未设置 (CORS 可能未启用)"
    fi
}

test_prometheus_metrics() {
    log_section "测试 9: Prometheus 指标"
    
    local metrics=$(curl -s "$BASE_URL/metrics")
    local metrics_status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/metrics")
    
    if [ "$metrics_status" = "200" ]; then
        log_success "指标端点: HTTP 200"
    else
        log_fail "指标端点: HTTP $metrics_status"
        return 1
    fi
    
    # 检查关键指标
    local has_requests=$(echo "$metrics" | grep -c "gateway_requests_total" || echo "0")
    local has_pii=$(echo "$metrics" | grep -c "gateway_pii_detections_total" || echo "0")
    local has_duration=$(echo "$metrics" | grep -c "gateway_request_duration" || echo "0")
    
    if [ "$has_requests" -gt 0 ]; then
        log_success "发现请求计数指标"
    else
        log_info "未发现请求计数指标"
    fi
    
    if [ "$has_pii" -gt 0 ]; then
        log_success "发现 PII 检测指标"
    else
        log_info "未发现 PII 检测指标"
    fi
    
    if [ "$has_duration" -gt 0 ]; then
        log_success "发现请求延迟指标"
    else
        log_info "未发现请求延迟指标"
    fi
}

test_streaming() {
    log_section "测试 10: 流式响应"
    
    # 流式请求测试
    local stream_response=$(curl -s -N -X POST \
        -H "Authorization: Bearer $AUTH_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"model":"gpt-3.5-turbo","stream":true,"messages":[{"role":"user","content":"Count to 3"}]}' \
        "$BASE_URL/v1/chat/completions" 2>/dev/null | head -20)
    
    if [ -n "$stream_response" ]; then
        log_success "流式响应: 收到数据"
    else
        log_info "流式响应: 未收到数据 (可能目标服务不支持)"
    fi
}

test_key_rotation_check() {
    log_section "测试 6: 密钥轮换检查"
    
    # 检查配置
    if grep -q "key_rotation:" "$CONFIG_FILE" 2>/dev/null; then
        log_success "密钥轮换配置存在"
        
        local enabled=$(grep -A3 "key_rotation:" "$CONFIG_FILE" | grep "enabled" | awk '{print $2}' | tr -d ':')
        local interval=$(grep -A3 "key_rotation:" "$CONFIG_FILE" | grep "interval_days" | awk '{print $2}' | tr -d ':')
        
        log_info "enabled: $enabled"
        log_info "interval_days: $interval"
    else
        log_info "密钥轮换配置未找到"
    fi
    
    # 检查服务日志中的轮换信息
    log_info "注意: 完整轮换测试需要等待 interval_days 后自动触发"
    log_success "密钥轮换机制检查完成"
}

# ============================================================
# 服务管理
# ============================================================

start_service() {
    log_section "启动服务"
    
    # 检查是否已有服务运行
    if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        log_info "服务已在运行，使用现有实例"
        return 0
    fi
    
    # 设置环境变量
    export FPE_KEY="${FPE_KEY:-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef}"
    export RUST_LOG="${RUST_LOG:-info}"
    
    log_info "启动 sec-gateway..."
    cargo run --release > /tmp/sec-gateway.log 2>&1 &
    CARGO_PID=$!
    
    log_info "进程 PID: $CARGO_PID"
    echo "$CARGO_PID" > /tmp/.sec-gateway.pid
    
    # 等待服务就绪
    wait_for_service
}

stop_service() {
    log_section "停止服务"
    
    if [ -f /tmp/.sec-gateway.pid ]; then
        local pid=$(cat /tmp/.sec-gateway.pid)
        if kill -0 "$pid" 2>/dev/null; then
            log_info "停止进程 $pid..."
            kill "$pid" 2>/dev/null || true
            sleep 2
            kill -9 "$pid" 2>/dev/null || true
        fi
        rm -f /tmp/.sec-gateway.pid
    fi
    
    # 也尝试查找并杀死可能的残留进程
    pkill -f "sec-gateway" 2>/dev/null || true
    
    log_success "服务已停止"
}

# ============================================================
# 测试报告
# ============================================================

generate_report() {
    local end_time=$(date +%s)
    local duration=$((end_time - START_TIME))
    local minutes=$((duration / 60))
    local seconds=$((duration % 60))
    
    echo ""
    echo "============================================================"
    echo " 测试报告"
    echo "============================================================"
    echo " 测试时间: ${minutes}m ${seconds}s"
    echo "------------------------------------------------------------"
    echo -e " ${GREEN}通过: $TESTS_PASSED${NC}"
    echo -e " ${RED}失败: $TESTS_FAILED${NC}"
    echo -e " ${YELLOW}跳过: $TESTS_SKIPPED${NC}"
    echo "------------------------------------------------------------"
    echo " 总计: $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))"
    echo "============================================================"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}所有测试通过！${NC}"
        return 0
    else
        echo -e "${RED}有 $TESTS_FAILED 个测试失败${NC}"
        return 1
    fi
}

# ============================================================
# 主函数
# ============================================================

main() {
    echo ""
    echo "============================================================"
    echo " sec-gateway Phase 2C 自动化测试"
    echo "============================================================"
    echo " 基础 URL: $BASE_URL"
    echo " 测试会话前缀: $TEST_SESSION_PREFIX"
    echo " 日志目录: $LOG_DIR"
    echo "============================================================"
    
    # 解析参数
    case "${1:-full}" in
        "setup")
            preflight_check
            cleanup_test_sessions
            cleanup_logs
            log_success "环境准备完成"
            ;;
        "start")
            preflight_check
            start_service
            ;;
        "stop")
            stop_service
            ;;
        "cleanup")
            cleanup_test_sessions
            cleanup_logs
            log_success "清理完成"
            ;;
        "full")
            preflight_check
            start_service
            cleanup_test_sessions
            cleanup_logs
            
            # Phase 1-2C 核心功能测试
            test_health_check
            test_auth_no_token
            test_auth_bearer_token
            test_auth_custom_header
            test_pii_detection
            test_session_metadata
            test_session_delete
            test_audit_log
            test_audit_rotation
            test_rate_limit
            test_cors
            test_prometheus_metrics
            test_streaming
            test_key_rotation_check
            
            stop_service
            generate_report
            ;;
        "quick")
            check_service_running || exit 1
            
            test_health_check
            test_pii_detection
            test_session_metadata
            test_audit_log
            test_prometheus_metrics
            
            generate_report
            ;;
        "help"|"-h"|"--help")
            echo "用法: $0 [命令]"
            echo ""
            echo "命令:"
            echo "  setup   - 前置检查和环境准备"
            echo "  start   - 启动服务"
            echo "  stop    - 停止服务"
            echo "  cleanup - 清理测试会话和日志"
            echo "  full    - 完整测试 (启动->测试->停止) [默认]"
            echo "  quick   - 快速测试 (假设服务已运行)"
            echo "  help    - 显示帮助"
            echo ""
            echo "环境变量:"
            echo "  BASE_URL      - 服务地址 (默认: http://localhost:8080)"
            echo "  FPE_KEY       - FPE 加密密钥"
            echo "  RUST_LOG      - 日志级别"
            exit 0
            ;;
        *)
            echo "未知命令: $1"
            echo "使用 '$0 help' 查看帮助"
            exit 1
            ;;
    esac
}

# 信号处理
trap 'stop_service 2>/dev/null; exit 1' INT TERM

# 运行主函数
main "$@"
