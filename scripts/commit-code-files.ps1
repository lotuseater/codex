param(
    [string]$Message = "chore: commit code files",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel).Trim()
if (-not $repoRoot) {
    throw "Not inside a git repository."
}

$extensions = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
@(".rs", ".py", ".ps1", ".psm1", ".psd1", ".bat", ".cmd") | ForEach-Object {
    [void]$extensions.Add($_)
}

function Select-CodePath {
    param([string[]]$Paths)

    foreach ($path in $Paths) {
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }

        $normalized = $path.Replace("\", "/")
        if ($extensions.Contains([System.IO.Path]::GetExtension($normalized))) {
            $normalized
        }
    }
}

$worktreePaths = @(git -C $repoRoot ls-files --modified --deleted --others --exclude-standard)
$stagedPaths = @(git -C $repoRoot diff --cached --name-only --diff-filter=ACDMRTUXB)
$paths = @(Select-CodePath -Paths @($worktreePaths + $stagedPaths) | Sort-Object -Unique)

if ($paths.Count -eq 0) {
    Write-Host "No changed Rust, Python, PowerShell, or batch files found."
    exit 0
}

Write-Host "Selected $($paths.Count) code file(s)."
if ($DryRun) {
    $paths | ForEach-Object { Write-Host $_ }
    exit 0
}

$pathspecFile = New-TemporaryFile
try {
    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($pathspecFile.FullName, ($paths -join "`n"), $utf8NoBom)

    git -C $repoRoot add --pathspec-from-file=$($pathspecFile.FullName)

    $selected = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::Ordinal
    )
    $paths | ForEach-Object {
        [void]$selected.Add($_)
    }
    $stagedCodePaths = @(
        git -C $repoRoot diff --cached --name-only --diff-filter=ACDMRTUXB |
            Where-Object { $selected.Contains($_.Replace("\", "/")) }
    )
    if (-not $stagedCodePaths) {
        Write-Host "No staged code changes to commit after filtering."
        exit 0
    }

    git -C $repoRoot commit -m $Message --pathspec-from-file=$($pathspecFile.FullName)
} finally {
    Remove-Item -LiteralPath $pathspecFile.FullName -Force -ErrorAction SilentlyContinue
}
