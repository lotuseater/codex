$ErrorActionPreference = 'Stop'
$tool = $env:CODEX_DAB_TOOL
$json = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($env:CODEX_DAB_INPUT_B64))
$ArgsObj = if ([string]::IsNullOrWhiteSpace($json)) { [pscustomobject]@{} } else { $json | ConvertFrom-Json }

Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class CodexDabNative {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool repaint);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT lpPoint);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hWnd, ref POINT point);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
}
'@

try {
    Add-Type -AssemblyName System.Drawing.Common -ErrorAction Stop
} catch {
    Add-Type -AssemblyName System.Drawing
}

function Get-DabCSharpReferences {
    $runtimeMajor = [Environment]::Version.Major
    if ($runtimeMajor -ge 5) {
        $tfm = "net$runtimeMajor.0"
        $packRoot = Join-Path $env:ProgramFiles 'dotnet\packs'
        $packNames = @('Microsoft.NETCore.App.Ref', 'Microsoft.WindowsDesktop.App.Ref')
        $refs = @()
        foreach ($packName in $packNames) {
            $packDir = Join-Path $packRoot $packName
            if (-not (Test-Path -LiteralPath $packDir)) { continue }
            $versionDir = Get-ChildItem -LiteralPath $packDir -Directory |
                Sort-Object Name -Descending |
                Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "ref\$tfm") } |
                Select-Object -First 1
            if ($null -eq $versionDir) { continue }
            $refDir = Join-Path $versionDir.FullName "ref\$tfm"
            $refs += Get-ChildItem -LiteralPath $refDir -Filter '*.dll' | ForEach-Object { $_.FullName }
        }
        if ($refs.Count -gt 0) {
            return $refs | Select-Object -Unique
        }
    }
    @('System.Drawing', 'System.Core')
}

Add-Type -ReferencedAssemblies (Get-DabCSharpReferences) -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.Linq;
using System.Runtime.InteropServices;

public sealed class CodexDabVisualRect {
    public int left { get; set; }
    public int top { get; set; }
    public int right { get; set; }
    public int bottom { get; set; }
    public int width { get { return right - left; } }
    public int height { get { return bottom - top; } }
}

public sealed class CodexDabVisualPoint {
    public int x { get; set; }
    public int y { get; set; }
}

public sealed class CodexDabVisualCandidate {
    public string kind { get; set; }
    public double score { get; set; }
    public CodexDabVisualRect rect { get; set; }
    public CodexDabVisualRect outer_rect { get; set; }
    public CodexDabVisualRect inner_rect { get; set; }
    public CodexDabVisualRect checkbox_rect { get; set; }
    public CodexDabVisualRect checkbox_inner_rect { get; set; }
    public CodexDabVisualPoint center { get; set; }
    public CodexDabVisualPoint click_point { get; set; }
}

public static class CodexDabVision {
    public static double LastElapsedMs { get; private set; }

    struct Component {
        public int Left, Top, Right, Bottom, Count;
    }

    static CodexDabVisualRect Rect(int left, int top, int right, int bottom) {
        return new CodexDabVisualRect { left = left, top = top, right = right, bottom = bottom };
    }

    static int Area(CodexDabVisualRect rect) {
        return Math.Max(0, rect.width) * Math.Max(0, rect.height);
    }

    static bool Contains(CodexDabVisualRect outer, CodexDabVisualRect inner, int padding) {
        return outer.left <= inner.left + padding &&
               outer.top <= inner.top + padding &&
               outer.right >= inner.right - padding &&
               outer.bottom >= inner.bottom - padding;
    }

    static CodexDabVisualRect Clip(CodexDabVisualRect rect, int width, int height) {
        return Rect(Math.Max(0, rect.left), Math.Max(0, rect.top), Math.Min(width, rect.right), Math.Min(height, rect.bottom));
    }

    static CodexDabVisualCandidate Candidate(int left, int top, int right, int bottom, double score, string kind, int offsetX, int offsetY) {
        var rect = Rect(offsetX + left, offsetY + top, offsetX + right, offsetY + bottom);
        int inset = Math.Max(2, Math.Min(8, (int)(Math.Min(rect.width, rect.height) * 0.16)));
        var inner = Rect(rect.left + inset, rect.top + inset, rect.right - inset, rect.bottom - inset);
        var center = new CodexDabVisualPoint { x = (rect.left + rect.right) / 2, y = (rect.top + rect.bottom) / 2 };
        return new CodexDabVisualCandidate {
            kind = kind,
            score = Math.Round(score, 3),
            rect = rect,
            outer_rect = rect,
            inner_rect = inner,
            center = center,
            click_point = center
        };
    }

    static CodexDabVisualCandidate ChallengeCandidate(CodexDabVisualRect outer, CodexDabVisualCandidate checkbox, double score, string kind) {
        return new CodexDabVisualCandidate {
            kind = kind,
            score = Math.Round(score, 3),
            rect = outer,
            outer_rect = outer,
            inner_rect = checkbox.rect,
            checkbox_rect = checkbox.rect,
            checkbox_inner_rect = checkbox.inner_rect,
            center = new CodexDabVisualPoint { x = (outer.left + outer.right) / 2, y = (outer.top + outer.bottom) / 2 },
            click_point = checkbox.center
        };
    }

    static double? Score(string kind, int width, int height, int edgeCount) {
        if (width < 8 || height < 8) return null;
        double aspect = width / (double)Math.Max(1, height);
        double perimeter = Math.Max(1, 2 * (width + height));
        double edgeDensity = Math.Min(1.0, edgeCount / perimeter);
        if (kind == "checkbox") {
            if (10 <= width && width <= 80 && 10 <= height && height <= 80 && 0.70 <= aspect && aspect <= 1.35) {
                return 0.65 + 0.25 * (1.0 - Math.Min(Math.Abs(width - height) / (double)Math.Max(width, height), 1.0)) + 0.10 * edgeDensity;
            }
            return null;
        }
        if (kind == "input") {
            if (width >= 60 && 14 <= height && height <= 80 && aspect >= 2.0) return Math.Min(1.0, 0.50 + aspect / 10.0 + 0.10 * edgeDensity);
            return null;
        }
        if (kind == "button") {
            if (width >= 40 && 16 <= height && height <= 90 && aspect >= 1.2) return Math.Min(1.0, 0.45 + aspect / 8.0 + 0.10 * edgeDensity);
            return null;
        }
        if (width >= 8 && height >= 8) return Math.Min(1.0, 0.35 + Math.Min(width * height, 50000) / 100000.0 + 0.15 * edgeDensity);
        return null;
    }

    static List<Component> Components(Bitmap bitmap) {
        int width = bitmap.Width;
        int height = bitmap.Height;
        var rect = new Rectangle(0, 0, width, height);
        using (var clone = bitmap.Clone(rect, PixelFormat.Format32bppArgb)) {
            var data = clone.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try {
                int stride = Math.Abs(data.Stride);
                byte[] bytes = new byte[stride * height];
                Marshal.Copy(data.Scan0, bytes, 0, bytes.Length);
                int[] gray = new int[width * height];
                for (int y = 0; y < height; y++) {
                    int row = y * stride;
                    int dst = y * width;
                    for (int x = 0; x < width; x++) {
                        int b = bytes[row + x * 4];
                        int g = bytes[row + x * 4 + 1];
                        int r = bytes[row + x * 4 + 2];
                        gray[dst + x] = (r * 30 + g * 59 + b * 11) / 100;
                    }
                }
                bool[] mask = new bool[width * height];
                for (int y = 1; y < height - 1; y++) {
                    int row = y * width;
                    for (int x = 1; x < width - 1; x++) {
                        int idx = row + x;
                        int value = gray[idx];
                        if (value < 95 || Math.Abs(value - gray[idx + 1]) > 45 || Math.Abs(value - gray[idx + width]) > 45) mask[idx] = true;
                    }
                }
                bool[] seen = new bool[width * height];
                var components = new List<Component>();
                var stack = new Stack<int>();
                for (int y = 0; y < height; y++) {
                    for (int x = 0; x < width; x++) {
                        int start = y * width + x;
                        if (!mask[start] || seen[start]) continue;
                        stack.Clear();
                        stack.Push(start);
                        seen[start] = true;
                        int minX = x, maxX = x, minY = y, maxY = y, count = 0;
                        while (stack.Count > 0) {
                            int idx = stack.Pop();
                            count++;
                            int cy = idx / width;
                            int cx = idx - cy * width;
                            if (cx < minX) minX = cx;
                            if (cx > maxX) maxX = cx;
                            if (cy < minY) minY = cy;
                            if (cy > maxY) maxY = cy;
                            for (int ny = cy - 1; ny <= cy + 1; ny++) {
                                if (ny < 0 || ny >= height) continue;
                                int baseIdx = ny * width;
                                for (int nx = cx - 1; nx <= cx + 1; nx++) {
                                    if (nx < 0 || nx >= width) continue;
                                    int nidx = baseIdx + nx;
                                    if (mask[nidx] && !seen[nidx]) {
                                        seen[nidx] = true;
                                        stack.Push(nidx);
                                    }
                                }
                            }
                        }
                        if (count >= 16) components.Add(new Component { Left = minX, Top = minY, Right = maxX + 1, Bottom = maxY + 1, Count = count });
                    }
                }
                return components;
            } finally {
                clone.UnlockBits(data);
            }
        }
    }

    static bool IsChallenge(string kind) {
        return kind == "captcha" || kind == "capture" || kind == "challenge" || kind == "turnstile";
    }

    static CodexDabVisualCandidate[] LocateChallengeFast(Bitmap bitmap, string kind, int offsetX, int offsetY) {
        int width = bitmap.Width;
        int height = bitmap.Height;
        var rect = new Rectangle(0, 0, width, height);
        using (var clone = bitmap.Clone(rect, PixelFormat.Format32bppArgb)) {
            var data = clone.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try {
                int stride = Math.Abs(data.Stride);
                byte[] bytes = new byte[stride * height];
                Marshal.Copy(data.Scan0, bytes, 0, bytes.Length);
                int minX = width, minY = height, maxX = -1, maxY = -1, darkCount = 0;
                for (int y = 0; y < height; y++) {
                    int row = y * stride;
                    for (int x = 0; x < width; x++) {
                        int b = bytes[row + x * 4];
                        int g = bytes[row + x * 4 + 1];
                        int r = bytes[row + x * 4 + 2];
                        int gray = (r * 30 + g * 59 + b * 11) / 100;
                        if (gray >= 120) continue;
                        if (x < minX) minX = x;
                        if (y < minY) minY = y;
                        if (x > maxX) maxX = x;
                        if (y > maxY) maxY = y;
                        darkCount++;
                    }
                }
                if (darkCount < 20 || maxX < minX || maxY < minY) return new CodexDabVisualCandidate[0];

                int cbMinX = width, cbMinY = height, cbMaxX = -1, cbMaxY = -1, cbCount = 0;
                int searchLeft = Math.Min(width - 1, minX + 6);
                int searchRight = Math.Min(maxX - 6, minX + Math.Max(70, (maxX - minX) / 3));
                int searchTop = Math.Min(height - 1, minY + 6);
                int searchBottom = Math.Max(searchTop, maxY - 6);
                for (int y = searchTop; y <= searchBottom; y++) {
                    int row = y * stride;
                    for (int x = searchLeft; x <= searchRight; x++) {
                        int b = bytes[row + x * 4];
                        int g = bytes[row + x * 4 + 1];
                        int r = bytes[row + x * 4 + 2];
                        int gray = (r * 30 + g * 59 + b * 11) / 100;
                        if (gray >= 130) continue;
                        if (x < cbMinX) cbMinX = x;
                        if (y < cbMinY) cbMinY = y;
                        if (x > cbMaxX) cbMaxX = x;
                        if (y > cbMaxY) cbMaxY = y;
                        cbCount++;
                    }
                }
                int cbWidth = cbMaxX - cbMinX + 1;
                int cbHeight = cbMaxY - cbMinY + 1;
                if (cbCount < 8 || cbWidth < 10 || cbHeight < 10 || cbWidth > 80 || cbHeight > 80) {
                    return new CodexDabVisualCandidate[0];
                }

                var outer = Rect(offsetX + minX, offsetY + minY, offsetX + maxX + 1, offsetY + maxY + 1);
                var checkbox = Candidate(cbMinX, cbMinY, cbMaxX + 1, cbMaxY + 1, 1.0, "checkbox", offsetX, offsetY);
                return new [] { ChallengeCandidate(outer, checkbox, 1.0, kind) };
            } finally {
                clone.UnlockBits(data);
            }
        }
    }

    public static CodexDabVisualCandidate[] Locate(string path, string kind, int maxCandidates, int roiX, int roiY, int roiW, int roiH) {
        var stopwatch = Stopwatch.StartNew();
        try {
            return LocateCore(path, kind, maxCandidates, roiX, roiY, roiW, roiH);
        } finally {
            stopwatch.Stop();
            LastElapsedMs = stopwatch.Elapsed.TotalMilliseconds;
        }
    }

    static CodexDabVisualCandidate[] LocateCore(string path, string kind, int maxCandidates, int roiX, int roiY, int roiW, int roiH) {
        kind = String.IsNullOrWhiteSpace(kind) ? "generic_box" : kind.Trim().ToLowerInvariant();
        maxCandidates = Math.Max(1, Math.Min(50, maxCandidates));
        using (var original = new Bitmap(path)) {
            var crop = new Rectangle(Math.Max(0, roiX), Math.Max(0, roiY), roiW > 0 ? roiW : original.Width, roiH > 0 ? roiH : original.Height);
            crop.Width = Math.Max(1, Math.Min(crop.Width, original.Width - crop.X));
            crop.Height = Math.Max(1, Math.Min(crop.Height, original.Height - crop.Y));
            using (var work = original.Clone(crop, PixelFormat.Format32bppArgb)) {
                if (IsChallenge(kind)) {
                    var fast = LocateChallengeFast(work, kind, crop.X, crop.Y);
                    if (fast.Length > 0) return fast.Take(maxCandidates).ToArray();
                }
                var checkboxes = new List<CodexDabVisualCandidate>();
                var boxes = new List<CodexDabVisualCandidate>();
                var candidates = new List<CodexDabVisualCandidate>();
                foreach (var component in Components(work)) {
                    int width = component.Right - component.Left;
                    int height = component.Bottom - component.Top;
                    if (IsChallenge(kind)) {
                        var cbScore = Score("checkbox", width, height, component.Count);
                        if (cbScore.HasValue) checkboxes.Add(Candidate(component.Left, component.Top, component.Right, component.Bottom, cbScore.Value, "checkbox", crop.X, crop.Y));
                        var boxScore = Score("generic_box", width, height, component.Count);
                        if (boxScore.HasValue) boxes.Add(Candidate(component.Left, component.Top, component.Right, component.Bottom, boxScore.Value, "generic_box", crop.X, crop.Y));
                    } else {
                        var score = Score(kind, width, height, component.Count);
                        if (score.HasValue) candidates.Add(Candidate(component.Left, component.Top, component.Right, component.Bottom, score.Value, kind, crop.X, crop.Y));
                    }
                }
                if (IsChallenge(kind)) {
                    foreach (var checkbox in checkboxes.OrderByDescending(c => c.score).Take(25)) {
                        int cbArea = Math.Max(1, Area(checkbox.rect));
                        var outer = boxes
                            .Where(b => Area(b.rect) >= cbArea * 3 && Contains(b.rect, checkbox.rect, 2))
                            .OrderBy(b => Area(b.rect))
                            .Select(b => b.rect)
                            .FirstOrDefault();
                        if (outer == null) outer = Clip(Rect(checkbox.rect.left - 16, checkbox.rect.top - 16, checkbox.rect.right + 260, checkbox.rect.bottom + 34), original.Width, original.Height);
                        double areaBonus = Math.Min(0.12, Area(outer) / (double)Math.Max(1, original.Width * original.Height) * 0.30);
                        candidates.Add(ChallengeCandidate(outer, checkbox, Math.Min(1.0, checkbox.score + areaBonus), kind));
                    }
                }
                return candidates.OrderByDescending(c => c.score).Take(maxCandidates).ToArray();
            }
        }
    }
}
'@

function Write-DabResult($obj) {
    $obj | ConvertTo-Json -Depth 12 -Compress
}

function Get-Hwnd($value) {
    if ($null -eq $value) { return [IntPtr]::Zero }
    if ($value -is [string]) {
        $text = $value.Trim()
        if ($text.StartsWith('0x')) {
            return [IntPtr]::new([Convert]::ToInt64($text.Substring(2), 16))
        }
        return [IntPtr]::new([Int64]::Parse($text))
    }
    [IntPtr]::new([Int64]$value)
}

function Get-WindowTitle([IntPtr]$hwnd) {
    $len = [CodexDabNative]::GetWindowTextLength($hwnd)
    $sb = New-Object Text.StringBuilder ([Math]::Max($len + 1, 256))
    [void][CodexDabNative]::GetWindowText($hwnd, $sb, $sb.Capacity)
    $sb.ToString()
}

function Get-VirtualScreenRect {
    [pscustomobject]@{
        x = [CodexDabNative]::GetSystemMetrics(76)
        y = [CodexDabNative]::GetSystemMetrics(77)
        width = [CodexDabNative]::GetSystemMetrics(78)
        height = [CodexDabNative]::GetSystemMetrics(79)
    }
}

function Test-RectOffScreen($rect) {
    if ($null -eq $rect -or $rect.width -le 0 -or $rect.height -le 0) { return $true }
    $screen = Get-VirtualScreenRect
    $right = [int]$rect.x + [int]$rect.width
    $bottom = [int]$rect.y + [int]$rect.height
    $screenRight = [int]$screen.x + [int]$screen.width
    $screenBottom = [int]$screen.y + [int]$screen.height
    return ($right -le $screen.x -or $rect.x -ge $screenRight -or $bottom -le $screen.y -or $rect.y -ge $screenBottom)
}

function Get-WindowInfo([IntPtr]$hwnd) {
    $rect = New-Object CodexDabNative+RECT
    [void][CodexDabNative]::GetWindowRect($hwnd, [ref]$rect)
    $rectObj = [pscustomobject]@{
        x = $rect.Left
        y = $rect.Top
        width = $rect.Right - $rect.Left
        height = $rect.Bottom - $rect.Top
    }
    $processIdValue = [uint32]0
    [void][CodexDabNative]::GetWindowThreadProcessId($hwnd, [ref]$processIdValue)
    $processName = $null
    try { $processName = (Get-Process -Id $processIdValue -ErrorAction Stop).ProcessName } catch {}
    [pscustomobject]@{
        hwnd = ('0x{0:x}' -f $hwnd.ToInt64())
        hwnd_decimal = $hwnd.ToInt64()
        title = Get-WindowTitle $hwnd
        process_id = [int]$processIdValue
        process_name = $processName
        visible = [CodexDabNative]::IsWindowVisible($hwnd)
        is_minimized = [CodexDabNative]::IsIconic($hwnd)
        is_foreground = ([CodexDabNative]::GetForegroundWindow() -eq $hwnd)
        is_offscreen = Test-RectOffScreen $rectObj
        virtual_screen = Get-VirtualScreenRect
        rect = $rectObj
    }
}

function Find-Windows($argsObj) {
    $title = [string]$argsObj.title
    $process = [string]$argsObj.process
    $includeHidden = [bool]$argsObj.include_hidden
    $limit = if ($argsObj.limit) { [int]$argsObj.limit } else { 30 }
    $script:CodexDabWindows = New-Object System.Collections.Generic.List[object]
    $callback = [CodexDabNative+EnumWindowsProc]{
        param([IntPtr]$hwnd, [IntPtr]$lparam)
        if (-not [CodexDabNative]::IsWindow($hwnd)) { return $true }
        $info = Get-WindowInfo $hwnd
        if (-not $includeHidden -and -not $info.visible) { return $true }
        if ([string]::IsNullOrWhiteSpace($info.title) -and [string]::IsNullOrWhiteSpace($info.process_name)) { return $true }
        if (-not [string]::IsNullOrWhiteSpace($title) -and $info.title.IndexOf($title, [StringComparison]::OrdinalIgnoreCase) -lt 0) { return $true }
        if (-not [string]::IsNullOrWhiteSpace($process) -and ($info.process_name -eq $null -or $info.process_name.IndexOf($process, [StringComparison]::OrdinalIgnoreCase) -lt 0)) { return $true }
        $script:CodexDabWindows.Add($info)
        return $script:CodexDabWindows.Count -lt $limit
    }
    [void][CodexDabNative]::EnumWindows($callback, [IntPtr]::Zero)
    $script:CodexDabWindows.ToArray()
}

function Resolve-TargetWindow($argsObj) {
    $hwnd = Get-Hwnd $argsObj.hwnd
    if ($hwnd -ne [IntPtr]::Zero -and [CodexDabNative]::IsWindow($hwnd)) {
        return Get-WindowInfo $hwnd
    }
    $matches = Find-Windows $argsObj
    if ($matches.Count -gt 0) { return $matches[0] }
    $null
}

function Test-HasTarget($argsObj) {
    if ($null -eq $argsObj) { return $false }
    foreach ($name in @('hwnd', 'title', 'process')) {
        $property = $argsObj.PSObject.Properties[$name]
        if ($null -ne $property -and $null -ne $property.Value -and -not [string]::IsNullOrWhiteSpace([string]$property.Value)) {
            return $true
        }
    }
    $false
}

function Get-ArgBool($argsObj, [string]$name, [bool]$defaultValue) {
    $property = $argsObj.PSObject.Properties[$name]
    if ($null -eq $property -or $null -eq $property.Value) { return $defaultValue }
    return [bool]$property.Value
}

function Get-ArgInt($argsObj, [string]$name, [int]$defaultValue) {
    $property = $argsObj.PSObject.Properties[$name]
    if ($null -eq $property -or $null -eq $property.Value) { return $defaultValue }
    try { return [int]$property.Value } catch { return $defaultValue }
}

function Prepare-Window($argsObj) {
    $window = Resolve-TargetWindow $argsObj
    if ($null -eq $window) {
        return [pscustomobject]@{ ok = $false; error = 'target window not found' }
    }

    $hwnd = Get-Hwnd $window.hwnd
    $before = Get-WindowInfo $hwnd
    $screen = Get-VirtualScreenRect
    $targetX = Get-ArgInt $argsObj 'x' ([Math]::Max($screen.x + 40, 40))
    $targetY = Get-ArgInt $argsObj 'y' ([Math]::Max($screen.y + 40, 40))
    $targetW = Get-ArgInt $argsObj 'width' ([Math]::Min(1280, [Math]::Max(320, $screen.width - 120)))
    $targetH = Get-ArgInt $argsObj 'height' ([Math]::Min(900, [Math]::Max(240, $screen.height - 120)))
    $focus = Get-ArgBool $argsObj 'focus' $true

    $restored = $false
    if ($before.is_minimized) {
        [void][CodexDabNative]::ShowWindowAsync($hwnd, 9)
        Start-Sleep -Milliseconds 150
        $restored = $true
    }

    $current = Get-WindowInfo $hwnd
    $tooSmall = $current.rect.width -lt 160 -or $current.rect.height -lt 120
    $moved = $false
    if ($current.is_offscreen -or $tooSmall) {
        [void][CodexDabNative]::MoveWindow($hwnd, $targetX, $targetY, $targetW, $targetH, $true)
        Start-Sleep -Milliseconds 150
        $moved = $true
    }

    $focused = $false
    if ($focus) {
        [void][CodexDabNative]::ShowWindowAsync($hwnd, 5)
        [void][CodexDabNative]::SetForegroundWindow($hwnd)
        Start-Sleep -Milliseconds 120
        $focused = $true
    }

    [pscustomobject]@{
        ok = $true
        before = $before
        after = Get-WindowInfo $hwnd
        restored = $restored
        moved = $moved
        focused = $focused
        target_rect = [pscustomobject]@{ x = $targetX; y = $targetY; width = $targetW; height = $targetH }
    }
}

function Get-MissingNumericFields($argsObj, [string[]]$names) {
    $missing = @()
    foreach ($name in $names) {
        $property = $argsObj.PSObject.Properties[$name]
        if ($null -eq $property -or $null -eq $property.Value) {
            $missing += $name
            continue
        }
        try {
            $number = [double]$property.Value
            if ([double]::IsNaN($number) -or [double]::IsInfinity($number)) {
                $missing += $name
            }
        } catch {
            $missing += $name
        }
    }
    $missing
}

function Get-Elements($hwnd, $maxElements) {
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $root = [System.Windows.Automation.AutomationElement]::FromHandle((Get-Hwnd $hwnd))
    if ($null -eq $root) { return @() }
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $items = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $all.Count -and $items.Count -lt $maxElements; $i++) {
        $el = $all.Item($i)
        $name = $el.Current.Name
        $automationId = $el.Current.AutomationId
        $rect = $el.Current.BoundingRectangle
        if ([string]::IsNullOrWhiteSpace($name) -and [string]::IsNullOrWhiteSpace($automationId)) { continue }
        $items.Add([pscustomobject]@{
            index = $items.Count
            name = $name
            automation_id = $automationId
            control_type = $el.Current.ControlType.ProgrammaticName
            enabled = $el.Current.IsEnabled
            rect = [pscustomobject]@{
                x = Convert-UiRectNumber $rect.X
                y = Convert-UiRectNumber $rect.Y
                width = Convert-UiRectNumber $rect.Width
                height = Convert-UiRectNumber $rect.Height
            }
        })
    }
    $items.ToArray()
}

function Convert-UiRectNumber($value) {
    try {
        $number = [double]$value
        if ([double]::IsNaN($number) -or [double]::IsInfinity($number)) { return 0 }
        if ($number -gt [int]::MaxValue -or $number -lt [int]::MinValue) { return 0 }
        return [int][Math]::Round($number)
    } catch {
        return 0
    }
}

function Get-VisibleText($elements) {
    $seen = New-Object System.Collections.Generic.HashSet[string]
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($element in $elements) {
        $text = [string]$element.name
        if ([string]::IsNullOrWhiteSpace($text)) { continue }
        $text = $text.Trim()
        if ($seen.Add($text)) {
            $lines.Add($text)
        }
    }
    [string]::Join("`n", $lines.ToArray())
}

function Get-NavigationKeys($destination) {
    $name = ([string]$destination).Trim()
    if ([string]::IsNullOrWhiteSpace($name)) {
        throw 'destination is required'
    }
    switch -Regex ($name.ToLowerInvariant()) {
        '^(command_palette|palette)$' { return '^+p' }
        '^(find|search)$' { return '^f' }
        '^(save)$' { return '^s' }
        '^(open)$' { return '^o' }
        '^(new_tab|browser_new_tab)$' { return '^t' }
        '^(close_tab|browser_close_tab)$' { return '^w' }
        '^(next_tab|tab_next|terminal_next_tab)$' { return '^{TAB}' }
        '^(previous_tab|prev_tab|tab_previous|terminal_previous_tab)$' { return '^+{TAB}' }
        '^(terminal_new_tab|wt_new_tab)$' { return '^+t' }
        '^(terminal_close_tab|wt_close_tab)$' { return '^+w' }
        '^(address_bar|url)$' { return '^l' }
        '^(terminal)$' { return '^`' }
        '^(copy)$' { return '^c' }
        '^(paste)$' { return '^v' }
        '^(select_all)$' { return '^a' }
        '^(codex_submit|claude_submit)$' { return '{ENTER}' }
        '^(codex_interrupt|claude_interrupt)$' { return '^c' }
        '^(codex_newline|claude_newline)$' { return '+{ENTER}' }
        '^(codex_paste|claude_paste)$' { return '^v' }
        '^(next|tab)$' { return '{TAB}' }
        '^(previous|prev|back)$' { return '+{TAB}' }
        '^(confirm|enter|submit)$' { return '{ENTER}' }
        '^(cancel|escape|esc)$' { return '{ESC}' }
        '^(up)$' { return '{UP}' }
        '^(down)$' { return '{DOWN}' }
        '^(left)$' { return '{LEFT}' }
        '^(right)$' { return '{RIGHT}' }
        default { return $name }
    }
}

function Save-Screenshot($window, $argsObj) {
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing
    if ($null -eq $window) {
        $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    } else {
        $bounds = New-Object Drawing.Rectangle($window.rect.x, $window.rect.y, $window.rect.width, $window.rect.height)
    }
    if ($bounds.Width -le 0 -or $bounds.Height -le 0) {
        throw 'target window has an empty rectangle'
    }
    $path = [string]$argsObj.path
    if ([string]::IsNullOrWhiteSpace($path)) {
        $path = [IO.Path]::Combine([IO.Path]::GetTempPath(), ('codex-dab-{0}.png' -f ([Guid]::NewGuid().ToString('n'))))
    }
    $dir = [IO.Path]::GetDirectoryName($path)
    if (-not [string]::IsNullOrWhiteSpace($dir)) { [IO.Directory]::CreateDirectory($dir) | Out-Null }
    $bmp = New-Object Drawing.Bitmap($bounds.Width, $bounds.Height)
    $graphics = [Drawing.Graphics]::FromImage($bmp)
    try {
        $graphics.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bmp.Size)
        $bmp.Save($path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bmp.Dispose()
    }
    $embedImage = $true
    $embedProperty = $argsObj.PSObject.Properties['embed_image']
    if ($null -ne $embedProperty) { $embedImage = [bool]$embedProperty.Value }
    $imageUrl = $null
    if ($embedImage) {
        $bytes = [IO.File]::ReadAllBytes($path)
        $imageUrl = 'data:image/png;base64,' + [Convert]::ToBase64String($bytes)
    }
    [pscustomobject]@{
        path = $path
        image_url = $imageUrl
        width = $bounds.Width
        height = $bounds.Height
    }
}

function Convert-VisualRoi($roi) {
    if ($null -eq $roi) { return @(0, 0, 0, 0) }
    $text = ([string]$roi).Trim()
    if ([string]::IsNullOrWhiteSpace($text)) { return @(0, 0, 0, 0) }
    try {
        if ($text.StartsWith('{')) {
            $obj = $text | ConvertFrom-Json
            if ($obj.PSObject.Properties['left'] -and $obj.PSObject.Properties['top'] -and $obj.PSObject.Properties['right'] -and $obj.PSObject.Properties['bottom']) {
                $left = [int][double]$obj.left
                $top = [int][double]$obj.top
                return @($left, $top, [int]([double]$obj.right - $left), [int]([double]$obj.bottom - $top))
            }
            $vx = if ($obj.PSObject.Properties['x']) { $obj.x } elseif ($obj.PSObject.Properties['left']) { $obj.left } else { 0 }
            $vy = if ($obj.PSObject.Properties['y']) { $obj.y } elseif ($obj.PSObject.Properties['top']) { $obj.top } else { 0 }
            $vw = if ($obj.PSObject.Properties['w']) { $obj.w } elseif ($obj.PSObject.Properties['width']) { $obj.width } else { 0 }
            $vh = if ($obj.PSObject.Properties['h']) { $obj.h } elseif ($obj.PSObject.Properties['height']) { $obj.height } else { 0 }
            return @(
                [int][double]$vx,
                [int][double]$vy,
                [int][double]$vw,
                [int][double]$vh
            )
        }
        $parts = @($text -split '[,x ]+' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { [int][double]$_ })
        if ($parts.Count -eq 4) { return @($parts[0], $parts[1], $parts[2], $parts[3]) }
    } catch {}
    @(0, 0, 0, 0)
}

function Add-VisualScreenFields($candidates, $window) {
    if ($null -eq $window -or $null -eq $window.rect) { return @($candidates) }
    foreach ($candidate in @($candidates)) {
        if ($null -eq $candidate.rect) { continue }
        $screenRect = [pscustomobject]@{
            left = [int]$window.rect.x + [int]$candidate.rect.left
            top = [int]$window.rect.y + [int]$candidate.rect.top
            right = [int]$window.rect.x + [int]$candidate.rect.right
            bottom = [int]$window.rect.y + [int]$candidate.rect.bottom
            width = [int]$candidate.rect.width
            height = [int]$candidate.rect.height
        }
        $candidate | Add-Member -NotePropertyName screen_rect -NotePropertyValue $screenRect -Force
        if ($null -ne $candidate.click_point) {
            $candidate | Add-Member -NotePropertyName screen_click_point -NotePropertyValue ([pscustomobject]@{
                x = [int]$window.rect.x + [int]$candidate.click_point.x
                y = [int]$window.rect.y + [int]$candidate.click_point.y
            }) -Force
        }
    }
    @($candidates)
}

function Invoke-VisualLocate($argsObj) {
    $kind = if ($argsObj.kind) { [string]$argsObj.kind } else { 'generic_box' }
    $max = Get-ArgInt $argsObj 'max_candidates' 5
    $path = [string]$argsObj.path
    $window = $null
    $source = $null
    if ([string]::IsNullOrWhiteSpace($path)) {
        $hasTarget = Test-HasTarget $argsObj
        if (-not $hasTarget) { return [pscustomobject]@{ ok = $false; error = 'path or target window is required' } }
        $window = Resolve-TargetWindow $argsObj
        if ($null -eq $window) { return [pscustomobject]@{ ok = $false; error = 'target window not found' } }
        $shotArgs = [pscustomobject]@{ embed_image = $false }
        $shot = Save-Screenshot $window $shotArgs
        $path = $shot.path
        $source = [pscustomobject]@{ type = 'window'; window = $window; screenshot = $shot }
    } else {
        $path = [IO.Path]::GetFullPath($path)
        $source = [pscustomobject]@{ type = 'image'; path = $path }
    }
    $roi = Convert-VisualRoi $argsObj.roi
    $candidates = [CodexDabVision]::Locate($path, $kind, $max, $roi[0], $roi[1], $roi[2], $roi[3])
    $candidates = Add-VisualScreenFields $candidates $window
    $image = [Drawing.Image]::FromFile($path)
    try {
        [pscustomobject]@{
            ok = $true
            kind = $kind
            source = $source
            coordinate_space = if ($window) { 'window' } else { 'image' }
            size = [pscustomobject]@{ width = $image.Width; height = $image.Height }
            roi = if (($roi | Measure-Object -Sum).Sum -eq 0) { $null } else { $roi }
            elapsed_ms = [Math]::Round([CodexDabVision]::LastElapsedMs, 3)
            candidates = @($candidates)
        }
    } finally {
        $image.Dispose()
    }
}

function Invoke-ForegroundClick($x, $y) {
    [void][CodexDabNative]::SetCursorPos([int]$x, [int]$y)
    [CodexDabNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [CodexDabNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
}

function Invoke-OrganicClick($x, $y) {
    $targetX = [int]$x
    $targetY = [int]$y

    $startPt = New-Object CodexDabNative+POINT
    [void][CodexDabNative]::GetCursorPos([ref]$startPt)

    $sx = $startPt.X
    $sy = $startPt.Y

    $steps = Get-Random -Minimum 10 -Maximum 20
    $cxDir = (Get-Random -Minimum 0 -Maximum 2) * 2 - 1
    $cyDir = (Get-Random -Minimum 0 -Maximum 2) * 2 - 1
    $amp = Get-Random -Minimum 10 -Maximum 40

    for ($i = 0; $i -lt $steps; $i++) {
        $t = $i / ($steps - 1)
        $easeT = 1.0 - [Math]::Pow(1.0 - $t, 3.0)
        $curveAmp = [Math]::Sin($t * [Math]::PI) * $amp

        $curX = [int]($sx + ($targetX - $sx) * $easeT + ($curveAmp * $cxDir))
        $curY = [int]($sy + ($targetY - $sy) * $easeT + ($curveAmp * $cyDir))

        [void][CodexDabNative]::SetCursorPos($curX, $curY)
        Start-Sleep -Milliseconds (Get-Random -Minimum 10 -Maximum 30)
    }

    [void][CodexDabNative]::SetCursorPos($targetX, $targetY)
    Start-Sleep -Milliseconds (Get-Random -Minimum 100 -Maximum 300)

    [CodexDabNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds (Get-Random -Minimum 70 -Maximum 150)
    [CodexDabNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
}

function Invoke-ForegroundDrag($startX, $startY, $endX, $endY, $durationMs, $steps) {
    $stepCount = if ($steps) { [Math]::Max(1, [int]$steps) } else { 16 }
    $totalMs = if ($durationMs) { [Math]::Max(0, [int]$durationMs) } else { 350 }
    $sleepMs = if ($stepCount -gt 0) { [Math]::Max(1, [int]($totalMs / $stepCount)) } else { 1 }
    $sx = [double]$startX
    $sy = [double]$startY
    $ex = [double]$endX
    $ey = [double]$endY

    [void][CodexDabNative]::SetCursorPos([int]$sx, [int]$sy)
    Start-Sleep -Milliseconds 40
    [CodexDabNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    try {
        for ($i = 1; $i -le $stepCount; $i++) {
            $ratio = $i / [double]$stepCount
            $x = [int][Math]::Round($sx + (($ex - $sx) * $ratio))
            $y = [int][Math]::Round($sy + (($ey - $sy) * $ratio))
            [void][CodexDabNative]::SetCursorPos($x, $y)
            Start-Sleep -Milliseconds $sleepMs
        }
    } finally {
        [CodexDabNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    }
}

function Invoke-ForegroundScroll($x, $y, $amount) {
    if ($null -ne $x -and $null -ne $y) {
        [void][CodexDabNative]::SetCursorPos([int]$x, [int]$y)
        Start-Sleep -Milliseconds 40
    }
    $delta = if ($amount) { [int]$amount } else { -120 }
    [CodexDabNative]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero)
}

function Invoke-BackgroundClick($window, $x, $y) {
    $point = New-Object CodexDabNative+POINT
    $point.X = [int]$x
    $point.Y = [int]$y
    [void][CodexDabNative]::ScreenToClient((Get-Hwnd $window.hwnd), [ref]$point)
    $lparam = [IntPtr](($point.X -band 0xffff) -bor (($point.Y -band 0xffff) -shl 16))
    [void][CodexDabNative]::PostMessage((Get-Hwnd $window.hwnd), 0x0201, [IntPtr]1, $lparam)
    Start-Sleep -Milliseconds 40
    [void][CodexDabNative]::PostMessage((Get-Hwnd $window.hwnd), 0x0202, [IntPtr]0, $lparam)
}

function Resolve-TerminalWindow($argsObj) {
    if (Test-HasTarget $argsObj) { return Resolve-TargetWindow $argsObj }
    foreach ($processName in @('WindowsTerminal', 'wt', 'pwsh', 'powershell', 'cmd')) {
        $candidateArgs = [pscustomobject]@{ process = $processName; limit = 1 }
        $window = Resolve-TargetWindow $candidateArgs
        if ($null -ne $window) { return $window }
    }
    $null
}

function Get-TerminalTabs($window, $maxElements) {
    $elements = Get-Elements $window.hwnd $maxElements
    $tabs = New-Object System.Collections.Generic.List[object]
    foreach ($element in $elements) {
        $controlType = [string]$element.control_type
        $name = [string]$element.name
        if ([string]::IsNullOrWhiteSpace($name)) { continue }
        $looksLikeTab = $controlType.IndexOf('TabItem', [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
            $controlType.IndexOf('ListItem', [StringComparison]::OrdinalIgnoreCase) -ge 0 -and
            $element.rect.y -le ($window.rect.y + 120)
        if (-not $looksLikeTab) { continue }
        $tabs.Add([pscustomobject]@{
            index = $tabs.Count
            name = $name
            automation_id = $element.automation_id
            control_type = $element.control_type
            rect = $element.rect
        })
    }
    [pscustomobject]@{
        tabs = $tabs.ToArray()
        text = Get-VisibleText $elements
        elements = $elements
    }
}

try {
    switch ($tool) {
        'dab_find_window' {
            Write-DabResult ([pscustomobject]@{ ok = $true; windows = @(Find-Windows $ArgsObj) })
        }
        'dab_window_check' {
            $window = Resolve-TargetWindow $ArgsObj
            Write-DabResult ([pscustomobject]@{ ok = ($null -ne $window); window = $window })
        }
        'dab_prepare_window' {
            Write-DabResult (Prepare-Window $ArgsObj)
        }
        'dab_screenshot' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $shot = Save-Screenshot $window $ArgsObj
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; screenshot = $shot; image_url = $shot.image_url })
        }
        'dab_locate_visual' {
            Write-DabResult (Invoke-VisualLocate $ArgsObj)
        }
        'dab_element_map' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 80 }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; elements = @(Get-Elements $window.hwnd $max) })
        }
        'dab_ocr' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; text = ''; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 120 }
            $elements = @(Get-Elements $window.hwnd $max)
            $shot = if ($ArgsObj.screenshot -eq $false) { $null } else { Save-Screenshot $window $ArgsObj }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; text = Get-VisibleText $elements; elements = $elements; screenshot = $shot; image_url = if ($shot) { $shot.image_url } else { $null } })
        }
        'dab_visual_scan' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 80 }
            $shot = if ($ArgsObj.screenshot -eq $false) { $null } else { Save-Screenshot $window $ArgsObj }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; elements = @(Get-Elements $window.hwnd $max); screenshot = $shot; image_url = if ($shot) { $shot.image_url } else { $null } })
        }
        'dab_click' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $missing = @(Get-MissingNumericFields $ArgsObj @('x', 'y'))
            if ($missing.Count -gt 0) { Write-DabResult ([pscustomobject]@{ ok = $false; error = "missing or invalid numeric click fields: $($missing -join ', ')" }); break }
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 80 }
            Invoke-ForegroundClick $ArgsObj.x $ArgsObj.y
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; clicked = @{ x = [int]$ArgsObj.x; y = [int]$ArgsObj.y; mode = 'foreground' } })
        }
        'dab_drag' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $missing = @(Get-MissingNumericFields $ArgsObj @('x', 'y', 'end_x', 'end_y'))
            if ($missing.Count -gt 0) { Write-DabResult ([pscustomobject]@{ ok = $false; error = "missing or invalid numeric drag fields: $($missing -join ', ')" }); break }
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 100 }
            Invoke-ForegroundDrag $ArgsObj.x $ArgsObj.y $ArgsObj.end_x $ArgsObj.end_y $ArgsObj.duration_ms $ArgsObj.steps
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; drag = @{ start = @{ x = [int]$ArgsObj.x; y = [int]$ArgsObj.y }; end = @{ x = [int]$ArgsObj.end_x; y = [int]$ArgsObj.end_y }; mode = 'foreground' } })
        }
        'dab_scroll' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 80 }
            Invoke-ForegroundScroll $ArgsObj.x $ArgsObj.y $ArgsObj.amount
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; scroll = @{ x = if ($ArgsObj.x) { [int]$ArgsObj.x } else { $null }; y = if ($ArgsObj.y) { [int]$ArgsObj.y } else { $null }; amount = if ($ArgsObj.amount) { [int]$ArgsObj.amount } else { -120 }; mode = 'foreground' } })
        }
        'dab_bg_click' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $missing = @(Get-MissingNumericFields $ArgsObj @('x', 'y'))
            if ($missing.Count -gt 0) { Write-DabResult ([pscustomobject]@{ ok = $false; error = "missing or invalid numeric click fields: $($missing -join ', ')" }); break }
            Invoke-BackgroundClick $window $ArgsObj.x $ArgsObj.y
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; clicked = @{ x = [int]$ArgsObj.x; y = [int]$ArgsObj.y; mode = 'background' } })
        }
        'dab_smart_click' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $text = [string]$ArgsObj.text
            $elements = Get-Elements $window.hwnd 200
            $match = $elements | Where-Object { $_.name -eq $text -or $_.automation_id -eq $text } | Select-Object -First 1
            if ($null -eq $match) { $match = $elements | Where-Object { $_.name -and $_.name.IndexOf($text, [StringComparison]::OrdinalIgnoreCase) -ge 0 } | Select-Object -First 1 }
            if ($null -eq $match) {
            Write-DabResult ([pscustomobject]@{ ok = $false; error = 'no element matched'; available_elements = @($elements | Select-Object -First 30) })
                break
            }
            $x = [int]($match.rect.x + ($match.rect.width / 2))
            $y = [int]($match.rect.y + ($match.rect.height / 2))
            [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd))
            Start-Sleep -Milliseconds 80
            Invoke-ForegroundClick $x $y
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; matched = $match; clicked = @{ x = $x; y = $y; mode = 'smart' } })
        }
        'dab_send_keys' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 120 }
            $shell = New-Object -ComObject WScript.Shell
            $shell.SendKeys([string]$ArgsObj.keys)
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; sent = @{ keys = [string]$ArgsObj.keys } })
        }
        'dab_navigate' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 120 }
            $keys = Get-NavigationKeys $ArgsObj.destination
            $shell = New-Object -ComObject WScript.Shell
            $shell.SendKeys($keys)
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; navigation = @{ destination = [string]$ArgsObj.destination; keys = $keys } })
        }
        'dab_terminal_tabs' {
            $window = Resolve-TerminalWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target terminal window not found'; tabs = @(); text = '' }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 300 }
            $terminal = Get-TerminalTabs $window $max
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; tabs = @($terminal.tabs); text = $terminal.text })
        }
        'dab_terminal_focus' {
            $window = Resolve-TerminalWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target terminal window not found'; tabs = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 300 }
            $terminal = Get-TerminalTabs $window $max
            $tab = $null
            if ($null -ne $ArgsObj.index) {
                $targetIndex = [int]$ArgsObj.index
                $tab = $terminal.tabs | Where-Object { $_.index -eq $targetIndex } | Select-Object -First 1
            }
            if ($null -eq $tab -and -not [string]::IsNullOrWhiteSpace([string]$ArgsObj.tab_title)) {
                $needle = [string]$ArgsObj.tab_title
                $tab = $terminal.tabs | Where-Object { $_.name -and $_.name.IndexOf($needle, [StringComparison]::OrdinalIgnoreCase) -ge 0 } | Select-Object -First 1
            }
            if ($null -eq $tab -and -not [string]::IsNullOrWhiteSpace([string]$ArgsObj.text)) {
                $needle = [string]$ArgsObj.text
                $tab = $terminal.tabs | Where-Object { $_.name -and $_.name.IndexOf($needle, [StringComparison]::OrdinalIgnoreCase) -ge 0 } | Select-Object -First 1
                if ($null -eq $tab -and $terminal.text.IndexOf($needle, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                    [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd))
                    Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; focused = @{ mode = 'window_content'; text = $needle }; tabs = @($terminal.tabs) })
                    break
                }
            }
            if ($null -eq $tab) {
                [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd))
                Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; focused = @{ mode = 'window' }; tabs = @($terminal.tabs) })
                break
            }
            $x = [int]($tab.rect.x + ($tab.rect.width / 2))
            $y = [int]($tab.rect.y + ($tab.rect.height / 2))
            [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd))
            Start-Sleep -Milliseconds 80
            Invoke-ForegroundClick $x $y
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; focused = @{ mode = 'tab'; tab = $tab; x = $x; y = $y }; tabs = @($terminal.tabs) })
        }
        default {
            Write-DabResult ([pscustomobject]@{ ok = $false; error = "unsupported DAB tool $tool" })
        }
    }
} catch {
    Write-DabResult ([pscustomobject]@{ ok = $false; error = $_.Exception.Message; stack = $_.ScriptStackTrace })
    exit 0
}

