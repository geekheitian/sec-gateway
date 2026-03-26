.PHONY: test test-full test-quick test-setup test-clean test-start test-stop help

help:
	@echo "sec-gateway 测试命令"
	@echo ""
	@echo "用法: make [目标]"
	@echo ""
	@echo "测试目标:"
	@echo "  test-full    - 完整测试 (前置检查 -> 启动 -> 测试 -> 停止) [默认]"
	@echo "  test-quick   - 快速测试 (假设服务已运行)"
	@echo "  test-setup   - 前置检查和环境准备"
	@echo "  test-clean   - 清理测试会话和日志"
	@echo ""
	@echo "服务目标:"
	@echo "  test-start   - 启动服务"
	@echo "  test-stop    - 停止服务"
	@echo ""
	@echo "直接使用脚本:"
	@echo "  ./scripts/auto-test.sh full   # 完整测试"
	@echo "  ./scripts/auto-test.sh quick  # 快速测试"
	@echo "  ./scripts/auto-test.sh help  # 查看帮助"

test: test-full

test-full:
	@echo "Running full test suite..."
	@./scripts/auto-test.sh full

test-quick:
	@echo "Running quick tests (service must be running)..."
	@./scripts/auto-test.sh quick

test-setup:
	@echo "Setting up test environment..."
	@./scripts/auto-test.sh setup

test-clean:
	@echo "Cleaning up test artifacts..."
	@./scripts/auto-test.sh cleanup

test-start:
	@echo "Starting sec-gateway service..."
	@./scripts/auto-test.sh start

test-stop:
	@echo "Stopping sec-gateway service..."
	@./scripts/auto-test.sh stop
