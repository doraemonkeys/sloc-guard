
  ---
  sloc-guard 需求清单

  一、核心功能 (MVP)

  | 功能         | 描述                                                    |
  |--------------|---------------------------------------------------------|
  | 文件后缀过滤 | 支持指定多种后缀，如 .rs, .go, .py                      |
  | 行数阈值检查 | 单文件有效代码行数（SLOC）不超过 N 行                   |
  | 跳过空行     | 不计入空白行                                            |
  | 跳过注释     | 支持单行注释 //、多行注释 /* */、文档注释 /// //!       |
  | 排除列表     | 支持排除特定文件/目录，支持 glob 模式 (**/generated/**) |

  ---
  二、扩展功能

  2.1 多语言支持

  | 语言                  | 注释语法               |
  |-----------------------|------------------------|
  | Rust                  | //, /* */, ///, //!    |
  | Go                    | //, /* */              |
  | Python                | #, ''', """            |
  | JavaScript/TypeScript | //, /* */              |
  | C/C++                 | //, /* */              |
  | 自定义                | 允许用户自定义注释语法 |

  2.2 统计维度扩展

  - 函数/方法行数限制：单个函数不超过 N 行（比文件级更精细）
  - 总行数统计：汇总项目总 SLOC
  - 分类统计：代码行 / 注释行 / 空行 分别统计

  2.3 阈值配置灵活性

  - 全局阈值：默认所有文件 ≤ 500 行
  - 按后缀阈值：.rs ≤ 300，.py ≤ 400
  - 按路径阈值：src/generated/** ≤ 1000（生成代码放宽限制）
  - 按文件阈值：特定文件单独配置

  2.4 增量检查

  - 仅检查 git diff 中变更的文件（加速 CI）
  - 支持指定 base commit/branch 对比

  ---
  三、配置管理

  3.1 配置文件格式

  # .sloc-guard.toml
  [default]
  max_lines = 500
  extensions = ["rs", "go", "py"]

  [rules.rust]
  extensions = ["rs"]
  max_lines = 300
  skip_comments = true
  skip_blank = true

  [exclude]
  patterns = [
      "**/target/**",
      "**/generated/**",
      "**/*.generated.rs",
  ]

  # 特定文件覆盖
  [[override]]
  path = "src/legacy/big_file.rs"
  max_lines = 800
  reason = "Legacy code, scheduled for refactor"

  3.2 配置查找顺序

  1. 命令行参数（最高优先级）
  2. 项目根目录 .sloc-guard.toml
  3. $HOME/.config/sloc-guard/config.toml
  4. 内置默认值

  ---
  四、输出与报告

  4.1 输出格式

  | 格式     | 用途                      |
  |----------|---------------------------|
  | text     | 终端友好，带颜色高亮      |
  | json     | 程序解析，CI 集成         |
  | sarif    | GitHub Code Scanning 集成 |
  | markdown | PR Comment 使用           |

  4.2 输出内容

  ❌ FAILED: src/parser/mod.rs
     Lines: 523 (limit: 500)
     Breakdown: code=480, comment=38, blank=5

  ⚠️ WARNING: src/utils.rs
     Lines: 480 (limit: 500, 96% used)

  ✅ Checked 128 files, 2 failed, 3 warnings

  4.3 详细报告

  - --report 生成 HTML/Markdown 报告
  - 包含：趋势图、Top 10 大文件、各目录统计

  ---
  五、CI/CD 集成

  | 功能            | 描述                               |
  |-----------------|------------------------------------|
  | Exit Code       | 0=通过, 1=超限, 2=配置错误         |
  | GitHub Action   | 提供官方 Action                    |
  | Pre-commit Hook | 提供 hook 配置                     |
  | Warning 模式    | --warn-only 只警告不失败           |
  | Baseline        | 允许现有超限文件"豁免"，只检查新增 |

  ---
  六、CLI 设计

  # 基础用法
  sloc-guard check ./src

  # 指定配置
  sloc-guard check --config .sloc-guard.toml

  # 覆盖参数
  sloc-guard check --max-lines 300 --ext rs,go

  # 输出格式
  sloc-guard check --format json

  # 增量模式
  sloc-guard check --diff origin/main

  # 仅检查特定文件
  sloc-guard check src/main.rs src/lib.rs

  # 生成默认配置
  sloc-guard init

  # 显示统计（不检查阈值）
  sloc-guard stats ./src

