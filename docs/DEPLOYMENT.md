# SM4-FF1 Deployment Guide

部署 sec-gateway 的 SM4-FF1 国密加密后端

---

## 系统要求

- **操作系统**: Linux (Ubuntu 20.04+) / macOS (11.0+) / Windows (WSL2)
- **编译器**: GCC 7+ / Clang 10+ (支持 C11)
- **Rust**: 1.94+
- **依赖**: OpenSSL 1.1.1+ (仅编译 wbcrypto-mode 时需要)

---

## wbcrypto-mode 静态库编译

SM4-FF1 后端依赖 [wbcrypto-mode](https://github.com/nanshenjiang/wbcrypto-mode) 静态库。

### Linux (Ubuntu/Debian)

```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake libssl-dev

git clone https://github.com/nanshenjiang/wbcrypto-mode.git /tmp/wbcrypto-mode
cd /tmp/wbcrypto-mode

sed -i '/find_package(OpenMP REQUIRED)/d' CMakeLists.txt

mkdir -p build/out
cd build
cmake ..
make
```

**验证产物**:
```bash
ls -lh /tmp/wbcrypto-mode/build/out/libwbcrypto.a
```

预期输出: `543K libwbcrypto.a`

### macOS

```bash
brew install cmake openssl@3

git clone https://github.com/nanshenjiang/wbcrypto-mode.git /tmp/wbcrypto-mode
cd /tmp/wbcrypto-mode

sed -i '' '/find_package(OpenMP REQUIRED)/d' CMakeLists.txt

mkdir -p build/out
cd build
cmake -DOPENSSL_ROOT_DIR=$(brew --prefix openssl@3) ..
make
```

**验证产物**:
```bash
ls -lh /tmp/wbcrypto-mode/build/out/libwbcrypto.a
```

### Windows (WSL2)

使用 Ubuntu WSL2 发行版，按 Linux 步骤操作。

---

## 环境变量配置

### AES-256-FF1 后端（默认）

```bash
export FPE_KEY="4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60"
```

- **格式**: 64 个十六进制字符（32 字节）
- **生成方式**: `openssl rand -hex 32`

### SM4-128-FF1 后端

```bash
export FPE_BACKEND=sm4
export SM4_FPE_KEY="0123456789abcdef0123456789abcdef"
```

- **格式**: 32 个十六进制字符（16 字节）
- **生成方式**: `openssl rand -hex 16`

### 运行时切换

```bash
export FPE_BACKEND=aes
cargo run

export FPE_BACKEND=sm4
cargo run
```

---

## 配置文件

编辑 `config/default.yaml`：

```yaml
crypto:
  fpe:
    backend: "aes"  # 或 "sm4"
    radix: 10
```

环境变量 `FPE_BACKEND` 会覆盖 YAML 配置。

---

## 编译 sec-gateway

### 前提条件

确保 wbcrypto-mode 静态库已编译至：
- **Linux/macOS**: `/tmp/wbcrypto-mode/build/out/libwbcrypto.a`
- **Include 头文件**: `/tmp/wbcrypto-mode/include/`

### 编译命令

```bash
cd sec-gateway
cargo build --release
```

**链接配置** (已包含在 `.cargo/config.toml`):
```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-L", "/tmp/wbcrypto-mode/build/out", "-l", "static=wbcrypto"]

[target.x86_64-apple-darwin]
rustflags = ["-L", "/tmp/wbcrypto-mode/build/out", "-l", "static=wbcrypto"]

[target.aarch64-apple-darwin]
rustflags = ["-L", "/tmp/wbcrypto-mode/build/out", "-l", "static=wbcrypto"]
```

### 验证编译

```bash
./target/release/sec-gateway --version
cargo test
```

预期输出:
```
sec-gateway 0.1.0
test result: ok. 287 passed; 0 failed
```

---

## Docker 部署

### Dockerfile 示例

```dockerfile
FROM rust:1.94-alpine AS builder

RUN apk add --no-cache build-base cmake openssl-dev git

RUN git clone https://github.com/nanshenjiang/wbcrypto-mode.git /tmp/wbcrypto-mode && \
    cd /tmp/wbcrypto-mode && \
    sed -i '/find_package(OpenMP REQUIRED)/d' CMakeLists.txt && \
    mkdir -p build/out && \
    cd build && \
    cmake .. && \
    make

WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.18
RUN apk add --no-cache libgcc
COPY --from=builder /app/target/release/sec-gateway /usr/local/bin/
COPY --from=builder /app/config /app/config

EXPOSE 8080
CMD ["sec-gateway"]
```

### 构建与运行

```bash
docker build -t sec-gateway:sm4 .
docker run -p 8080:8080 \
  -e FPE_BACKEND=sm4 \
  -e SM4_FPE_KEY="0123456789abcdef0123456789abcdef" \
  sec-gateway:sm4
```

---

## 性能基准

在 macOS M1 Pro 上的基准测试结果：

| 后端 | 加密性能 | 解密性能 | 11位手机号加密 |
|------|---------|---------|---------------|
| AES-256-FF1 | 15.7 µs | 15.8 µs | 15.7 µs |
| SM4-128-FF1 | 17.2 µs | 17.2 µs | 17.2 µs |

**性能差异**: SM4-FF1 比 AES-FF1 慢约 9.5%

运行基准测试：
```bash
cargo bench --bench fpe_benchmark
```

---

## 故障排查

### 链接错误: `libwbcrypto.a not found`

**解决方案**:
1. 确认静态库路径: `ls /tmp/wbcrypto-mode/build/out/libwbcrypto.a`
2. 检查 `.cargo/config.toml` 中的路径是否正确
3. 重新编译 wbcrypto-mode

### 运行时错误: `SM4_FPE_KEY environment variable not set`

**解决方案**:
```bash
export SM4_FPE_KEY=$(openssl rand -hex 16)
```

### 测试失败: `Plaintext length must be at least 6`

SM4-FF1 要求明文长度 ≥ 6 字符。检查测试用例是否使用了过短的字符串。

---

## 安全建议

1. **密钥管理**:
   - 生产环境使用密钥管理服务（AWS KMS / HashiCorp Vault）
   - 切勿在代码或配置文件中硬编码密钥
   - 定期轮换密钥（推荐 90 天）

2. **后端选择**:
   - **合规要求**: 中国大陆项目建议使用 SM4-FF1（符合 GB/T 32907）
   - **国际项目**: 使用 AES-FF1（NIST 标准）
   - **性能敏感**: AES-FF1 性能略优（9.5% faster）

3. **测试覆盖**:
   ```bash
   cargo test
   cargo bench --bench fpe_benchmark
   cargo test --test concurrent_test
   ```

---

## 参考资料

- **SM4 标准**: [GB/T 32907-2016](http://www.gmbz.org.cn/main/viewfile/20180108015408817323.html)
- **AES-FF1 标准**: [NIST SP 800-38G Rev.1](https://csrc.nist.gov/pubs/sp/800/38/g/upd1/final)
- **wbcrypto-mode**: [GitHub Repository](https://github.com/nanshenjiang/wbcrypto-mode)
- **sec-gateway**: [GitHub Repository](https://github.com/geekheitian/sec-gateway)
