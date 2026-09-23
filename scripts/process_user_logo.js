const fs = require('fs');
const path = require('path');
const cachePath = 'C:\\Users\\parth\\AppData\\Local\\npm-cache\\_npx\\e41f203b7505f1fb\\node_modules\\playwright';
const { chromium } = require(cachePath);

const srcPath = 'C:/Users/parth/.gemini/antigravity/brain/debc37a0-4a52-4c5c-a5f8-d4b50dce6d1d/.user_uploaded/media_1790095660571.png';
const repoRoot = path.resolve(__dirname, '..');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const b64 = fs.readFileSync(srcPath).toString('base64');
  await page.setContent(`<!DOCTYPE html><html><body><img id="img" src="data:image/png;base64,${b64}"><canvas id="c"></canvas></body></html>`);
  
  const result = await page.evaluate(() => {
    const img = document.getElementById('img');
    const c = document.getElementById('c');
    const w = img.naturalWidth;
    const h = img.naturalHeight;
    c.width = w;
    c.height = h;
    const ctx = c.getContext('2d');
    ctx.drawImage(img, 0, 0);
    const imgData = ctx.getImageData(0, 0, w, h);
    const data = imgData.data;

    // Background color components
    const Br = 20.0;
    const Bg = 24.0;
    const Bb = 33.0;

    // Orb center & geometry:
    // x in [243, 268], y in [20, 44]
    const orbCx = 256.0;
    const orbCy = 32.0;

    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4;
        const r = data[i];
        const g = data[i + 1];
        const b = data[i + 2];

        const isOrbRegion = (x >= 243 && y <= 45);

        if (!isOrbRegion) {
          // --- Text: "Evergreen" ---
          const dR = r - Br;
          const dG = g - Bg;
          const dB = b - Bb;
          const lumDiff = 0.299 * dR + 0.587 * dG + 0.114 * dB;

          if (lumDiff <= 3.0) {
            data[i] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;
            data[i + 3] = 0;
          } else {
            let alpha;
            if (lumDiff >= 130.0) {
              alpha = 1.0;
            } else {
              const t = (lumDiff - 3.0) / (130.0 - 3.0);
              alpha = t * t * (3.0 - 2.0 * t);
            }

            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = Math.round(Math.min(1.0, Math.max(0.0, alpha)) * 255);
          }
        } else {
          // --- Orb (Teal to Emerald circle) ---
          const dx = x - orbCx;
          const dy = y - orbCy;
          const dist = Math.sqrt(dx * dx + dy * dy);

          const dR = r - Br;
          const dG = g - Bg;
          const dB = b - Bb;
          const maxDelta = Math.max(dG, dB);

          if (dist > 12.0 || maxDelta <= 3.0) {
            data[i] = 0;
            data[i + 1] = 0;
            data[i + 2] = 0;
            data[i + 3] = 0;
          } else {
            let alpha = 1.0;
            if (dist > 9.5) {
              const t = (11.5 - dist) / 2.0;
              alpha = Math.max(0.0, Math.min(1.0, t * t * (3.0 - 2.0 * t)));
            }
            const colorT = Math.min(1.0, Math.max(0.0, maxDelta / 80.0));
            alpha = Math.min(alpha, colorT);

            if (alpha <= 0.02) {
              data[i] = 0;
              data[i + 1] = 0;
              data[i + 2] = 0;
              data[i + 3] = 0;
            } else {
              const unblendR = Math.min(255, Math.max(0, Math.round((r - (1.0 - alpha) * Br) / alpha)));
              const unblendG = Math.min(255, Math.max(0, Math.round((g - (1.0 - alpha) * Bg) / alpha)));
              const unblendB = Math.min(255, Math.max(0, Math.round((b - (1.0 - alpha) * Bb) / alpha)));

              data[i] = unblendR;
              data[i + 1] = unblendG;
              data[i + 2] = unblendB;
              data[i + 3] = Math.round(alpha * 255);
            }
          }
        }
      }
    }

    ctx.putImageData(imgData, 0, 0);

    // Return RGBA array and data URL
    const rawRgba = Array.from(data);
    return {
      w,
      h,
      dataUrl: c.toDataURL('image/png'),
      rawRgba
    };
  });

  await browser.close();

  const { w, h, dataUrl, rawRgba } = result;
  const base64Png = dataUrl.replace(/^data:image\/png;base64,/, '');
  const pngBuffer = Buffer.from(base64Png, 'base64');

  // 1. Save docs/assets/logo_full.png
  const targetLogoPath = path.join(repoRoot, 'docs', 'assets', 'logo_full.png');
  fs.writeFileSync(targetLogoPath, pngBuffer);
  console.log(`Saved ${targetLogoPath} (${pngBuffer.length} bytes, ${w}x${h})`);

  // 2. Save crates/app/ui/logo_full.b64
  const targetB64Path = path.join(repoRoot, 'crates', 'app', 'ui', 'logo_full.b64');
  fs.writeFileSync(targetB64Path, base64Png);
  console.log(`Saved ${targetB64Path}`);

  // Also update crates/app/ui/logo_full.png
  const targetAppUiPng = path.join(repoRoot, 'crates', 'app', 'ui', 'logo_full.png');
  fs.writeFileSync(targetAppUiPng, pngBuffer);
  console.log(`Saved ${targetAppUiPng}`);

  // 3. Generate 32-bit premultiplied BGRA binary for Win32 GDI AlphaBlend
  // Format: [B, G, R, A] where B = round(B * A / 255), G = round(G * A / 255), R = round(R * A / 255)
  const bgraBuffer = Buffer.alloc(w * h * 4);
  for (let idx = 0; idx < w * h; idx++) {
    const srcIdx = idx * 4;
    const r = rawRgba[srcIdx];
    const g = rawRgba[srcIdx + 1];
    const b = rawRgba[srcIdx + 2];
    const a = rawRgba[srcIdx + 3];

    const dstIdx = idx * 4;
    if (a === 0) {
      bgraBuffer[dstIdx] = 0;
      bgraBuffer[dstIdx + 1] = 0;
      bgraBuffer[dstIdx + 2] = 0;
      bgraBuffer[dstIdx + 3] = 0;
    } else {
      const scale = a / 255.0;
      bgraBuffer[dstIdx] = Math.round(b * scale);     // B
      bgraBuffer[dstIdx + 1] = Math.round(g * scale); // G
      bgraBuffer[dstIdx + 2] = Math.round(r * scale); // R
      bgraBuffer[dstIdx + 3] = a;                     // A
    }
  }

  const targetBgraPath = path.join(repoRoot, 'crates', 'installer', 'assets', 'logo_full_bgra.bin');
  fs.writeFileSync(targetBgraPath, bgraBuffer);
  console.log(`Saved ${targetBgraPath} (${bgraBuffer.length} bytes, ${w}x${h})`);

  console.log('Successfully processed user logo and generated all transparent assets.');
})();

