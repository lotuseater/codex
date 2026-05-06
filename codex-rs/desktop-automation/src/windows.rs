use crate::DesktopAutomationError;
use crate::DesktopAutomationResult;
use serde_json::Value;

#[cfg(not(windows))]
pub async fn execute_dab_tool(
    _tool_name: &str,
    _input: Value,
) -> Result<DesktopAutomationResult, DesktopAutomationError> {
    Err(DesktopAutomationError::Unsupported(
        "native DAB is only available on Windows".to_string(),
    ))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;
    use tokio::time::timeout;

    const BRIDGE_TIMEOUT_SECONDS: u64 = 20;

    pub async fn execute_dab_tool(
        tool_name: &str,
        input: Value,
    ) -> Result<DesktopAutomationResult, DesktopAutomationError> {
        let input_json = serde_json::to_vec(&input)
            .map_err(|err| DesktopAutomationError::Bridge(err.to_string()))?;
        let input_b64 = STANDARD.encode(input_json);
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "-",
            ])
            .env("CODEX_DAB_TOOL", tool_name)
            .env("CODEX_DAB_INPUT_B64", input_b64)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(DesktopAutomationError::Spawn)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(BRIDGE_SCRIPT.as_bytes())
                .await
                .map_err(DesktopAutomationError::Spawn)?;
        }

        let output = match timeout(
            Duration::from_secs(BRIDGE_TIMEOUT_SECONDS),
            child.wait_with_output(),
        )
        .await
        {
            Ok(result) => result.map_err(DesktopAutomationError::Spawn)?,
            Err(_) => return Err(DesktopAutomationError::Timeout(BRIDGE_TIMEOUT_SECONDS)),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(DesktopAutomationError::Bridge(if stderr.is_empty() {
                stdout
            } else {
                stderr
            }));
        }

        let value: Value =
            serde_json::from_str(&stdout).map_err(DesktopAutomationError::InvalidJson)?;
        let image_url = value
            .get("image_url")
            .and_then(Value::as_str)
            .map(str::to_string);
        Ok(DesktopAutomationResult::with_image(value, image_url))
    }

    const BRIDGE_SCRIPT: &str = r#"
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
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
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
    $pid = [uint32]0
    [void][CodexDabNative]::GetWindowThreadProcessId($hwnd, [ref]$pid)
    $processName = $null
    try { $processName = (Get-Process -Id $pid -ErrorAction Stop).ProcessName } catch {}
    [pscustomobject]@{
        hwnd = ('0x{0:x}' -f $hwnd.ToInt64())
        hwnd_decimal = $hwnd.ToInt64()
        title = Get-WindowTitle $hwnd
        process_id = [int]$pid
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
                x = [int]$rect.X
                y = [int]$rect.Y
                width = [int]$rect.Width
                height = [int]$rect.Height
            }
        })
    }
    $items.ToArray()
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
        '^(new_tab)$' { return '^t' }
        '^(close_tab)$' { return '^w' }
        '^(address_bar|url)$' { return '^l' }
        '^(terminal)$' { return '^`' }
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
    $bytes = [IO.File]::ReadAllBytes($path)
    [pscustomobject]@{
        path = $path
        image_url = 'data:image/png;base64,' + [Convert]::ToBase64String($bytes)
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

try {
    switch ($tool) {
        'dab_find_window' {
            Write-DabResult ([pscustomobject]@{ ok = $true; windows = Find-Windows $ArgsObj })
        }
        'dab_window_check' {
            $window = Resolve-TargetWindow $ArgsObj
            Write-DabResult ([pscustomobject]@{ ok = ($null -ne $window); window = $window })
        }
        'dab_screenshot' {
            $window = if (Test-HasTarget $ArgsObj) { Resolve-TargetWindow $ArgsObj } else { $null }
            $shot = Save-Screenshot $window $ArgsObj
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; screenshot = $shot; image_url = $shot.image_url })
        }
        'dab_element_map' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 80 }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; elements = Get-Elements $window.hwnd $max })
        }
        'dab_ocr' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; text = ''; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 120 }
            $elements = Get-Elements $window.hwnd $max
            $shot = if ($ArgsObj.screenshot -eq $false) { $null } else { Save-Screenshot $window $ArgsObj }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; text = Get-VisibleText $elements; elements = $elements; screenshot = $shot; image_url = if ($shot) { $shot.image_url } else { $null } })
        }
        'dab_visual_scan' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($null -eq $window) { Write-DabResult ([pscustomobject]@{ ok = $false; error = 'target window not found'; elements = @() }); break }
            $max = if ($ArgsObj.max_elements) { [int]$ArgsObj.max_elements } else { 80 }
            $shot = if ($ArgsObj.screenshot -eq $false) { $null } else { Save-Screenshot $window $ArgsObj }
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; elements = Get-Elements $window.hwnd $max; screenshot = $shot; image_url = if ($shot) { $shot.image_url } else { $null } })
        }
        'dab_click' {
            $window = Resolve-TargetWindow $ArgsObj
            if ($window) { [void][CodexDabNative]::SetForegroundWindow((Get-Hwnd $window.hwnd)); Start-Sleep -Milliseconds 80 }
            Invoke-ForegroundClick $ArgsObj.x $ArgsObj.y
            Write-DabResult ([pscustomobject]@{ ok = $true; window = $window; clicked = @{ x = [int]$ArgsObj.x; y = [int]$ArgsObj.y; mode = 'foreground' } })
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
                Write-DabResult ([pscustomobject]@{ ok = $false; error = 'no element matched'; available_elements = $elements | Select-Object -First 30 })
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
            $window = Resolve-TargetWindow $ArgsObj
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
        default {
            Write-DabResult ([pscustomobject]@{ ok = $false; error = "unsupported DAB tool $tool" })
        }
    }
} catch {
    Write-DabResult ([pscustomobject]@{ ok = $false; error = $_.Exception.Message })
    exit 0
}
"#;
}

#[cfg(windows)]
pub use imp::execute_dab_tool;
