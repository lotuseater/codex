if (-not ("SolidTerminalWindowNative" -as [type])) {
    Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public delegate bool SolidEnumWindowsProc(IntPtr hWnd, IntPtr lParam);

public static class SolidTerminalWindowNative
{
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(SolidEnumWindowsProc lpEnumFunc, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool IsWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out int lpdwProcessId);
}
"@
}

function Get-SolidVisibleWindows {
    $windows = [System.Collections.Generic.List[object]]::new()
    $callback = [SolidEnumWindowsProc]{
        param([IntPtr]$Hwnd, [IntPtr]$Lparam)

        if ([SolidTerminalWindowNative]::IsWindowVisible($Hwnd)) {
            $title = [System.Text.StringBuilder]::new(512)
            [void][SolidTerminalWindowNative]::GetWindowText($Hwnd, $title, $title.Capacity)
            $windowTitle = $title.ToString()
            if (-not [string]::IsNullOrWhiteSpace($windowTitle)) {
                $windowPid = 0
                [void][SolidTerminalWindowNative]::GetWindowThreadProcessId($Hwnd, [ref]$windowPid)
                $windows.Add([pscustomobject]@{
                    Handle = $Hwnd
                    ProcessId = [int]$windowPid
                    Title = $windowTitle
                }) | Out-Null
            }
        }

        return $true
    }

    [void][SolidTerminalWindowNative]::EnumWindows($callback, [IntPtr]::Zero)
    $windows
}

function Get-SolidWindowByHandle {
    param([long]$WindowHandle)

    if ($WindowHandle -le 0) {
        return $null
    }

    $handle = [IntPtr]::new($WindowHandle)
    if (-not [SolidTerminalWindowNative]::IsWindow($handle)) {
        return $null
    }

    if (-not [SolidTerminalWindowNative]::IsWindowVisible($handle)) {
        return $null
    }

    $title = [System.Text.StringBuilder]::new(512)
    [void][SolidTerminalWindowNative]::GetWindowText($handle, $title, $title.Capacity)
    $windowPid = 0
    [void][SolidTerminalWindowNative]::GetWindowThreadProcessId($handle, [ref]$windowPid)

    [pscustomobject]@{
        Handle = $handle
        ProcessId = [int]$windowPid
        Title = $title.ToString()
    }
}

function Get-SolidProcessTreeIds {
    param([int]$RootPid)

    if ($RootPid -le 0) {
        return @()
    }

    $processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $queue = [System.Collections.Generic.Queue[int]]::new()
    [void]$ids.Add($RootPid)
    $queue.Enqueue($RootPid)

    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($child in $processes | Where-Object { [int]$_.ParentProcessId -eq $parent }) {
            $childId = [int]$child.ProcessId
            if ($ids.Add($childId)) {
                $queue.Enqueue($childId)
            }
        }
    }

    @($ids)
}

function Find-SolidTerminalWindow {
    param(
        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0
    )

    $byHandle = Get-SolidWindowByHandle -WindowHandle $WindowHandle
    if ($byHandle) {
        return $byHandle
    }

    $windows = @(Get-SolidVisibleWindows)
    if (-not [string]::IsNullOrWhiteSpace($Title)) {
        $exact = $windows | Where-Object { $_.Title -eq $Title } | Select-Object -First 1
        if ($exact) {
            return $exact
        }

        $contains = $windows | Where-Object { $_.Title -like "*$Title*" } | Select-Object -First 1
        if ($contains) {
            return $contains
        }
    }

    $treeIds = @(Get-SolidProcessTreeIds -RootPid $RootPid)
    if ($treeIds.Count -gt 0) {
        $byTree = $windows | Where-Object { $treeIds -contains [int]$_.ProcessId } | Select-Object -First 1
        if ($byTree) {
            return $byTree
        }
    }

    return $null
}

function Wait-SolidTerminalWindow {
    param(
        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0,
        [long[]]$BaselineHandles = @(),
        [int]$WaitMs = 5000
    )

    $baseline = [System.Collections.Generic.HashSet[long]]::new()
    foreach ($handle in @($BaselineHandles)) {
        [void]$baseline.Add([long]$handle)
    }

    $deadline = [DateTime]::UtcNow.AddMilliseconds($WaitMs)
    while ([DateTime]::UtcNow -lt $deadline) {
        $window = Find-SolidTerminalWindow -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle
        if ($window) {
            return $window
        }

        if ($baseline.Count -gt 0) {
            $newWindow = Get-SolidVisibleWindows |
                Where-Object { -not $baseline.Contains($_.Handle.ToInt64()) -and $_.Title -ne "Program Manager" } |
                Select-Object -First 1
            if ($newWindow) {
                return $newWindow
            }
        }

        Start-Sleep -Milliseconds 100
    }

    return $null
}

function Wait-SolidTerminalActivation {
    param(
        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0,
        [int]$WaitMs = 5000
    )

    $shell = New-Object -ComObject WScript.Shell
    $deadline = [DateTime]::UtcNow.AddMilliseconds($WaitMs)

    while ([DateTime]::UtcNow -lt $deadline) {
        $handleWindow = Get-SolidWindowByHandle -WindowHandle $WindowHandle
        if ($handleWindow -and [SolidTerminalWindowNative]::SetForegroundWindow($handleWindow.Handle)) {
            return [pscustomobject]@{
                Method = "SetForegroundWindow:handle"
                RootPid = $RootPid
                WindowPid = $handleWindow.ProcessId
                WindowHandle = $handleWindow.Handle.ToInt64()
                Title = $handleWindow.Title
            }
        }

        if (-not [string]::IsNullOrWhiteSpace($Title) -and $shell.AppActivate($Title)) {
            return [pscustomobject]@{
                Method = "AppActivate:title"
                RootPid = $RootPid
                WindowPid = $null
                WindowHandle = $WindowHandle
                Title = $Title
            }
        }

        if ($RootPid -gt 0 -and $shell.AppActivate($RootPid)) {
            return [pscustomobject]@{
                Method = "AppActivate:pid"
                RootPid = $RootPid
                WindowPid = $RootPid
                WindowHandle = $WindowHandle
                Title = $Title
            }
        }

        $window = Find-SolidTerminalWindow -Title $Title -RootPid $RootPid
        if ($window -and [SolidTerminalWindowNative]::SetForegroundWindow($window.Handle)) {
            Start-Sleep -Milliseconds 75
            return [pscustomobject]@{
                Method = "SetForegroundWindow"
                RootPid = $RootPid
                WindowPid = $window.ProcessId
                WindowHandle = $window.Handle.ToInt64()
                Title = $window.Title
            }
        }

        Start-Sleep -Milliseconds 100
    }

    throw "Timed out waiting for terminal window activation. Title='$Title' RootPid=$RootPid"
}

function Invoke-SolidTerminalPasteEnter {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message,

        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0,
        [int]$WaitMs = 5000,
        [int]$PasteDelayMs = 250,
        [string]$SubmitKey = "{ENTER}",
        [int]$SubmitRepeat = 1,
        [int]$SubmitDelayMs = 250
    )

    $activation = Wait-SolidTerminalActivation -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle -WaitMs $WaitMs
    Set-Clipboard -Value $Message
    Start-Sleep -Milliseconds $PasteDelayMs

    $shell = New-Object -ComObject WScript.Shell
    $shell.SendKeys("^v")
    Start-Sleep -Milliseconds $PasteDelayMs
    for ($i = 0; $i -lt $SubmitRepeat; $i++) {
        $shell.SendKeys($SubmitKey)
        Start-Sleep -Milliseconds $SubmitDelayMs
    }

    [pscustomobject]@{
        Sent = $true
        Method = $activation.Method
        RootPid = $RootPid
        WindowPid = $activation.WindowPid
        WindowHandle = $activation.WindowHandle
        Title = $activation.Title
        SubmitKey = $SubmitKey
        SubmitRepeat = $SubmitRepeat
        MessageLength = $Message.Length
    }
}

function Invoke-SolidTerminalSendKeys {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Keys,

        [string]$Title,
        [int]$RootPid = 0,
        [long]$WindowHandle = 0,
        [int]$WaitMs = 800,
        [int]$Repeat = 1,
        [int]$DelayMs = 25
    )

    $activation = Wait-SolidTerminalActivation -Title $Title -RootPid $RootPid -WindowHandle $WindowHandle -WaitMs $WaitMs
    $shell = New-Object -ComObject WScript.Shell
    for ($i = 0; $i -lt $Repeat; $i++) {
        $shell.SendKeys($Keys)
        if ($DelayMs -gt 0 -and $i -lt ($Repeat - 1)) {
            Start-Sleep -Milliseconds $DelayMs
        }
    }

    [pscustomobject]@{
        Sent = $true
        Method = $activation.Method
        RootPid = $RootPid
        WindowPid = $activation.WindowPid
        WindowHandle = $activation.WindowHandle
        Title = $activation.Title
        Keys = $Keys
        Repeat = $Repeat
    }
}
