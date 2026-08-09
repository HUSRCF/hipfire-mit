# SPDX-License-Identifier: MIT
# hipfire installer for Windows — detects GPU and builds locked source + kernels.
# Usage: irm https://raw.githubusercontent.com/HUSRCF/hipfire-mit/main/scripts/install.ps1 | iex
$ErrorActionPreference = "Stop"

# ─── Paths ───────────────────────────────────────────────
$HipfireDir  = "$env:USERPROFILE\.hipfire"
$BinDir      = "$HipfireDir\bin"
$RuntimeDir  = "$HipfireDir\runtime"
$ModelsDir   = "$HipfireDir\models"
$SrcDir      = "$HipfireDir\src"

# ─── Source provenance ───────────────────────────────────
$GithubRepo = $env:HIPFIRE_GITHUB_REPO
if ([string]::IsNullOrWhiteSpace($GithubRepo)) { $GithubRepo = "HUSRCF/hipfire-mit" }
$GithubRef = $env:HIPFIRE_INSTALL_REF
if ([string]::IsNullOrWhiteSpace($GithubRef)) { $GithubRef = "main" }
$GithubUrl = $env:HIPFIRE_GITHUB_URL
if ([string]::IsNullOrWhiteSpace($GithubUrl)) { $GithubUrl = "https://github.com/$GithubRepo.git" }

Write-Host "=== hipfire installer ===" -ForegroundColor Cyan
Write-Host ""

# ─── GPU Detection ───────────────────────────────────────
Write-Host "Checking for AMD GPU..." -ForegroundColor Cyan

$GpuArch = "unknown"
try {
    $VideoControllers = Get-CimInstance Win32_VideoController -ErrorAction Stop
    $AmdGpu = $VideoControllers | Where-Object { $_.Name -match "AMD|Radeon" } | Select-Object -First 1
    if ($AmdGpu) {
        $GpuName = $AmdGpu.Name
        Write-Host "  Found: $GpuName"

        # Map GPU name to arch
        if ($GpuName -match "5700|RX 5[0-9]{3}") {
            $GpuArch = "gfx1010"
        } elseif ($GpuName -match "6[89]00|6[79]50|6[89]50|RX 6[0-9]{3}") {
            $GpuArch = "gfx1030"
        } elseif ($GpuName -match "7900|7800|7700|7600|RX 7[0-9]{3}") {
            $GpuArch = "gfx1100"
        } elseif ($GpuName -match "9070") {
            $GpuArch = "gfx1201"
        } elseif ($GpuName -match "9060|RX 9[0-9]{3}") {
            $GpuArch = "gfx1200"
        }
    } else {
        Write-Host "  WARNING: No AMD/Radeon GPU found in Win32_VideoController." -ForegroundColor Yellow
    }
} catch {
    Write-Host "  WARNING: Could not query GPU information: $_" -ForegroundColor Yellow
}

if ($GpuArch -eq "unknown") {
    Write-Host "  WARNING: Could not detect GPU architecture." -ForegroundColor Yellow
    Write-Host "  Supported: gfx1010 (RX 5700), gfx1030 (RX 6800), gfx1100 (RX 7900), gfx1200 (RX 9060), gfx1201 (RX 9070)"
    $GpuArch = Read-Host "  Enter your GPU arch [or Enter to skip]"
    if ([string]::IsNullOrWhiteSpace($GpuArch)) { $GpuArch = "unknown" }
}
Write-Host "  GPU arch: $GpuArch" -ForegroundColor Green

# ─── Create directories ──────────────────────────────────
Write-Host ""
Write-Host "Creating directories..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $BinDir    | Out-Null
New-Item -ItemType Directory -Force -Path $RuntimeDir | Out-Null
New-Item -ItemType Directory -Force -Path $ModelsDir  | Out-Null
Write-Host "  $BinDir" -ForegroundColor Green
Write-Host "  $RuntimeDir" -ForegroundColor Green
Write-Host "  $ModelsDir" -ForegroundColor Green

# ─── HIP DLL (amdhip64.dll) ──────────────────────────────
Write-Host ""
Write-Host "Checking HIP runtime (amdhip64.dll)..." -ForegroundColor Cyan

$HipDllFound = $false
$HipDllDest  = "$RuntimeDir\amdhip64.dll"

# Check RuntimeDir first (idempotent re-runs)
if (Test-Path $HipDllDest) {
    Write-Host "  amdhip64.dll: found in RuntimeDir ✓" -ForegroundColor Green
    $HipDllFound = $true
}

# Check %HIP_PATH%\bin (unversioned and versioned)
if (-not $HipDllFound -and $env:HIP_PATH) {
    foreach ($dllName in @("amdhip64.dll", "amdhip64_7.dll", "amdhip64_6.dll")) {
        $candidate = Join-Path $env:HIP_PATH "bin\$dllName"
        if (Test-Path $candidate) {
            Write-Host "  ${dllName}: found at $candidate ✓" -ForegroundColor Green
            Copy-Item $candidate $HipDllDest -Force
            $HipDllFound = $true
            break
        }
    }
}

# Check standard ROCm install locations (unversioned and versioned)
if (-not $HipDllFound) {
    foreach ($dllName in @("amdhip64.dll", "amdhip64_7.dll", "amdhip64_6.dll")) {
        # Check versioned ROCm dirs (e.g. C:\Program Files\AMD\ROCm\7.1\bin\)
        $rocmBase = "C:\Program Files\AMD\ROCm"
        if (Test-Path $rocmBase) {
            foreach ($verDir in (Get-ChildItem $rocmBase -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending)) {
                $candidate = Join-Path $verDir.FullName "bin\$dllName"
                if (Test-Path $candidate) {
                    Write-Host "  ${dllName}: found at $candidate ✓" -ForegroundColor Green
                    Copy-Item $candidate $HipDllDest -Force
                    $HipDllFound = $true
                    break
                }
            }
            if ($HipDllFound) { break }
        }
        # Also check flat layout
        $candidate = "C:\Program Files\AMD\ROCm\bin\$dllName"
        if (Test-Path $candidate) {
            Write-Host "  ${dllName}: found at $candidate ✓" -ForegroundColor Green
            Copy-Item $candidate $HipDllDest -Force
            $HipDllFound = $true
            break
        }
    }
}

# Do not download a mutable or unauthenticated runtime DLL from a project
# release. The HIP runtime must come from the user's AMD installation.
if (-not $HipDllFound) {
    Write-Host "  amdhip64.dll: not found in the installed AMD HIP SDK." -ForegroundColor Red
    Write-Host "  Install ROCm for Windows from AMD:" -ForegroundColor Yellow
    Write-Host "    https://rocm.docs.amd.com/en/latest/deploy/windows/quick_start.html"
    Write-Host "  Or place your locally installed amdhip64.dll in: $RuntimeDir"
    Write-Host ""
    $reply = Read-Host "  Continue without HIP runtime? [y/N]"
    if ($reply -notmatch "^[Yy]$") {
        Write-Host "Exiting. Re-run after installing ROCm." -ForegroundColor Red
        exit 1
    }
}

# Ensure runtime dir is in PATH for this session so daemon can find the DLL
if ($HipDllFound) {
    $env:PATH = "$RuntimeDir;$env:PATH"
}

# ─── HIP version vs GPU arch check ──────────────────────
if ($HipDllFound -and $GpuArch -ne "unknown") {
    # Try to get HIP version from the DLL or hipconfig
    $HipVer = ""
    $hipconfig = "$env:HIP_PATH\bin\hipconfig.exe"
    if (-not (Test-Path $hipconfig)) { $hipconfig = "C:\Program Files\AMD\ROCm\bin\hipconfig.exe" }
    if (Test-Path $hipconfig) {
        try { $HipVer = (& $hipconfig --version 2>$null) -replace '[^\d.]','' | Select-Object -First 1 } catch {}
    }
    # Fallback: check DLL file version
    if (-not $HipVer) {
        try {
            $dllPath = if (Test-Path $HipDllDest) { $HipDllDest } else { $candidate }
            $ver = (Get-Item $dllPath).VersionInfo.ProductVersion
            if ($ver) { $HipVer = $ver }
        } catch {}
    }

    if ($HipVer) {
        $parts = $HipVer.Split(".")
        $major = [int]$parts[0]
        $minor = if ($parts.Length -gt 1) { [int]$parts[1] } else { 0 }
        Write-Host "  HIP version: $major.$minor" -ForegroundColor Green

        # Minimum versions per arch
        $minMajor = 5; $minMinor = 0
        switch ($GpuArch) {
            { $_ -in "gfx1200","gfx1201" } { $minMajor = 6; $minMinor = 4 }
            { $_ -in "gfx1100","gfx1101" } { $minMajor = 5; $minMinor = 5 }
        }

        if ($major -lt $minMajor -or ($major -eq $minMajor -and $minor -lt $minMinor)) {
            Write-Host ""
            Write-Host "  WARNING: HIP $major.$minor is too old for $GpuArch (needs $minMajor.$minMinor+)" -ForegroundColor Red
            Write-Host "  Kernels may fail to load. Update AMD HIP SDK:" -ForegroundColor Yellow
            Write-Host "    https://www.amd.com/en/developer/resources/rocm-hub/hip-sdk.html" -ForegroundColor Yellow
            Write-Host ""
            $reply = Read-Host "  Continue anyway? [y/N]"
            if ($reply -notmatch "^[Yy]$") { exit 1 }
        }
    }
}

# ─── Bun (CLI runtime) ───────────────────────────────────
Write-Host ""
Write-Host "Checking Bun..." -ForegroundColor Cyan

$BunBin = "$env:USERPROFILE\.bun\bin"
if (Get-Command bun -ErrorAction SilentlyContinue) {
    Write-Host "  Bun: found ✓" -ForegroundColor Green
} else {
    Write-Host "  Bun not found. Installing..." -ForegroundColor Yellow
    try {
        powershell -c "irm bun.sh/install.ps1 | iex"
        # Add bun to PATH for remainder of this session
        $env:PATH = "$BunBin;$env:PATH"
        if (Get-Command bun -ErrorAction SilentlyContinue) {
            Write-Host "  Bun installed ✓" -ForegroundColor Green
        } else {
            Write-Host "  Bun installed but not in PATH. Add manually:" -ForegroundColor Yellow
            Write-Host "    $BunBin"
        }
    } catch {
        Write-Host "  Bun install failed: $_" -ForegroundColor Red
        Write-Host "  Visit https://bun.sh and install manually, then re-run."
        exit 1
    }
}

# ─── Clone / update repo ─────────────────────────────────
Write-Host ""
Write-Host "Setting up hipfire source..." -ForegroundColor Cyan

if (-not (Test-Path "$SrcDir\.git")) {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Host "  ERROR: git is required. Install from https://git-scm.com and re-run." -ForegroundColor Red
        exit 1
    }
    Write-Host "  Cloning $GithubUrl ..."
    & git clone --filter=blob:none --no-checkout $GithubUrl $SrcDir
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Clone failed." -ForegroundColor Red
        Write-Host "  Try manually: git clone $GithubUrl $SrcDir"
        exit 1
    }
    Write-Host "  Cloned ✓" -ForegroundColor Green
} else {
    Write-Host "  Existing clone found at $SrcDir"
    $status = & git -C $SrcDir status --porcelain 2>&1 | Out-String
    if ($status.Trim()) {
        Write-Host "  Local modifications detected." -ForegroundColor Yellow
        $reply = Read-Host "  Stash local changes and install the requested ref? [y/N]"
        if ($reply -match "^[Yy]$") {
            $stamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH-mm-ssZ")
            $stashMsg = "hipfire-install-$stamp"
            & git -C $SrcDir stash push --include-untracked -m $stashMsg 2>&1 | Out-Null
            if ($LASTEXITCODE -ne 0) {
                Write-Host "  git stash failed; aborting without resetting." -ForegroundColor Red
                exit 1
            }
            Write-Host "  Recover later with: git -C $SrcDir stash pop" -ForegroundColor Yellow
        } else {
            Write-Host "  Aborting so the existing checkout remains unchanged." -ForegroundColor Yellow
            exit 1
        }
    }
    & git -C $SrcDir remote set-url origin $GithubUrl
    if ($LASTEXITCODE -ne 0) { throw "failed to set clean-room source URL" }
}

$env:GIT_TERMINAL_PROMPT = "0"
Write-Host "  Fetching ref $GithubRef ..."
& git -C $SrcDir fetch origin $GithubRef --depth 1
if ($LASTEXITCODE -ne 0) { throw "could not fetch '$GithubRef' from $GithubUrl" }
$ResolvedCommit = (& git -C $SrcDir rev-parse --verify FETCH_HEAD 2>$null | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($ResolvedCommit)) {
    throw "could not resolve fetched ref '$GithubRef'"
}
& git -C $SrcDir checkout --detach --force $ResolvedCommit 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "failed to check out source commit $ResolvedCommit" }
Write-Host "  Source commit: $ResolvedCommit ✓" -ForegroundColor Green

$RepoDir = $SrcDir
$Provenance = "repository=$GithubUrl`nrequested_ref=$GithubRef`nresolved_commit=$ResolvedCommit`n"
[System.IO.File]::WriteAllText("$HipfireDir\install-source.txt", $Provenance)
Write-Host "  Install provenance: $HipfireDir\install-source.txt" -ForegroundColor Green

# ─── Build / install binaries ────────────────────────────
Write-Host ""
Write-Host "Installing hipfire binaries..." -ForegroundColor Cyan

# Build the selected checkout from Cargo.lock. Always invoking Cargo validates
# that cached artifacts match this source commit while retaining incremental
# compilation when they do.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "  Installing Rust via rustup..." -ForegroundColor Yellow
    $RustupUrl = "https://win.rustup.rs/x86_64"
    $RustupExe = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $RustupUrl -OutFile $RustupExe -UseBasicParsing
    & $RustupExe -y --default-toolchain stable
    if ($LASTEXITCODE -ne 0) { throw "rustup installation failed" }
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

Write-Host "  cargo build --release --locked (this may take several minutes)..."
Push-Location $RepoDir
try {
    & cargo build --release --locked --features deltanet --example daemon --example infer --example infer_hfq -p hipfire-runtime
    if ($LASTEXITCODE -ne 0) { throw "locked source build failed" }
} finally {
    Pop-Location
}

$Meta = cargo metadata --locked --format-version 1 --manifest-path "$RepoDir\Cargo.toml" 2>$null | ConvertFrom-Json
$TargetDir = $Meta.target_directory
$BuiltExe = "$TargetDir\release\examples\daemon.exe"
if (-not (Test-Path $BuiltExe)) {
    throw "build completed without expected daemon at $BuiltExe"
}
Copy-Item $BuiltExe "$BinDir\daemon.exe" -Force
Write-Host "  Build complete ✓" -ForegroundColor Green

# Copy optional helper binaries if present
foreach ($exe in @("infer.exe", "infer_hfq.exe")) {
    $src = "$TargetDir\release\examples\$exe"
    if (Test-Path $src) { Copy-Item $src "$BinDir\$exe" -Force }
}

# ─── CLI ─────────────────────────────────────────────────
Write-Host ""
Write-Host "Installing CLI..." -ForegroundColor Cyan

$CliDir = "$HipfireDir\cli"
New-Item -ItemType Directory -Force -Path $CliDir | Out-Null
# Recursive copy of the whole cli\ directory, then prune dev/test artifacts.
# New .ts files added to cli\ (next chat helper, future slash-command module)
# are picked up automatically — no install-script edit required. Replaces
# the previous per-file enumeration that grew stale after PR #129 added
# chat.ts/chat_pure.ts (issue #163, patched in #165; this is the structural
# follow-up that PR left for later).
if (-not (Test-Path "$RepoDir\cli\registry.json") -or -not (Test-Path "$RepoDir\cli\index.ts")) {
    Write-Host "ERROR: cli\registry.json or cli\index.ts missing in $RepoDir" -ForegroundColor Red
    Write-Host "       Repo checkout may be incomplete; aborting install." -ForegroundColor Red
    exit 1
}
# Robocopy mirrors better than Copy-Item -Recurse for this case (handles
# permissions, exit-code semantics, and is on every Windows installation),
# but Copy-Item is more portable across PowerShell core / Windows PS / pwsh
# on macOS-via-PS-remoting; sticking with Copy-Item for parity with the rest
# of the script.
Copy-Item "$RepoDir\cli\*" $CliDir -Recurse -Force
# Prune dev artifacts. Patterns mirror install.sh — tests follow
# `*.test.ts` / `test_*.ts` / `bench_*.ts` Bun conventions; node_modules
# and dotfiles are dev-only. Adding a new test file with the same naming
# requires no install-script change.
$prunePaths = @("node_modules", ".gitignore", "tsconfig.json", "README.md", "bun.lock")
foreach ($p in $prunePaths) {
    $target = Join-Path $CliDir $p
    if (Test-Path $target) { Remove-Item $target -Recurse -Force -ErrorAction SilentlyContinue }
}
Get-ChildItem -Path $CliDir -File | Where-Object {
    $_.Name -like "*.test.ts" -or $_.Name -like "test_*.ts" -or $_.Name -like "bench_*.ts"
} | Remove-Item -Force -ErrorAction SilentlyContinue

# Create hipfire.cmd wrapper
$CmdWrapper = "@echo off`r`nbun run `"%USERPROFILE%\.hipfire\cli\index.ts`" %*`r`n"
[System.IO.File]::WriteAllText("$BinDir\hipfire.cmd", $CmdWrapper)

Write-Host "  CLI installed to $CliDir ✓" -ForegroundColor Green
Write-Host "  Wrapper: $BinDir\hipfire.cmd ✓" -ForegroundColor Green

# ─── Kernels ─────────────────────────────────────────────
# kernels/compiled/<arch>/ is gitignored, so a fresh git clone never ships
# .hsaco blobs. We mirror the Linux flow (install.sh): seed any blobs that
# happen to be present in the checkout (developer case), then run
# daemon.exe --precompile to JIT-compile the default Qwen3.5 kernel set
# into ~/.hipfire/bin/kernels/compiled/<arch>/. First `hipfire run` is then
# instant instead of a multi-minute hipcc wall.
Write-Host ""
if ($GpuArch -ne "unknown") {
    Write-Host "Setting up kernels for $GpuArch..." -ForegroundColor Cyan
    $KernelSrc  = "$RepoDir\kernels\compiled\$GpuArch"
    $KernelDest = "$BinDir\kernels\compiled\$GpuArch"
    New-Item -ItemType Directory -Force -Path $KernelDest | Out-Null

    if (Test-Path $KernelSrc) {
        $Hsacos = Get-ChildItem "$KernelSrc\*.hsaco" -ErrorAction SilentlyContinue
        if ($Hsacos -and $Hsacos.Count -gt 0) {
            Copy-Item "$KernelSrc\*.hsaco" $KernelDest -Force
            Copy-Item "$KernelSrc\*.hash" $KernelDest -Force -ErrorAction SilentlyContinue
            Write-Host "  Seeded $($Hsacos.Count) kernels from repo checkout to $KernelDest ✓" -ForegroundColor Green
        } else {
            Write-Host "  No pre-compiled .hsaco found in repo (gitignored). Will JIT-compile below." -ForegroundColor Yellow
        }
    } else {
        Write-Host "  No pre-compiled kernels for $GpuArch in repo (gitignored). Will JIT-compile below." -ForegroundColor Yellow
    }
} else {
    Write-Host "Skipping kernel setup (GPU arch unknown)." -ForegroundColor Yellow
    Write-Host "  Re-run installer after fixing GPU detection, or run scripts\compile-kernels.ps1 manually."
}

# ─── Pre-compile via daemon (parity with install.sh) ─────
# Fills in any missing kernels for the active GPU. Uses hipcc in the
# background; writes back to ~/.hipfire/bin/kernels/compiled/<arch>/.
# Runs even when GpuArch is "unknown"; Gpu::init resolves the active arch
# at runtime regardless of install-time detection.
$DaemonExe = "$BinDir\daemon.exe"
if (Test-Path $DaemonExe) {
    Write-Host ""
    Write-Host "Pre-compiling GPU kernels (first run will be instant afterward)..." -ForegroundColor Cyan
    $hipccAvailable = $false
    if ($env:HIP_PATH -and (Test-Path (Join-Path $env:HIP_PATH "bin\hipcc.bat"))) { $hipccAvailable = $true }
    elseif ($env:HIP_PATH -and (Test-Path (Join-Path $env:HIP_PATH "bin\hipcc.exe"))) { $hipccAvailable = $true }
    elseif (Get-Command hipcc -ErrorAction SilentlyContinue) { $hipccAvailable = $true }
    elseif (Test-Path "C:\Program Files\AMD\ROCm") {
        $rocmHipcc = Get-ChildItem "C:\Program Files\AMD\ROCm" -Recurse -Filter "hipcc.bat" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($rocmHipcc) { $hipccAvailable = $true }
    }

    if (-not $hipccAvailable) {
        Write-Host "  hipcc not found in PATH or `$env:HIP_PATH; skipping pre-compile." -ForegroundColor Yellow
        Write-Host "  Install the AMD HIP SDK to enable JIT compilation:" -ForegroundColor Yellow
        Write-Host "    https://www.amd.com/en/developer/resources/rocm-hub/hip-sdk.html" -ForegroundColor Yellow
        Write-Host "  Pre-compiled blobs in the repo will still load if available."
    } else {
        try {
            & $DaemonExe --precompile
            if ($LASTEXITCODE -eq 0) {
                Write-Host "  Pre-compile complete ✓" -ForegroundColor Green
            } else {
                Write-Host "  Pre-compile finished with warnings; missing kernels will JIT on first use." -ForegroundColor Yellow
            }
        } catch {
            Write-Host "  Pre-compile failed: $_; missing kernels will JIT on first use." -ForegroundColor Yellow
        }
    }
}

# ─── Config ──────────────────────────────────────────────
$ConfigFile = "$HipfireDir\config.json"
if (-not (Test-Path $ConfigFile)) {
    $Config = [ordered]@{
        temperature = 0.3
        top_p       = 0.8
        max_tokens  = 512
        gpu_arch    = $GpuArch
    } | ConvertTo-Json
    [System.IO.File]::WriteAllText($ConfigFile, $Config)
    Write-Host ""
    Write-Host "Config written: $ConfigFile" -ForegroundColor Green
}

# ─── PATH ────────────────────────────────────────────────
Write-Host ""
$NoPath = $args -contains "--no-path"
$CurrentUserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($null -eq $CurrentUserPath) { $CurrentUserPath = "" }

if ($NoPath) {
    Write-Host "Skipping PATH modification (--no-path)" -ForegroundColor Yellow
    Write-Host "  Add manually to user PATH: $BinDir" -ForegroundColor Yellow
} elseif ($CurrentUserPath -notlike "*$BinDir*") {
    Write-Host "hipfire bin dir is not in your user PATH." -ForegroundColor Yellow
    Write-Host "  $BinDir"
    $reply = Read-Host "Add to user PATH permanently? [Y/n]"
    if ($reply -notmatch "^[Nn]$") {
        $NewPath = "$BinDir;$CurrentUserPath"
        # Safety: warn if PATH would exceed Windows limit (2047 chars)
        if ($NewPath.Length -gt 2040) {
            Write-Host "  WARNING: User PATH would be $($NewPath.Length) chars (limit ~2047)." -ForegroundColor Red
            Write-Host "  Skipping to avoid PATH truncation. Add manually:" -ForegroundColor Red
            Write-Host "    $BinDir" -ForegroundColor Yellow
        } else {
            [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
            $env:PATH = "$BinDir;$env:PATH"
            Write-Host "  PATH updated ✓ (restart your shell to apply)" -ForegroundColor Green
        }
    } else {
        Write-Host "  Add manually to user PATH: $BinDir" -ForegroundColor Yellow
    }
} else {
    Write-Host "hipfire already in PATH ✓" -ForegroundColor Green
}

# ─── Quick start ─────────────────────────────────────────
Write-Host ""
Write-Host "=== hipfire installed ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Quick start:" -ForegroundColor Green
Write-Host "  hipfire list                        # see local models"
Write-Host "  hipfire run <model.hfq> `"Hello`"    # generate text"
Write-Host "  hipfire serve                       # start OpenAI-compatible API"
Write-Host ""
Write-Host "Models go in $ModelsDir"
Write-Host ""
