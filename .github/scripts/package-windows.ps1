$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$target = 'x86_64-pc-windows-msvc'
$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
$version = $metadata.packages[0].version
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "MSI packaging requires a three-part release version; got $version."
}
$binary = (Resolve-Path "target/$target/release/logloom.exe").Path
$dist = New-Item -ItemType Directory -Force "target/dist/$target"
$msi = Join-Path $dist.FullName "logloom-$version-windows-x64.msi"
$object = Join-Path $env:RUNNER_TEMP 'logloom.wixobj'

& $binary --version
& $binary validate --strict CHANGELOG.md
Copy-Item $binary, 'LICENSE', 'README.md' $dist.FullName

# WiX 3 is included in the pinned windows-2022 runner image.
& "$env:WIX/bin/candle.exe" -nologo -wx -arch x64 "-dVersion=$version" "-dBinary=$binary" `
    -out $object .github/packaging/windows.wxs
& "$env:WIX/bin/light.exe" -nologo -wx -out $msi $object

function Invoke-Msi([string[]] $Arguments) {
    $process = Start-Process msiexec.exe -ArgumentList $Arguments -Wait -PassThru
    if ($process.ExitCode -notin @(0, 3010)) {
        throw "Windows Installer failed with exit code $($process.ExitCode)."
    }
}

$installedDirectory = Join-Path $env:ProgramFiles 'logloom'
$installedBinary = Join-Path $installedDirectory 'logloom.exe'
$pathBefore = [Environment]::GetEnvironmentVariable('Path', 'Machine')
try {
    Invoke-Msi -Arguments @('/i', "`"$msi`"", '/qn', '/norestart', '/l*v', "`"$env:RUNNER_TEMP/logloom-install.log`"")
    & $installedBinary --version
    & $installedBinary validate --strict CHANGELOG.md
    if ((Get-FileHash $binary).Hash -ne (Get-FileHash $installedBinary).Hash) {
        throw 'The installed executable differs from the built executable.'
    }
    $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $pathEntries = $machinePath.Split(';') | ForEach-Object { $_.TrimEnd('\') }
    if ($pathEntries -notcontains $installedDirectory) {
        throw 'The installer did not add logloom to the system PATH.'
    }
} finally {
    Invoke-Msi -Arguments @('/x', "`"$msi`"", '/qn', '/norestart', '/l*v', "`"$env:RUNNER_TEMP/logloom-uninstall.log`"")
}
if (Test-Path $installedBinary) {
    throw 'Uninstall left the executable behind.'
}
if ([Environment]::GetEnvironmentVariable('Path', 'Machine') -ne $pathBefore) {
    throw 'Uninstall did not restore the system PATH.'
}

$checksums = Get-ChildItem $dist.FullName -File | Sort-Object Name | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $($_.Name)"
}
$checksums | Set-Content (Join-Path $dist.FullName 'SHA256SUMS') -Encoding ascii
