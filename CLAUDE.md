# TBay - 技术分析指标库

TBay 是一个用 Rust 编写的金融技术分析指标库，用于计算各种常用技术指标。

## 技术栈

- **语言**: Rust (edition 2024)
- **工具链**: 1.94.0
- **核心依赖**:
  - `rust_decimal` - 高精度小数计算
  - `snafu` - 错误处理
  - `time` - 时间处理

## 项目结构

```
src/
├── lib.rs          # 库入口
├── prelude.rs      # 公共导入
├── res.rs         # 错误类型
├── util.rs        # 工具模块
│   └── ring_buffer.rs
└── indicator/
    ├── indicator.rs   # 核心 trait 定义
    ├── ma/            # 移动平均 (SMA, EMA)
    ├── macd.rs        # MACD 指标
    ├── bollinger.rs   # 布林带
    ├── cross.rs       # 交叉信号
    └── stddev.rs     # 标准差
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `make build` | 构建发布版本 |
| `make test` | 运行测试 |
| `make fmt` | 代码格式化 |
| `make lint` | 代码检查 |

## 开发规范

代码改动完成后，必须依次执行以下三步检查，确保没有 error 级别的错误：

1. `make build` - 编译检查
2. `make fmt` - 格式化检查
3. `make lint` - 代码检查

只有全部通过后才能提交代码。

### 模块组织规范

新增模块 (module, mod) 时，必须使用 Rust Edition 2018 引入的现代模块入口模式：
- 使用 `xxxx.rs` 作为模块入口文件
- 避免使用 `xxxx/mod.rs` 传统模式

例如：
- ✅ `src/indicator/cross.rs`
- ❌ `src/indicator/cross/mod.rs`

## 核心概念

### 数据结构

- **KRaw**: 原始 K 线数据（时间、开盘价、收盘价、最高价、最低价、成交量）
- **PriceBar**: 价格区间（高价、低价、中间价）
- **KInfo**: K 线信息（包含原始数据、价格区间）
- **KSummary**: K 线汇总（以键值对存储基础指标）

### 核心 Trait

- **Indicator**: 通用指标接口
  - `update()` - 针对已完成的 K 线数据进行更新
  - `calc()` - 对尚未结束、仍在变化的量进行计算，发现潜在的指标变化
  - `state()` - 获取当前状态
- **BaseIndicator**: 基础指标接口（设计中，尚未实现）
  - 提供基础量，其计算值可能被多个复杂指标复用
  - 执行顺序：先完成所有 BaseIndicator 的计算，再执行上层指标
  - 例如：布林带和均线穿越可能都用到 EMA20，执行时会先计算 EMA20

### 指标实现模式

新增指标需实现 `Indicator` trait，参考现有实现：
- `src/indicator/ma/sma.rs` - SMA 实现
- `src/indicator/ma/ema.rs` - EMA 实现
- `src/indicator/macd.rs` - MACD 实现
- `src/indicator/bollinger.rs` - 布林带实现

## 贡献流程

### 开发流程

1. **创建分支**: 从 `dev` 创建特性分支
   ```bash
   git checkout -b feat/xxx
   ```

2. **开发与测试**: 在分支上进行开发，确保通过所有检查

3. **代码检查**: 提交前必须依次执行：
   ```bash
   make build
   make fmt
   make lint
   ```

4. **提交与推送**: 提交代码并推送到远程仓库

5. **合并**: 通过内部 code review 后合并到 `dev`

### 分支命名规范

- `feat/xxx` - 新功能
- `fix/xxx` - Bug 修复
- `refact/xxx` - 代码重构

### 提交信息规范

使用conventional commits风格：
- `feat:` - 新功能
- `fix:` - Bug 修复
- `refact:` - 重构
- `docs:` - 文档
- `chore:` - 构建/工具类
