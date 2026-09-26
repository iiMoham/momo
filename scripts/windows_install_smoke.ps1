# End-to-end check of the Windows installers against a packaged
# momo-windows-x86_64.zip. Serves a fake release from a local web server, then
# runs the documented `irm ... | iex` one-liner, the install.cmd fallback, and
# the local-package mode `momo update` and remote installs use, all through
# Windows PowerShell 5.1 like users get. Restores the user PATH afterwards.
#
#   pwsh scripts/windows_install_smoke.ps1 -ArchivePath momo-windows-x86_64.zip [-ExpectedVersion "momo 0.9.1-momo.4"]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,

    [string]$ExpectedVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $repo "packaging\windows\install.ps1"
$bootstrap = Join-Path $repo "packaging\windows\install.cmd"
$archive = (Resolve-Path -LiteralPath $ArchivePath).Path

$tokens = $null
$parseErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile($installer, [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($parseErrors.Count -ne 0) {
    throw "install.ps1 does not parse: $($parseErrors | Out-String)"
}
if ([System.IO.File]::ReadAllBytes($installer) | Where-Object { $_ -gt 0x7f } | Select-Object -First 1) {
    throw "install.ps1 must stay ASCII; Windows PowerShell 5.1 misreads UTF-8 without a BOM"
}

function Assert-Installed {
    param([string]$Label, [string]$BinDir)

    $exe = Join-Path $BinDir "momo.exe"
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
        throw "${Label}: $exe was not installed"
    }
    $version = (& $exe --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "${Label}: momo --version failed with exit code $LASTEXITCODE"
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedVersion) -and $version -ne $ExpectedVersion) {
        throw "${Label}: expected '$ExpectedVersion' but momo --version printed '$version'"
    }
    Write-Host "ok: $Label installed $version"
}

function Invoke-WindowsPowerShell {
    param([string[]]$Arguments)

    # Out-Host keeps the installer's messages out of this function's return
    # value, which must be only the exit code.
    & powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass @Arguments | Out-Host
    return $LASTEXITCODE
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("momo-install-smoke-" + [Guid]::NewGuid().ToString("N"))
$webRoot = Join-Path $root "web"
New-Item -ItemType Directory -Force -Path $webRoot | Out-Null
Copy-Item -LiteralPath $archive -Destination (Join-Path $webRoot "momo-windows-x86_64.zip")
Copy-Item -LiteralPath $installer -Destination (Join-Path $webRoot "install.ps1")
Copy-Item -LiteralPath $bootstrap -Destination (Join-Path $webRoot "install.cmd")
$sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()

$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$listener.Stop()
$base = "http://127.0.0.1:$port"

# Same shape as packaging/momo/release_manifest.py writes.
$manifest = [ordered]@{
    version = "0.0.0"
    momo_release = 1
    notes = "smoke test"
    assets = [ordered]@{
        "windows-x86_64" = [ordered]@{ url = "$base/momo-windows-x86_64.zip"; sha256 = $sha256 }
    }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $webRoot "latest.json") -Encoding ascii

$environmentKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey("Environment")
$originalUserPath = $environmentKey.GetValue("Path", $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
$originalUserPathKind = if ($null -eq $originalUserPath) { $null } else { $environmentKey.GetValueKind("Path") }
$savedEnvironment = @{}
foreach ($name in @("MOMO_HOME", "MOMO_INSTALL_DIR", "MOMO_MANIFEST_URL", "MOMO_INSTALLER_URL")) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

$python = (Get-Command python -CommandType Application | Select-Object -First 1).Source
$server = Start-Process -FilePath $python `
    -ArgumentList @("-m", "http.server", "$port", "--bind", "127.0.0.1", "--directory", $webRoot) `
    -PassThru -WindowStyle Hidden
try {
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        & curl.exe --silent --fail --output NUL "$base/latest.json"
        if ($LASTEXITCODE -eq 0) { break }
        if ($attempt -eq 49) { throw "local release server did not start on $base" }
        Start-Sleep -Milliseconds 200
    }
    $env:MOMO_MANIFEST_URL = "$base/latest.json"

    # 1. The one-liner from the README.
    $env:MOMO_HOME = Join-Path $root "oneliner-home"
    $env:MOMO_INSTALL_DIR = Join-Path $root "oneliner-bin"
    $status = Invoke-WindowsPowerShell @("-Command", "irm $base/install.ps1 | iex")
    if ($status -ne 0) { throw "irm | iex install failed with exit code $status" }
    Assert-Installed -Label "irm | iex" -BinDir $env:MOMO_INSTALL_DIR
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $firstEntry = ($userPath -split ";")[0]
    if (-not (Test-Path -LiteralPath (Join-Path $firstEntry "momo.exe") -PathType Leaf)) {
        throw "installer did not put the MoMo release first on the user PATH: $firstEntry"
    }
    Write-Host "ok: user PATH starts with $firstEntry"

    # 2. Rerunning over an existing install is a no-op repair, not an error.
    $status = Invoke-WindowsPowerShell @("-Command", "irm $base/install.ps1 | iex")
    if ($status -ne 0) { throw "reinstall failed with exit code $status" }
    Assert-Installed -Label "reinstall" -BinDir $env:MOMO_INSTALL_DIR

    # 3. The Command Prompt fallback.
    $env:MOMO_HOME = Join-Path $root "cmd-home"
    $env:MOMO_INSTALL_DIR = Join-Path $root "cmd-bin"
    $env:MOMO_INSTALLER_URL = "$base/install.ps1"
    & cmd.exe /d /c (Join-Path $webRoot "install.cmd")
    if ($LASTEXITCODE -ne 0) { throw "install.cmd failed with exit code $LASTEXITCODE" }
    Assert-Installed -Label "install.cmd" -BinDir $env:MOMO_INSTALL_DIR

    # 4. Local-package mode: `momo update` passes -Channel stable, remote
    #    installs pass the build channel (`momo`).
    foreach ($channel in @("stable", "momo")) {
        $env:MOMO_HOME = Join-Path $root "local-$channel-home"
        $env:MOMO_INSTALL_DIR = Join-Path $root "local-$channel-bin"
        $status = Invoke-WindowsPowerShell @(
            "-File", $installer,
            "-Channel", $channel,
            "-LocalPackagePath", $archive,
            "-LocalPackageFormat", "zip",
            "-LocalPackageIdentity", "0.0.0-momo.2",
            "-LocalPackageSha256", $sha256
        )
        if ($status -ne 0) { throw "local package install (-Channel $channel) failed with exit code $status" }
        Assert-Installed -Label "local package (-Channel $channel)" -BinDir $env:MOMO_INSTALL_DIR
    }

    # 5. Refusals: a wrong checksum and the preview channel.
    $env:MOMO_HOME = Join-Path $root "refused-home"
    $env:MOMO_INSTALL_DIR = Join-Path $root "refused-bin"
    $status = Invoke-WindowsPowerShell @(
        "-File", $installer,
        "-LocalPackagePath", $archive,
        "-LocalPackageFormat", "zip",
        "-LocalPackageIdentity", "0.0.0-momo.3",
        "-LocalPackageSha256", ("0" * 64)
    )
    if ($status -eq 0) { throw "installer accepted a package with the wrong checksum" }
    $status = Invoke-WindowsPowerShell @("-File", $installer, "-Channel", "preview")
    if ($status -eq 0) { throw "installer accepted the preview channel" }
    if (Test-Path -LiteralPath (Join-Path $env:MOMO_INSTALL_DIR "momo.exe")) {
        throw "a refused install still activated momo.exe"
    }
    Write-Host "ok: wrong checksum and preview channel are refused"
} finally {
    Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
    foreach ($name in $savedEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], "Process")
    }
    if ($null -eq $originalUserPath) {
        $environmentKey.DeleteValue("Path", $false)
    } else {
        $environmentKey.SetValue("Path", $originalUserPath, $originalUserPathKind)
    }
    $environmentKey.Dispose()
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Windows installer smoke test passed."
