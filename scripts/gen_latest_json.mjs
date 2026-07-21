/**
 * 生成 GitHub Releases 用的 latest.json（Tauri updater 清单）
 *
 * 用法：
 *   node scripts/gen_latest_json.mjs \
 *     --version 0.1.1 \
 *     --notes "修复课标同步" \
 *     --url "https://github.com/zhangnuli/shijuan-shenqi/releases/download/v0.1.1/试卷神器_0.1.1_x64-setup.nsis.zip" \
 *     --sig "签名内容或.sig文件路径"
 *
 * 签名：打包时设置 TAURI_SIGNING_PRIVATE_KEY_PATH=.tauri/shijuan.key 会生成 .sig
 */
import fs from 'node:fs'
import path from 'node:path'

function arg(name, fallback = '') {
  const i = process.argv.indexOf(`--${name}`)
  if (i >= 0 && process.argv[i + 1]) return process.argv[i + 1]
  return fallback
}

const version = arg('version', '0.1.1')
const notes = arg('notes', `v${version}`)
const url = arg('url', '')
const sigRaw = arg('sig', '')
const out = arg('out', 'latest.json')

if (!url) {
  console.error('缺少 --url 安装包下载地址（通常是 nsis.zip 或 带 .sig 的更新产物）')
  process.exit(1)
}

let signature = sigRaw
if (sigRaw && fs.existsSync(sigRaw)) {
  signature = fs.readFileSync(sigRaw, 'utf8').trim()
}
if (!signature) {
  console.warn('警告：未提供 --sig，latest.json 将无法被客户端校验。请使用打包生成的 .sig 内容。')
}

const payload = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: signature || 'REPLACE_WITH_SIGNATURE',
      url,
    },
  },
}

fs.writeFileSync(out, JSON.stringify(payload, null, 2), 'utf8')
console.log('Wrote', path.resolve(out))
console.log(JSON.stringify(payload, null, 2))
