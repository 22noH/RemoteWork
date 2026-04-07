import { createRequire } from 'module'
import { writeFileSync, mkdirSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const require = createRequire(import.meta.url)

const sharp = require('sharp')
const png2icons = require('png2icons')

const ROOT = join(__dirname, '..')
const RESOURCES = join(ROOT, 'resources')

mkdirSync(RESOURCES, { recursive: true })

// SVG icon: monitor with connection arrow
const SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#1a237e"/>
      <stop offset="100%" style="stop-color:#0d47a1"/>
    </linearGradient>
  </defs>
  <rect width="1024" height="1024" rx="180" fill="url(#bg)"/>
  <!-- Monitor body -->
  <rect x="162" y="200" width="700" height="480" rx="40" fill="none" stroke="#90caf9" stroke-width="36"/>
  <!-- Monitor screen -->
  <rect x="202" y="240" width="620" height="400" rx="20" fill="#1565c0" opacity="0.7"/>
  <!-- Monitor stand -->
  <rect x="462" y="680" width="100" height="100" fill="#90caf9"/>
  <rect x="362" y="780" width="300" height="36" rx="18" fill="#90caf9"/>
  <!-- Connection arrows -->
  <path d="M 300 440 L 460 440 L 420 400 M 460 440 L 420 480" stroke="#4fc3f7" stroke-width="28" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
  <path d="M 724 440 L 564 440 L 604 400 M 564 440 L 604 480" stroke="#4fc3f7" stroke-width="28" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
  <!-- Center dot -->
  <circle cx="512" cy="440" r="40" fill="#4fc3f7"/>
</svg>`

console.log('Generating 1024x1024 PNG...')
const pngBuf = await sharp(Buffer.from(SVG)).resize(1024, 1024).png().toBuffer()
writeFileSync(join(RESOURCES, 'icon.png'), pngBuf)
console.log('  ✓ resources/icon.png')

console.log('Generating ICO (multi-size)...')
const icoBuf = png2icons.createICO(pngBuf, png2icons.BILINEAR, 0, true)
writeFileSync(join(RESOURCES, 'icon.ico'), icoBuf)
console.log('  ✓ resources/icon.ico')

console.log('Generating ICNS...')
const icnsBuf = png2icons.createICNS(pngBuf, png2icons.BILINEAR, 0)
writeFileSync(join(RESOURCES, 'icon.icns'), icnsBuf)
console.log('  ✓ resources/icon.icns')

console.log('Done!')
