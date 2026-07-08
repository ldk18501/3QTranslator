# 3Q语言助手

一款 Windows 优先的免费翻译与外语学习桌面软件。当前使用 Tauri 2、React、TypeScript 和 SQLite 实现，包含文本翻译、单词本、快捷键入口、开机自启和截图 OCR 翻译。

## 本机依赖

- Node.js 已安装时可直接使用 `npm.cmd`，避免 PowerShell 执行策略拦截 `npm.ps1`。
- 需要安装 Rustup、Microsoft C++ Build Tools 和 WebView2 Runtime 才能运行 Tauri 桌面端。

## 开发命令

```powershell
npm.cmd install
npm.cmd run dev
npm.cmd run tauri:dev
npm.cmd run tauri:build
```

## 功能说明

- 浏览器开发模式会使用 localStorage 和公开 API fallback。
- Tauri 桌面模式会调用 Rust 命令，并使用 SQLite 保存单词本和设置。
- 每日学习功能暂时关闭，避免用本地固定词库伪装成每日 AI 生成。
- 截图翻译会进入全屏框选模式，截取选区后调用 Windows OCR 识别并翻译。
- 中文 OCR 依赖 Windows 已安装中文 OCR 语言包。

## 翻译源配置

进入 `设置 -> 翻译源配置`，填写对应字段，启用该源，然后在 `当前使用翻译源` 中选择它并保存。保存后可以点击 `测试连接`，成功或失败都会在设置页顶部提示。

### 腾讯云机器翻译

推荐作为中文场景主力源。

- 类型：`腾讯云机器翻译`
- Base URL：`https://tmt.tencentcloudapi.com`
- SecretId：腾讯云访问密钥 SecretId
- SecretKey：腾讯云访问密钥 SecretKey
- 区域：建议 `ap-guangzhou`

准备方式：

1. 登录腾讯云控制台。
2. 开通机器翻译 TMT 服务。
3. 在访问管理 CAM 中创建或查看 API 密钥。
4. 将 SecretId、SecretKey 填入应用设置。
5. 建议在腾讯云侧设置预算或额度告警。

### Azure Translator

适合作为稳定的官方备用源。

- 类型：`Azure Translator`
- Base URL：`https://api.cognitive.microsofttranslator.com`
- API Key：Azure Translator 资源的密钥
- 区域：资源所在区域，例如 `eastasia`、`westus`，以 Azure 控制台显示为准

准备方式：

1. 在 Azure 创建 Translator 或 Azure AI services 资源。
2. 在资源页面复制 Key。
3. 填写资源所在 Region。
4. 保存后点击测试连接。

### DeepL API

适合欧语质量优先的场景。

- 类型：`DeepL API`
- Base URL：免费版使用 `https://api-free.deepl.com/v2`，Pro 版使用 `https://api.deepl.com/v2`
- API Key：DeepL API key。程序会按官方当前要求通过 `Authorization: DeepL-Auth-Key ...` 请求头发送，不需要也不要把 key 拼到 Base URL 里。

准备方式：

1. 注册 DeepL API 账号。
2. 在 DeepL 账号后台复制 API key。
3. 免费版保留默认 Base URL；Pro 版改成 Pro 地址。
4. 保存后点击测试连接。

注意：DeepL 已废弃把 `auth_key` 放在 URL 或请求体里的旧认证方式，免费版接口会返回 `403 Forbidden`。如果测试失败，优先确认账号是 Free 还是 Pro，以及 Base URL 是否对应。

### 百度翻译开放平台

适合作为国内低门槛备用源。

- 类型：`百度翻译开放平台`
- Base URL：`https://fanyi-api.baidu.com/api/trans/vip/translate`
- AppID：百度翻译应用 AppID
- 密钥：百度翻译应用密钥

准备方式：

1. 登录百度翻译开放平台。
2. 创建通用翻译 API 应用。
3. 复制 AppID 和密钥。
4. 填入应用设置并测试连接。

## 兜底源说明

- `MyMemory 免费源` 默认启用，但质量和稳定性有限，容易对技术文本原样返回。
- 应用会把“译文与原文几乎一样”的结果视为失败，并尝试 Google 免费接口 fallback。
- Google 免费接口不是正式商业 API，只适合作为无 key 兜底，不建议当主力源。
