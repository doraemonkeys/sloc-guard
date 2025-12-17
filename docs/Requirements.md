## Requirements
  

  一、核心功能 (MVP)

  | 功能         | 描述                                                    |
  |--------------|---------------------------------------------------------|
  | 文件后缀过滤 | 支持指定多种后缀，如 .rs, .go, .py                      |
  | 行数阈值检查 | 单文件有效代码行数（SLOC）不超过 N 行                   |
  | 跳过空行     | 不计入空白行                                            |
  | 跳过注释     | 支持单行注释 //、多行注释 /* */、文档注释 /// //!       |
  | 扫描目录     | 支持配置只扫描指定目录，如 src, lib（命令行未指定时生效）|
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

  每种语言自带排除目录，如 Rust 的 exclude 目录为 target, .git 等。

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

  ```toml
    # .sloc-guard.toml
  [default]
  max_lines = 500
  extensions = ["rs", "go", "py", "js", "ts", "c", "cpp"]
  include_paths = ["src", "lib"]
  skip_comments = true
  skip_blank = true
  warn_threshold = 0.9
  
  # 基于扩展名的规则
  [rules.rust]
  extensions = ["rs"]
  max_lines = 300
  warn_threshold = 0.85
  
  [rules.python]
  extensions = ["py"]
  max_lines = 400
  
  # 基于路径模式的规则（新增，优先于 rules）
  [[path_rules]]
  pattern = "src/generated/**"
  max_lines = 1000
  warn_threshold = 1.0  # 生成代码不警告
  
  [[path_rules]]
  pattern = "**/proto/**"
  max_lines = 800
  
  # 排除
  [exclude]
  patterns = [
      "**/target/**",
      "**/node_modules/**",
      "**/*.generated.rs",
  ]
  
  # 特定文件覆盖（最高优先级）
  [[override]]
  path = "src/legacy/parser.rs"
  max_lines = 800
  skip_comments = false
  reason = "Legacy code, scheduled for Q2 refactor"
  ```

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
  六、CLI 设计（配置文件优先，CLI 用于快捷操作和覆盖）

  6.1 全局选项

  | 选项 | 短写 | 描述 |
  |------|------|------|
  | --verbose | -v | 增加输出详细度（-v, -vv） |
  | --quiet | -q | 静默模式，仅输出必要信息 |
  | --color | | 控制颜色输出（auto/always/never） |
  | --no-config | | 跳过配置文件加载 |

  6.2 check 命令

  ```bash
  # 基础用法
  sloc-guard check ./src

  # 指定配置文件
  sloc-guard check --config .sloc-guard.toml

  # 覆盖阈值参数
  sloc-guard check --max-lines 300 --ext rs,go

  # 排除和包含路径
  sloc-guard check -x "**/target/**" -x "**/vendor/**"
  sloc-guard check -I src -I lib

  # 跳过注释/空行开关
  sloc-guard check --no-skip-comments --no-skip-blank

  # 警告阈值
  sloc-guard check --warn-threshold 0.8

  # 输出格式与文件
  sloc-guard check --format json --output report.json

  # 仅警告模式
  sloc-guard check --warn-only

  # 增量模式（仅检查 git 变更的文件）
  sloc-guard check --diff origin/main

  # 仅检查特定文件
  sloc-guard check src/main.rs src/lib.rs
  ```

  6.3 stats 命令

  ```bash
  # 显示统计（不检查阈值）
  sloc-guard stats ./src

  # 使用配置文件（获取 exclude 等设置）
  sloc-guard stats --config .sloc-guard.toml

  # 排除和包含
  sloc-guard stats -x "**/test/**" -I src

  # 输出为 JSON
  sloc-guard stats --format json --output stats.json
  ```

  6.4 init 命令

  ```bash
  # 生成默认配置
  sloc-guard init

  # 指定输出路径
  sloc-guard init --output custom-config.toml

  # 强制覆盖已存在的文件
  sloc-guard init --force
  ```

  6.5 config 命令

  ```bash
  # 验证配置文件语法
  sloc-guard config validate
  sloc-guard config validate --config custom.toml

  # 显示合并后的有效配置
  sloc-guard config show
  sloc-guard config show --format json
  ```

  6.6 完整选项参考

  | 命令 | 选项 | 短写 | 描述 |
  |------|------|------|------|
  | check | --config | -c | 配置文件路径 |
  | check | --max-lines | | 最大行数阈值 |
  | check | --ext | | 文件扩展名（逗号分隔） |
  | check | --exclude | -x | 排除模式（可多次指定） |
  | check | --include | -I | 包含目录（可多次指定） |
  | check | --no-skip-comments | | 计入注释行 |
  | check | --no-skip-blank | | 计入空行 |
  | check | --warn-threshold | | 警告阈值（0.0-1.0） |
  | check | --format | -f | 输出格式（text/json/sarif/markdown） |
  | check | --output | -o | 输出到文件 |
  | check | --warn-only | | 仅警告不失败 |
  | check | --diff | | Git 对比引用 |
  | stats | --config | -c | 配置文件路径 |
  | stats | --ext | | 文件扩展名 |
  | stats | --exclude | -x | 排除模式 |
  | stats | --include | -I | 包含目录 |
  | stats | --format | -f | 输出格式 |
  | stats | --output | -o | 输出到文件 |
  | init | --output | -o | 输出路径 |
  | init | --force | | 强制覆盖 |
  | config validate | --config | -c | 配置文件路径 |
  | config show | --config | -c | 配置文件路径 |
  | config show | --format | -f | 输出格式 |