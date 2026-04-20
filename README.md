# VRCText

一个极简、极轻量的 **VRChat OSC 聊天框发送器 + SAPI 语音朗读** 的 Windows 桌面工具。打字→头顶气泡+朗读，绕开游戏内软键盘和录音麦克风。

![Stack](https://img.shields.io/badge/rust-1.75%2B-orange)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)
![Binary](https://img.shields.io/badge/binary-3.2%20MB-5ED08C)
![License](https://img.shields.io/badge/license-MIT-blue)

---

## 为什么做这个

VRChat 的聊天框（头顶文字气泡）只能通过游戏内软键盘输入，鼠标点每一个字符，慢而烦人；而"让队友听到我说话"又需要开麦克风。这个工具把两件事都解决了：

- **快速打字 → 头顶气泡**：通过 VRChat 官方 OSC 协议把文字直接送到头顶
- **文字 → 语音**：用 Windows 自带 SAPI 合成语音，配合虚拟音频线缆可以让游戏里其他玩家"听到你说话"——**不需要任何模型文件、不需要 API key、不需要网络**

目标：
- 单文件 `.exe`，**3.2 MB**，双击即开，无运行时依赖
- 冷启动 <300ms
- UI 现代化，深色主题
- 所有功能开箱即用，不问"先装 .NET"

## 功能

### 核心
- [x] **OSC 聊天框发送** — `Enter` 直发到头顶气泡（`/chatbox/input`）
- [x] **144 字符计数** — 接近上限变黄，超出变红，发送时硬截断
- [x] **打字指示器** — 输入过程中向 VRChat 发 `/chatbox/typing`，队友看到 "..." 气泡
- [x] **发送提示音开关** — 控制 VRChat 收到消息时的 "ding" 声

### 语音朗读（TTS）
- [x] **Windows SAPI 5** — 直接调 COM 接口，不走 PowerShell 子进程
- [x] **语音/语言选择** — 枚举 `HKLM\SOFTWARE\Microsoft\Speech\Voices` 下所有声音，中文系统默认有 Microsoft Huihui
- [x] **输出设备选择** — 可把合成语音定向到指定扬声器/虚拟线缆，不污染常规音频
- [x] **试听按钮** — 不用发送消息就能预听当前选中的语音+设备
- [x] **新消息打断旧的** — `SPF_PURGEBEFORESPEAK`，避免朗读堆积

### 历史记录
- [x] **时间戳滚动列表** — 相对时间（"刚刚"/"X 分钟前"/"今天 HH:MM"/"昨天 HH:MM"/"MM-DD"）
- [x] **悬停重发** — 鼠标悬停在某条历史上，右侧出现 `↺` 按钮
- [x] **短按追加** — 单击把该条追加到当前输入框
- [x] **长按 2 秒直发** — 按住出现进度条，满 2 秒直接重发（不动输入框）
- [x] **↑/↓ 翻阅** — 输入框为空时 `↑/↓` 遍历最近消息，Shell 风格
- [x] **持久化** — 上限 50 条，存 `%APPDATA%\vrctext\vrctext\config\config.toml`

### 窗口行为
- [x] **始终置顶** — 📌 开关，VRChat 全屏窗口化也能叠在上面
- [x] **深色主题** — Slate 调色板（panel/surface/elevated 三级），蓝色 accent
- [x] **一屏集成的设置页** — 点 ⚙ 原地切换到设置视图，不是浮动对话框

## 截图

```
┌────────────────────────────────────────────┐
│ VRCText  ● OSC 127.0.0.1:9000    📌 🔔 🔊 ⚙│  ← 顶栏 48px
├────────────────────────────────────────────┤
│                                            │
│  14:32    你好，大家                  [↺] │  ← 历史卡片
│  昨天 20:15  今天天气怎么样？         [↺] │
│  2 分钟前   测试                      [↺] │
│                                            │
├────────────────────────────────────────────┤
│ ┌────────────────────────────────────────┐│
│ │ 输入消息… [Enter] 发送  [Shift+Enter]   ││  ← 输入框
│ └────────────────────────────────────────┘│
│ 12/144    已发送                  [发送 ⏎] │  ← 字数 + 状态 + 主按钮
└────────────────────────────────────────────┘
```

## 安装

1. **下载** `vrctext.exe`（Releases 页面）
2. **双击运行** — 不需要安装器，不需要管理员权限
3. **VRChat 里开启 OSC**：`Settings → OSC → Enabled`

## 使用

### 发送文字到头顶气泡

1. VRChat 启动并开 OSC
2. 打开 VRCText，在输入框里敲字
3. `Enter` 发送 / `Shift+Enter` 换行
4. 头顶气泡立即出现

### 用 TTS 让队友听到你说话

**前提**：VRChat 的"语音"是 **麦克风输入**，而 SAPI 合成出来的是 **扬声器输出**。要把后者灌进前者，必须装一条虚拟音频线缆。

1. 装 [**VB-Audio Virtual Cable**](https://vb-audio.com/Cable/)（免费）
2. 在 VRCText 设置里：
   - **输出设备** 选 `CABLE Input (VB-Audio Virtual Cable)`
   - 打开 🔊 开关
3. 在 Windows 声音设置里：
   - `CABLE Output` 属性 → 聆听 → 勾选"聆听此设备" → 回放到你的耳机（这样你自己也能听到）
4. VRChat 设置 → 麦克风输入 选 `CABLE Output`
5. 现在在 VRCText 里按 Enter，队友听到你打的字被 SAPI 念出来

> **说明**：不装虚拟线缆时 TTS 只会从你的默认扬声器出声，队友听不到。这是 Windows 音频架构决定的，任何同类 TTS 工具都得这么做。

### 选不同的语音

设置 → 语音朗读 → **语音 / 语言**：

Windows 10/11 默认装有：
- **Microsoft Zira Desktop** (en-US, female)
- **Microsoft David Desktop** (en-US, male)
- **Microsoft Huihui Desktop** (zh-CN, female)

想装更多？`Settings → Time & Language → Speech → Manage voices`，下载新的语言包。

点 **🔈 试听** 不发消息就能预听。

### 历史记录操作

| 操作 | 效果 |
|---|---|
| 鼠标悬停某条 | 右侧出现 `↺` 按钮 |
| 短按 `↺` | 该条内容追加到当前输入框（含空格） |
| 长按 `↺` 2 秒 | 直接重新发送（不动输入框，带进度条） |
| 输入框为空时 `↑` / `↓` | 翻阅最近发送的消息 |
| 设置 → 清空全部历史 | 两步确认，防止误点 |

## 配置文件

```
C:\Users\<你>\AppData\Roaming\vrctext\vrctext\config\config.toml
```

可直接编辑。示例：

```toml
ip = "127.0.0.1"
port = 9000
play_sound = true
always_on_top = false
tts_enabled = true
tts_device_name = "CABLE Input (VB-Audio Virtual C"   # WaveOut 设备名，31 字符上限
tts_voice_name = "Microsoft Huihui Desktop - Chinese (China)"

[[history]]
text = "你好"
ts = 1776659152
```

删除整个文件即可重置为默认。

## 从源码构建

### 依赖
- **Rust** stable 1.75+（[rustup](https://rustup.rs/)）
- **Windows 10/11**（用了 SAPI + winapi，不支持 Linux/macOS）

### 构建命令

```bash
git clone https://github.com/<你>/vrctext.git
cd vrctext

# 调试版（有控制台窗口便于看 println）
cargo run

# 发布版
cargo build --release
# 产物：target/release/vrctext.exe

# 运行诊断工具（不走 GUI，打印 SAPI/WaveOut 各步 HRESULT）
cargo run --release --bin vrctext-diag
```

### 发布 Profile

```toml
[profile.release]
opt-level = "z"       # 体积优先
lto = true            # 跨 crate 内联
codegen-units = 1     # 更激进优化
strip = true          # 去符号表
panic = "abort"       # 去 unwind 机制
```

最终产物 **3.2 MB**。

## 架构

```
src/
├── main.rs        # 入口；#![windows_subsystem = "windows"]；NativeOptions
├── app.rs         # egui::App 实现：状态 + 全部 UI
├── theme.rs       # 调色板 + 样式注入 + 常用 Frame 预设
├── osc.rs         # 手写 OSC 打包（/chatbox/input + /chatbox/typing） + UDP 发送
├── config.rs      # Config::load/save，toml 持久化
├── tts.rs         # SAPI 封装：ISpVoice + SpMMAudioOut + ISpObjectTokenCategory
└── bin/
    └── diag.rs    # 诊断 CLI：枚举设备、逐步测试 SAPI 调用链
```

| 模块 | 外部依赖 |
|---|---|
| `osc.rs` | `std::net::UdpSocket`（无第三方） |
| `tts.rs` | `windows` crate（SAPI COM + WaveOut API） |
| `app.rs` | `eframe`/`egui` + `chrono` |
| `config.rs` | `serde` + `toml` + `directories` |

### OSC 协议细节

VRChat 的 OSC 端点：

| 地址 | 参数 | 说明 |
|---|---|---|
| `/chatbox/input` | `(s, T/F, T/F)` | 文字 + bypassKeyboard + playSound |
| `/chatbox/typing` | `(T/F)` | 是否显示 "..." 打字气泡 |

本项目不依赖 `rosc` crate，手写了 30 行 OSC 编码（4 字节对齐 + `T`/`F` 布尔类型标签）。

### SAPI 实现要点

- 启动时 `CoInitializeEx(APARTMENTTHREADED)` + `CoCreateInstance(SpVoice)` 得到 `ISpVoice`
- 枚举声音：`ISpObjectTokenCategory::SetId("HKLM\\...Voices").EnumTokens()`
- 选声音：`ISpVoice::SetVoice(&ISpObjectToken)`
- 枚举音频设备：`waveOutGetNumDevs()` + `waveOutGetDevCapsW()`
- 路由到指定设备：`CoCreateInstance(SpMMAudioOut) → SetDeviceId(i) → ISpVoice::SetOutput(&IUnknown)`
- 朗读：`ISpVoice::Speak(text, SPF_ASYNC | SPF_PURGEBEFORESPEAK)`

### UDP 在 Windows 的坑

Windows 的 UDP socket 默认会在收到 ICMP Port Unreachable 之后，对下一次 `send_to` 返回 `WSAECONNRESET`——即便 UDP 是无连接协议。程序启动时会主动关掉这个行为：

```rust
WSAIoctl(sock, SIO_UDP_CONNRESET, &0u32 as _, 4, ...)
```

即便不关，TTS/历史记录逻辑也独立于 OSC 成败（OSC 失败只影响状态栏文字，`speak()` + `push_history()` 照样执行）。

## 为什么是 Rust + eframe/egui？

| 方案 | 体积 | 冷启动 | 运行时依赖 |
|---|---|---|---|
| **Rust + egui** ✅ | 3.2 MB | <200ms | 无 |
| C# + WinForms + NativeAOT | 10-15 MB | <100ms | 无（AOT） |
| C++ + Win32/ImGui | <2 MB | <50ms | 无 |
| Python + PyInstaller | 30-80 MB | 3-5s | 无 |
| Electron | 80+ MB | 1-2s | 无 |

egui 是立即模式 GUI，代码简单、产物小、启动快，最贴合"超轻量"这个主诉求。

## 已知限制

- **Windows only** — SAPI / WaveOut / WSAIoctl 都是 Windows API
- **不自带虚拟音频线缆** — 需要用户自己装 VB-Cable 才能让队友听到 TTS
- **中文字体** — 启动时从 `C:\Windows\Fonts\msyh.ttc`（微软雅黑）动态加载；万一系统里没这字体会显示豆腐（极罕见）
- **WAVEOUTCAPSW 设备名 31 字符上限** — 长设备名会被截断（比如 `CABLE Input (VB-Audio Virtual Cable)` 变成 `CABLE Input (VB-Audio Virtual C`），显示和匹配都按截断名字算
- **无系统托盘 / 全局热键** — 目标是极简单窗口，有需要的自己加

## License

MIT

## 致谢

- [egui](https://github.com/emilk/egui) — 立即模式 GUI 框架
- [windows-rs](https://github.com/microsoft/windows-rs) — Rust Windows 绑定
- [VB-Audio Virtual Cable](https://vb-audio.com/Cable/) — 免费虚拟音频线缆
- VRChat OSC 文档：<https://docs.vrchat.com/docs/osc-overview>
