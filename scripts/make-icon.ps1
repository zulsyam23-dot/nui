param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\assets")
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$OutDir = [System.IO.Path]::GetFullPath($OutDir)
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function New-RoundedPath {
    param(
        [single]$X,
        [single]$Y,
        [single]$Width,
        [single]$Height,
        [single]$Radius
    )

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $diameter = $Radius * 2
    $path.AddArc($X, $Y, $diameter, $diameter, 180, 90)
    $path.AddArc($X + $Width - $diameter, $Y, $diameter, $diameter, 270, 90)
    $path.AddArc($X + $Width - $diameter, $Y + $Height - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($X, $Y + $Height - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    return $path
}

function New-LogoBitmap {
    param([int]$Size)

    $bitmap = New-Object System.Drawing.Bitmap(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $scale = $Size / 512.0

    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $graphics.Clear([System.Drawing.Color]::Transparent)

        $background = New-RoundedPath (16 * $scale) (16 * $scale) (480 * $scale) (480 * $scale) (112 * $scale)
        $gradient = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            [System.Drawing.PointF]::new(16 * $scale, 16 * $scale),
            [System.Drawing.PointF]::new(496 * $scale, 496 * $scale),
            [System.Drawing.ColorTranslator]::FromHtml("#6366F1"),
            [System.Drawing.ColorTranslator]::FromHtml("#06B6D4")
        )
        try {
            $graphics.FillPath($gradient, $background)
        }
        finally {
            $gradient.Dispose()
            $background.Dispose()
        }

        if ($Size -ge 32) {
            $inner = New-RoundedPath (96 * $scale) (96 * $scale) (320 * $scale) (320 * $scale) (56 * $scale)
            $innerPen = New-Object System.Drawing.Pen(
                [System.Drawing.Color]::FromArgb(46, 255, 255, 255),
                [single][Math]::Max(0.75, 6 * $scale)
            )
            try {
                $graphics.DrawPath($innerPen, $inner)
            }
            finally {
                $innerPen.Dispose()
                $inner.Dispose()
            }
        }

        $points = [System.Drawing.PointF[]]@(
            [System.Drawing.PointF]::new(160 * $scale, 368 * $scale),
            [System.Drawing.PointF]::new(160 * $scale, 144 * $scale),
            [System.Drawing.PointF]::new(352 * $scale, 368 * $scale),
            [System.Drawing.PointF]::new(352 * $scale, 144 * $scale)
        )
        $linePen = New-Object System.Drawing.Pen(
            [System.Drawing.Color]::White,
            [single](46 * $scale)
        )
        try {
            $linePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
            $linePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
            $linePen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
            $graphics.DrawLines($linePen, $points)
        }
        finally {
            $linePen.Dispose()
        }

        $nodeBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
        try {
            $radius = 38 * $scale
            foreach ($point in $points) {
                $graphics.FillEllipse(
                    $nodeBrush,
                    [single]($point.X - $radius),
                    [single]($point.Y - $radius),
                    [single]($radius * 2),
                    [single]($radius * 2)
                )
            }
        }
        finally {
            $nodeBrush.Dispose()
        }
    }
    finally {
        $graphics.Dispose()
    }

    return $bitmap
}

foreach ($size in @(512, 256, 64, 32)) {
    $bitmap = New-LogoBitmap $size
    try {
        $bitmap.Save(
            (Join-Path $OutDir "logo-$size.png"),
            [System.Drawing.Imaging.ImageFormat]::Png
        )
    }
    finally {
        $bitmap.Dispose()
    }
}

$iconSizes = @(16, 24, 32, 48, 64, 128, 256)
$images = @()
foreach ($size in $iconSizes) {
    $bitmap = New-LogoBitmap $size
    $stream = New-Object System.IO.MemoryStream
    try {
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        $images += ,$stream.ToArray()
    }
    finally {
        $stream.Dispose()
        $bitmap.Dispose()
    }
}

$iconStream = New-Object System.IO.MemoryStream
$writer = New-Object System.IO.BinaryWriter($iconStream)
try {
    $writer.Write([UInt16]0)
    $writer.Write([UInt16]1)
    $writer.Write([UInt16]$images.Count)

    $offset = 6 + (16 * $images.Count)
    for ($index = 0; $index -lt $images.Count; $index++) {
        $size = $iconSizes[$index]
        $dimension = if ($size -ge 256) { [byte]0 } else { [byte]$size }
        $writer.Write($dimension)
        $writer.Write($dimension)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$images[$index].Length)
        $writer.Write([UInt32]$offset)
        $offset += $images[$index].Length
    }

    foreach ($image in $images) {
        $writer.Write($image)
    }
    $writer.Flush()
}
finally {
    $writer.Dispose()
}

[System.IO.File]::WriteAllBytes(
    (Join-Path $OutDir "icon.ico"),
    $iconStream.ToArray()
)
$iconStream.Dispose()

Write-Output "Logo dibuat di $OutDir"
