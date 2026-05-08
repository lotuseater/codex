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
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extraInfo);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hWnd, ref POINT point);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
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

function Get-WindowInfo([IntPtr]$hwnd) {
    $rect = New-Object CodexDabNative+RECT
    [void][CodexDabNative]::GetWindowRect($hwnd, [ref]$rect)
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
        rect = [pscustomobject]@{
            x = $rect.Left
            y = $rect.Top
            width = $rect.Right - $rect.Left
            height = $rect.Bottom - $rect.Top
        }
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

function Invoke-ForegroundClick($x, $y) {
    [void][CodexDabNative]::SetCursorPos([int]$x, [int]$y)
    [CodexDabNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
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
        'dab_screenshot' {
            $hasTarget = Test-HasTarget $ArgsObj
            $window = if ($hasTarget) { Resolve-TargetWindow $ArgsObj } else { $null }
            if ($hasTarget -and $null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found' }); break }
            $shot = Save-Screenshot $window $ArgsObj
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; screenshot = $shot; image_url = $shot.image_url })
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

