# scripts/generate-high-dpi-icons.ps1
# Generates a high-DPI Windows ICO file and raw RGBA buffers for Evergreen Browser.
param(
    [string]$SourcePng = "crates/app/ui/logo.png",
    [string]$TargetIco = "crates/app/ui/icon.ico",
    [string]$TargetRgba64 = "crates/app/ui/icon_64.rgba",
    [string]$TargetRgba32 = "crates/app/ui/icon_32.rgba"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

Write-Host "Loading source image from $SourcePng..."
$src = [System.Drawing.Image]::FromFile((Resolve-Path $SourcePng).Path)

# Desired icon sizes for full Windows DPI scaling:
# 16: Small icon, tray, detail list (100% DPI)
# 24: Small icon (150% DPI)
# 32: Standard desktop icon (100% DPI)
# 48: Taskbar icon (100% DPI), Medium icon
# 64: Taskbar icon (150-200% DPI), Large icon
# 128: Extra Large / Jump list
# 256: Jumbo icon (256x256, Alt-Tab, Windows Search)
$sizes = @(16, 24, 32, 48, 64, 128, 256)

$pngEntries = @()

foreach ($size in $sizes) {
    Write-Host "Rendering crisp ${size}x${size} layer..."
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    
    # Enable maximum quality anti-aliasing and bicubic interpolation
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    # Compute aspect-preserving fit with subtle margin padding (4%)
    $pad = [Math]::Max(1.0, [Math]::Round($size * 0.04))
    $availW = $size - (2 * $pad)
    $availH = $size - (2 * $pad)

    $srcAspect = $src.Width / [double]$src.Height
    $boxAspect = $availW / [double]$availH

    if ($srcAspect -gt $boxAspect) {
        $destW = $availW
        $destH = $availW / $srcAspect
    } else {
        $destH = $availH
        $destW = $availH * $srcAspect
    }

    $destX = $pad + ($availW - $destW) / 2.0
    $destY = $pad + ($availH - $destH) / 2.0

    $destRect = New-Object System.Drawing.RectangleF $destX, $destY, $destW, $destH
    $srcRect = New-Object System.Drawing.RectangleF 0, 0, $src.Width, $src.Height

    $g.DrawImage($src, $destRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
    $g.Dispose()

    # Save to PNG memory stream
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngBytes = $ms.ToArray()
    $ms.Dispose()

    $pngEntries += [PSCustomObject]@{
        Size = $size
        Bytes = $pngBytes
        Bitmap = $bmp
    }
}

# Write 64x64 raw RGBA buffer for winit window icon
$entry64 = $pngEntries | Where-Object { $_.Size -eq 64 }
$bmp64 = $entry64.Bitmap
$rgba64 = New-Object byte[] (64 * 64 * 4)
$idx = 0
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $pixel = $bmp64.GetPixel($x, $y)
        $rgba64[$idx]   = $pixel.R
        $rgba64[$idx+1] = $pixel.G
        $rgba64[$idx+2] = $pixel.B
        $rgba64[$idx+3] = $pixel.A
        $idx += 4
    }
}
[System.IO.File]::WriteAllBytes((Resolve-Path (Split-Path $TargetRgba64 -Parent)).Path + "\" + (Split-Path $TargetRgba64 -Leaf), $rgba64)
Write-Host "Exported $($rgba64.Length) bytes to $TargetRgba64"

# Write 32x32 raw RGBA buffer
$entry32 = $pngEntries | Where-Object { $_.Size -eq 32 }
$bmp32 = $entry32.Bitmap
$rgba32 = New-Object byte[] (32 * 32 * 4)
$idx = 0
for ($y = 0; $y -lt 32; $y++) {
    for ($x = 0; $x -lt 32; $x++) {
        $pixel = $bmp32.GetPixel($x, $y)
        $rgba32[$idx]   = $pixel.R
        $rgba32[$idx+1] = $pixel.G
        $rgba32[$idx+2] = $pixel.B
        $rgba32[$idx+3] = $pixel.A
        $idx += 4
    }
}
[System.IO.File]::WriteAllBytes((Resolve-Path (Split-Path $TargetRgba32 -Parent)).Path + "\" + (Split-Path $TargetRgba32 -Leaf), $rgba32)
Write-Host "Exported $($rgba32.Length) bytes to $TargetRgba32"

# Assemble multi-image ICO binary
# Header: 6 bytes (idReserved=0, idType=1, idCount=N)
# Directory entries: N * 16 bytes
# Image data: Sum of image byte lengths
$count = $pngEntries.Count
$headerSize = 6 + ($count * 16)
$currentOffset = $headerSize

$icoStream = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $icoStream

# ICO Header
$bw.Write([uint16]0)       # Reserved
$bw.Write([uint16]1)       # Type 1 = Icon
$bw.Write([uint16]$count)   # Number of images

# Calculate offsets and write directory entries
foreach ($entry in $pngEntries) {
    $s = $entry.Size
    $widthByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }
    $heightByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }

    $bw.Write($widthByte)       # Width (0 for 256)
    $bw.Write($heightByte)      # Height (0 for 256)
    $bw.Write([byte]0)          # Color count (0 for >=8bpp)
    $bw.Write([byte]0)          # Reserved
    $bw.Write([uint16]1)        # Color planes (1)
    $bw.Write([uint16]32)       # Bits per pixel (32bpp)
    $bw.Write([uint32]$entry.Bytes.Length) # Length of image data
    $bw.Write([uint32]$currentOffset)      # Offset of image data

    $currentOffset += $entry.Bytes.Length
}

# Write image data
foreach ($entry in $pngEntries) {
    $bw.Write($entry.Bytes)
    $entry.Bitmap.Dispose()
}

$bw.Flush()
$icoBytes = $icoStream.ToArray()
$bw.Close()
$icoStream.Dispose()
$src.Dispose()

$icoFullPath = (Resolve-Path (Split-Path $TargetIco -Parent)).Path + "\" + (Split-Path $TargetIco -Leaf)
[System.IO.File]::WriteAllBytes($icoFullPath, $icoBytes)
Write-Host "Successfully generated High-DPI icon with $($count) layers at $TargetIco ($($icoBytes.Length) bytes)"

