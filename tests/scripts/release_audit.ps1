$ErrorActionPreference = "Continue"
$ProjectRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location $ProjectRoot

$RG = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory + "\release\repo-guard.exe"
if (-not (Test-Path $RG)) {
    cargo build --release | Out-Null
    $RG = (cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory + "\release\repo-guard.exe"
}

$tmpdir = New-Item -ItemType Directory -Path ([System.IO.Path]::GetTempPath() + [guid]::NewGuid().ToString())
Set-Location $tmpdir.FullName
Write-Host "AUDIT REPO: $($tmpdir.FullName)"
Write-Host "BINARY: $RG"

git init 2>$null | Out-Null
git config user.email "audit@test.com"
git config user.name "Audit"

function Invoke-RG([string[]]$RgArgs) {
    $outFile = Join-Path $tmpdir.FullName "rg_out.txt"
    $errFile = Join-Path $tmpdir.FullName "rg_err.txt"
    $p = Start-Process -FilePath $RG -ArgumentList $RgArgs -WorkingDirectory $tmpdir.FullName -Wait -PassThru `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile -NoNewWindow
    $out = if (Test-Path $outFile) { Get-Content $outFile -Raw } else { "" }
    return @{ code = $p.ExitCode; out = $out }
}

$log = @()

$r = Invoke-RG @("init")
$log += "init dry-run: exit=$($r.code) config=$([bool](Test-Path .repo-guard.toml)) hooks=$([bool](Test-Path .githooks))"

$r = Invoke-RG @("init", "--apply")
$log += "init apply: exit=$($r.code) config=$([bool](Test-Path .repo-guard.toml)) hook=$([bool](Test-Path .githooks/pre-commit)) hooksPath=$(git config --local --get core.hooksPath)"

$r = Invoke-RG @("doctor")
$log += "doctor: exit=$($r.code)"

$r = Invoke-RG @("doctor", "--format", "json")
$jsonOk = $false
try { $null = ($r.out | ConvertFrom-Json); $jsonOk = $true } catch { $jsonOk = $false }
$log += "doctor json: exit=$($r.code) valid=$jsonOk"

$r = Invoke-RG @("install")
$log += "install dry-run: exit=$($r.code)"

$r = Invoke-RG @("install", "--apply")
$log += "install apply: exit=$($r.code)"

"small" | Out-File small.txt -Encoding ascii
git add small.txt 2>$null | Out-Null
$r = Invoke-RG @("check")
$log += "check small: exit=$($r.code)"

$large = New-Object byte[] 52428800
[System.IO.File]::WriteAllBytes("$($tmpdir.FullName)\big.bin", $large)
git add big.bin 2>$null | Out-Null
$r = Invoke-RG @("check")
$log += "check large: exit=$($r.code) repair=$($r.out -match 'git lfs track')"

$r = Invoke-RG @("scan", "--format", "json")
$log += "scan: exit=$($r.code)"

$r = Invoke-RG @("lfs", "check")
$log += "lfs check: exit=$($r.code)"

$r = Invoke-RG @("ignore", "check")
$log += "ignore check: exit=$($r.code)"

$r = Invoke-RG @("preset", "list")
$log += "preset list: exit=$($r.code)"

$r = Invoke-RG @("remote", "check")
$log += "remote check: exit=$($r.code)"

$r = Invoke-RG @("hook", "pre-commit")
$log += "hook pre-commit large: exit=$($r.code)"

$stdinFile = Join-Path $tmpdir.FullName "refs.txt"
Set-Content -Path $stdinFile -Value "refs/heads/main 0000000000000000000000000000000000000000 1111111111111111111111111111111111111111 refs/heads/main" -NoNewline
$p = Start-Process -FilePath $RG -ArgumentList @("hook", "pre-push") -WorkingDirectory $tmpdir.FullName -Wait -PassThru `
    -RedirectStandardInput $stdinFile -RedirectStandardOutput (Join-Path $tmpdir.FullName "pp_out.txt") `
    -RedirectStandardError (Join-Path $tmpdir.FullName "pp_err.txt") -NoNewWindow
$log += "hook pre-push: exit=$($p.ExitCode)"

$r = Invoke-RG @("preset", "apply", "python")
$log += "preset dry-run: exit=$($r.code) gitignore=$([bool](Test-Path .gitignore))"

git reset HEAD big.bin 2>$null | Out-Null
git lfs install --local 2>$null | Out-Null
git lfs track --filename "big.bin" 2>$null | Out-Null
git add .gitattributes 2>$null | Out-Null
git add --renormalize -- big.bin 2>$null | Out-Null
$r = Invoke-RG @("check")
$log += "check after lfs repair: exit=$($r.code)"

$spdir = "spaced dir"
New-Item -ItemType Directory -Path $spdir | Out-Null
"ok" | Out-File "$spdir\file.txt" -Encoding ascii
git add -- "$spdir/file.txt" 2>$null | Out-Null
$r = Invoke-RG @("check")
$log += "check spaced path: exit=$($r.code)"

Copy-Item .repo-guard.toml .repo-guard.toml.bak
"not valid [[[" | Out-File .repo-guard.toml -Encoding ascii
$r = Invoke-RG @("doctor")
$log += "doctor invalid toml: exit=$($r.code)"
Copy-Item .repo-guard.toml.bak .repo-guard.toml -Force

$env:PATH = (Split-Path $RG) + ";" + $env:PATH
git commit -m "audit commit" 2>&1 | Out-Null
$log += "git commit blocked: exit=$LASTEXITCODE"

$log | ForEach-Object { Write-Host $_ }

Set-Location $ProjectRoot
Remove-Item -Recurse -Force $tmpdir.FullName -ErrorAction SilentlyContinue
