# Windows PowerShell 5.1-compatible, Python-free preview verification.
# ASCII source avoids legacy Windows PowerShell script decoding ambiguity.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackageRoot,
    [Parameter(Mandatory = $true)][string]$ReportPath
)
Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$Utf8 = New-Object System.Text.UTF8Encoding($false, $true)
$checks = New-Object 'System.Collections.Generic.List[string]'
$temporary = $null

function Require([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function Hash-File([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function From-Points([int[]]$Points) {
    return -join ($Points | ForEach-Object { [char]$_ })
}
function Snapshot([string]$Root) {
    $rootItem = Get-Item -LiteralPath $Root -Force
    $items = @($rootItem) + @(Get-ChildItem -LiteralPath $Root -Recurse -Force)
    $rows = foreach ($item in ($items | Sort-Object FullName)) {
        # Directory enumeration pre-caches FileSystemInfo metadata. Refresh each
        # entry before ALL observations, including initial baseline and reparse
        # checks; never tolerate differences or rebase after the CLI starts.
        # https://learn.microsoft.com/dotnet/api/system.io.filesysteminfo.lastwritetimeutc
        $item.Refresh()
        Require ($item.Exists) 'Snapshot entry disappeared during fresh retrieval'
        Require (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) 'Reparse points are not allowed in a preview package or test tree'
        $relative = $item.FullName.Substring($rootItem.FullName.Length).TrimStart([char[]]@('\', '/'))
        $file = -not $item.PSIsContainer
        [ordered]@{
            path = $relative
            directory = $item.PSIsContainer
            attributes = [int]$item.Attributes
            length = $(if ($file) { $item.Length.ToString() } else { '0' })
            modified_utc_ticks = $item.LastWriteTimeUtc.Ticks.ToString()
            permissions_sddl = (Get-Acl -LiteralPath $item.FullName).Sddl
            sha256 = $(if ($file) { Hash-File $item.FullName } else { $null })
        }
    }
    return (ConvertTo-Json -InputObject @($rows) -Depth 8 -Compress)
}
function Read-SnapshotRows([string]$Json) {
    # PS5.1 returns a JSON array as one pipeline object; PS7 enumerates it.
    # Request no enumeration when available, then assign and explicitly iterate.
    # This preserves the top-level shape and rejects nested arrays in both hosts.
    Require ($Json.TrimStart().StartsWith('[') -and $Json.TrimEnd().EndsWith(']')) 'Snapshot must be a JSON array'
    $jsonCommand = Get-Command ConvertFrom-Json
    if ($jsonCommand.Parameters.ContainsKey('NoEnumerate')) {
        $parsed = ConvertFrom-Json -InputObject $Json -NoEnumerate
    } else {
        $parsed = ConvertFrom-Json -InputObject $Json
    }
    if ($null -eq $parsed) { return }
    foreach ($row in @($parsed)) {
        Require ($null -ne $row -and $row -is [pscustomobject]) 'Snapshot row must be an object, never a nested array'
        Require ($null -ne $row.PSObject.Properties['path'] -and $row.path -is [string]) 'Snapshot row lacks a string path'
        $row
    }
}
function Write-RawSnapshotEvidence([string]$Label, $Value) {
    $type = if ($null -eq $Value) { 'null' } else { $Value.GetType().FullName }
    $text = if ($Value -is [string]) { $Value } else { [string]$Value }
    $bytes = $Utf8.GetBytes($text)
    $hash = [Security.Cryptography.SHA256]::Create()
    try { $digest = ([BitConverter]::ToString($hash.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $hash.Dispose() }
    $limit = 262144
    $shown = [Math]::Min($bytes.Length, $limit)
    $base64 = [Convert]::ToBase64String($bytes, 0, $shown)
    Write-Host ('IMMUTABILITY_RAW ' + $Label + ' type=' + $type + ' bytes=' + $bytes.Length + ' sha256=' + $digest + ' truncated=' + ($shown -ne $bytes.Length))
    for ($offset = 0; $offset -lt $base64.Length; $offset += 4096) {
        Write-Host ('IMMUTABILITY_RAW_BASE64 ' + $Label + ' offset=' + $offset + ' ' + $base64.Substring($offset, [Math]::Min(4096, $base64.Length - $offset)))
    }
}
function Snapshot-Differences([string]$Before, [string]$After) {
    $oldRows = @(Read-SnapshotRows $Before)
    $newRows = @(Read-SnapshotRows $After)
    $oldByPath = @{}
    $newByPath = @{}
    foreach ($row in $oldRows) { $oldByPath[$row.path] = $row }
    foreach ($row in $newRows) { $newByPath[$row.path] = $row }
    $paths = @(@($oldByPath.Keys) + @($newByPath.Keys) | Sort-Object -Unique)
    foreach ($path in $paths) {
        if (-not $oldByPath.ContainsKey($path)) {
            [pscustomobject]@{ path = $path; field = 'entry'; before = $null; after = $newByPath[$path] }
        } elseif (-not $newByPath.ContainsKey($path)) {
            [pscustomobject]@{ path = $path; field = 'entry'; before = $oldByPath[$path]; after = $null }
        } else {
            foreach ($field in @('path', 'directory', 'attributes', 'length', 'modified_utc_ticks', 'permissions_sddl', 'sha256')) {
                $old = $oldByPath[$path].PSObject.Properties[$field].Value
                $new = $newByPath[$path].PSObject.Properties[$field].Value
                # JSON preserves null, boolean and string distinctions for diagnostics.
                $oldJson = ConvertTo-Json -InputObject $old -Compress
                $newJson = ConvertTo-Json -InputObject $new -Compress
                if ($oldJson -cne $newJson) {
                    [pscustomobject]@{ path = $path; field = $field; before = $old; after = $new }
                }
            }
        }
    }
}
function Assert-SnapshotEqual($Before, $After, [string]$Context) {
    if ($Before -is [string] -and $After -is [string] -and $After -ceq $Before) { return }
    # Persist and log bounded raw evidence BEFORE JSON parsing. A helper shape
    # failure must not hide the original snapshots or produce a PASS receipt.
    Write-RawSnapshotEvidence 'before' $Before
    Write-RawSnapshotEvidence 'after' $After
    $rawFailure = [ordered]@{ result = 'FAIL'; check = 'exact-filesystem-immutability'; context = $Context; before_snapshot = $Before; after_snapshot = $After }
    [IO.File]::WriteAllText($ReportPath, (ConvertTo-Json -InputObject $rawFailure -Depth 12) + "`n", $Utf8)
    Require ($Before -is [string] -and $After -is [string]) 'Snapshot return type changed; raw evidence retained'
    $changes = @(Snapshot-Differences $Before $After)
    $diagnostic = [ordered]@{
        result = 'FAIL'
        check = 'exact-filesystem-immutability'
        context = $Context
        changes = $changes
        snapshot_order_or_serialization_changed = ($changes.Count -eq 0)
        before_path_order = @(Read-SnapshotRows $Before | ForEach-Object { $_.path })
        after_path_order = @(Read-SnapshotRows $After | ForEach-Object { $_.path })
        completed_checks = @($checks.ToArray())
    }
    $failureJson = ConvertTo-Json -InputObject $diagnostic -Depth 12
    # The report is outside protected trees. Only owned fixture names and metadata
    # are logged, never audio/model contents. Failure never produces PASS evidence.
    [IO.File]::WriteAllText($ReportPath, $failureJson + "`n", $Utf8)
    Write-Host ('IMMUTABILITY_DIAGNOSTIC ' + (ConvertTo-Json -InputObject $diagnostic -Depth 12 -Compress))
    throw ($Context + ': exact filesystem snapshot changed; failure details: ' + $ReportPath)
}
function Assert-Unchanged([string]$Root, $Before, [string]$Context) {
    Assert-SnapshotEqual $Before (Snapshot $Root) $Context
}
function Verify-Manifest([string]$Root) {
    $manifest = Join-Path $Root 'SHA256SUMS.txt'
    Require (Test-Path -LiteralPath $manifest -PathType Leaf) 'Missing SHA256SUMS.txt'
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd([char[]]@('\', '/')) + [IO.Path]::DirectorySeparatorChar
    $expected = @{}
    foreach ($line in [IO.File]::ReadAllLines($manifest, $Utf8)) {
        if ($line.Length -eq 0) { continue }
        Require ($line -cmatch '^([0-9a-f]{64})  (.+)$') 'Invalid SHA256SUMS line'
        $digest = $Matches[1]
        $relative = $Matches[2]
        Require (-not $relative.Contains('\') -and -not $relative.Contains(':') -and -not [IO.Path]::IsPathRooted($relative)) 'Manifest paths must be relative POSIX paths'
        Require (@($relative.Split('/') | Where-Object { $_ -eq '' -or $_ -eq '.' -or $_ -eq '..' }).Count -eq 0) 'Unsafe manifest path segment'
        Require ($relative -ne 'SHA256SUMS.txt' -and -not $expected.ContainsKey($relative)) 'Duplicate or self-referential manifest entry'
        $path = [IO.Path]::GetFullPath((Join-Path $Root $relative))
        Require ($path.StartsWith($rootFull, [StringComparison]::OrdinalIgnoreCase)) 'Manifest path escaped package'
        Require (Test-Path -LiteralPath $path -PathType Leaf) ('Manifest file missing: ' + $relative)
        Require ((Hash-File $path) -ceq $digest) ('Manifest hash mismatch: ' + $relative)
        $expected[$relative] = $digest
    }
    Require ($expected.Count -gt 5) 'Incomplete package manifest'
    foreach ($file in @(Get-ChildItem -LiteralPath $Root -Recurse -File -Force)) {
        $relative = $file.FullName.Substring($rootFull.Length).Replace('\', '/')
        Require ($relative -eq 'SHA256SUMS.txt' -or $expected.ContainsKey($relative)) ('Unlisted package file: ' + $relative)
    }
}
function Invoke-Cli([string]$Arguments, [AllowNull()][string]$Json) {
    Assert-Unchanged $script:TestRoot $script:AllBefore ('before CLI ' + $Arguments)
    $invocationBefore = Snapshot $script:TestRoot
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $script:Executable
    $info.Arguments = $Arguments
    $info.WorkingDirectory = $script:Unrelated
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.StandardOutputEncoding = $Utf8
    $info.StandardErrorEncoding = $Utf8
    # Newer .NET exposes this property. .NET Framework hosts may not; in that
    # case Process.Start captures Console.InputEncoding for its input writer.
    $inputEncodingProperty = $info.GetType().GetProperty('StandardInputEncoding')
    if ($null -ne $inputEncodingProperty) {
        $inputEncodingProperty.SetValue($info, $Utf8, $null)
    }
    # The executable must not find build tools, Python, Git or dev DLLs via PATH.
    $info.EnvironmentVariables['PATH'] = (Join-Path $env:SystemRoot 'System32')
    foreach ($name in @('PYTHONPATH', 'PYTHONHOME', 'VIRTUAL_ENV', 'PCRE2_STATIC_DIR', 'CARGO_HOME', 'RUSTUP_HOME')) {
        $info.EnvironmentVariables.Remove($name)
    }
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    try {
        $savedInputEncoding = $null
        $restoreInputEncoding = $false
        try {
            if ($null -eq $inputEncodingProperty) {
                $savedInputEncoding = [Console]::InputEncoding
                # Raw BaseStream writes bypass character encoding. Only prevent
                # a StreamWriter auto-preamble; avoid setters on console-less
                # .NET Framework hosts whose current encoding is already safe.
                if ($savedInputEncoding.GetPreamble().Length -gt 0) {
                    [Console]::InputEncoding = $Utf8
                    $restoreInputEncoding = $true
                }
            }
            Require ($process.Start()) 'Cannot launch preview executable'
        } finally {
            if ($restoreInputEncoding) { [Console]::InputEncoding = $savedInputEncoding }
        }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if ($null -ne $Json) {
            $bytes = $Utf8.GetBytes($Json)
            $process.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
            $process.StandardInput.BaseStream.Flush()
        }
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(120000)) {
            $process.Kill()
            $process.WaitForExit()
            throw 'CLI smoke invocation timed out; no result accepted'
        }
        $out = $stdout.GetAwaiter().GetResult()
        $err = $stderr.GetAwaiter().GetResult()
        Assert-Unchanged $script:TestRoot $invocationBefore ('CLI invocation ' + $Arguments)
        return [pscustomobject]@{ code = $process.ExitCode; stdout = $out; stderr = $err }
    } finally { $process.Dispose() }
}
function New-Request($Profile) {
    return [ordered]@{
        schema_version = 2
        mode = 'Sort'
        source = $script:Source
        target = $script:Target
        settings = @{}
        ai_profile = $Profile
        personal_aliases = $null
    }
}
function Configured([string]$Directory, [string]$Id, $Limits) {
    $config = [ordered]@{ bundle_id = $Id; model_directory = $Directory }
    if ($null -ne $Limits) { $config['limits'] = $Limits }
    return @{ Configured = $config }
}
function Plan-Check([string]$Name, $Profile, [bool]$Success, [bool]$Inferred, [AllowNull()][string]$ErrorCode) {
    $before = Snapshot $script:TestRoot
    $payload = New-Request $Profile | ConvertTo-Json -Depth 20 -Compress
    $response = Invoke-Cli 'plan-files --json' $payload
    Require ($response.stdout.EndsWith("`n")) ($Name + ': response lacks newline')
    $value = ConvertFrom-Json -InputObject $response.stdout
    Require ($value.schema_version -eq 2) ($Name + ': wrong schema')
    Require ($value.result.executable -ceq $false) ($Name + ': executable result')
    Require ($value.result.inference_executed -ceq $Inferred) ($Name + ': incorrect invocation status')
    if ($Success) {
        Require ($response.code -eq 0 -and $value.status -eq 'ok' -and $null -eq $value.error -and $response.stderr.Length -eq 0) ($Name + ': expected success')
        Require ($null -ne $value.result.plan -and $value.result.plan.executable -ceq $false) ($Name + ': missing non-executable plan')
    } else {
        Require ($response.code -ne 0 -and $value.status -eq 'error' -and $response.stderr.Length -gt 0) ($Name + ': expected explicit failure')
        Require ($null -eq $value.result.plan -and $value.error.code -eq $ErrorCode) ($Name + ': partial plan or wrong error')
        Require (@($value.result.blockers) -contains $ErrorCode) ($Name + ': missing fatal blocker')
    }
    Assert-Unchanged $script:TestRoot $before ($Name + ': plan check')
    [void]$checks.Add($Name)
    return $value
}

try {
    Require ($env:OS -eq 'Windows_NT') 'This verification must run on actual Windows'
    Require ($PSVersionTable.PSVersion -ge [version]'5.1') 'Windows PowerShell 5.1 or newer is required'
    $PackageRoot = (Resolve-Path -LiteralPath $PackageRoot).Path
    $ReportPath = [IO.Path]::GetFullPath($ReportPath)
    $prefix = $PackageRoot.TrimEnd([char[]]@('\', '/')) + [IO.Path]::DirectorySeparatorChar
    Require (-not $ReportPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) 'Smoke report must be outside the input package'
    $inputBefore = Snapshot $PackageRoot
    Verify-Manifest $PackageRoot
    $hebrew = From-Points @(0x05ea, 0x05d9, 0x05e7, 0x05d9, 0x05d9, 0x05ea, 0x0020, 0x05d1, 0x05d3, 0x05d9, 0x05e7, 0x05d4)
    $temporary = Join-Path ([IO.Path]::GetTempPath()) ('original-ai-preview-' + [Guid]::NewGuid().ToString('N'))
    [void][IO.Directory]::CreateDirectory($temporary)
    $script:TestRoot = $temporary
    $copy = Join-Path $temporary ($hebrew + ' package')
    Copy-Item -LiteralPath $PackageRoot -Destination $copy -Recurse
    Verify-Manifest $copy
    $script:Executable = Join-Path $copy 'singles-sorter.exe'
    $script:Source = Join-Path $copy 'examples/source'
    $script:Target = Join-Path $copy 'examples/target'
    Require (Test-Path -LiteralPath $script:Target -PathType Container) 'Package must contain an empty examples/target directory'
    Require (@(Get-ChildItem -LiteralPath $script:Target -Force).Count -eq 0) 'Example target must be empty'
    $script:Unrelated = Join-Path $temporary 'unrelated cwd'
    [void][IO.Directory]::CreateDirectory($script:Unrelated)
    [IO.File]::WriteAllText((Join-Path $script:Unrelated 'preferences-sentinel.txt'), 'owned unchanged preference sentinel', $Utf8)
    $model = Join-Path $copy 'models/original'
    $bundleId = 'original-v14-ner-3.7.5'
    $missing = Join-Path $temporary 'missing-original'
    $corrupt = Join-Path $temporary 'corrupt-original'
    Copy-Item -LiteralPath $model -Destination $missing -Recurse
    Copy-Item -LiteralPath $model -Destination $corrupt -Recurse
    Remove-Item -LiteralPath (Join-Path $missing 'classifier-v14.weights.bin')
    $bad = Join-Path $corrupt 'classifier-v14.weights.bin'
    $bytes = [IO.File]::ReadAllBytes($bad)
    $bytes[0] = $bytes[0] -bxor 1
    [IO.File]::WriteAllBytes($bad, $bytes)
    $script:AllBefore = Snapshot $script:TestRoot

    $help = Invoke-Cli '--help' $null
    Require ($help.code -eq 0 -and $help.stdout.Contains('plan-files') -and $help.stderr.Length -eq 0) 'Help failed without developer PATH'
    [void]$checks.Add('help-without-developer-path')
    $capsResponse = Invoke-Cli 'capabilities --json' $null
    Require ($capsResponse.code -eq 0 -and $capsResponse.stderr.Length -eq 0) 'Capabilities failed'
    $caps = ConvertFrom-Json -InputObject $capsResponse.stdout
    Require ($caps.result.capabilities.native_inference -ceq $false) 'V1 must remain supplied-facts only'
    Require ($caps.result.capabilities.real_file_planning.native_inference -ceq $true) 'Wrong executable: original-ai feature is unavailable'
    Require ($caps.result.capabilities.physical_apply -ceq $false) 'Physical apply must stay unavailable'
    [void]$checks.Add('feature-capabilities')

    $configured = Plan-Check 'configured-original-plan' (Configured $model $bundleId $null) $true $true $null
    $expectedArtist = From-Points @(0x05d9, 0x05e9, 0x05e8, 0x05d0, 0x05dc, 0x0020, 0x05db, 0x05d4, 0x05df)
    $artists = @()
    foreach ($decision in @($configured.result.plan.decisions)) {
        if ($null -ne $decision.PSObject.Properties['Single']) { $artists += @($decision.Single.artists) }
    }
    Require ($artists -contains $expectedArtist) 'Authored NER fixture did not reach original positive class0 gate'
    Require (@($configured.result.plan.request.resource_ids | Where-Object { $_ -like 'original-*' }).Count -eq 4) 'Both original model identities must be bound'
    Require (@($configured.result.plan.request.adapter_versions | Where-Object { $_ -like '*pcre2-archive-sha256:*' }).Count -eq 1) 'Actual linked PCRE2 identity missing'
    $provenance = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText((Join-Path $copy 'provenance.json'), $Utf8))
    $pcre2 = [string]$provenance.pcre2_build.library_sha256
    Require ($pcre2 -cmatch '^[0-9a-f]{64}$') 'Invalid PCRE2 provenance hash'
    $runtimeIdentity = 'native-runtime-build:x86_64-pc-windows-msvc:release:pcre2-archive-sha256:' + $pcre2
    Require (@($configured.result.plan.request.adapter_versions) -ccontains $runtimeIdentity) 'Plan runtime does not match packaged build provenance'
    $resourcePrefixes = @{
        'tensors.f32' = 'original-ner-tensors:sha256:'
        'tokenizer.json' = 'original-ner-tokenizer:sha256:'
        'classifier-v14.weights.bin' = 'original-secondary-weights:sha256:'
        'classifier-v14.vocab.txt' = 'original-secondary-vocabulary:sha256:'
    }
    foreach ($name in $resourcePrefixes.Keys) {
        $hash = [string]$provenance.assets_sha256.PSObject.Properties[$name].Value
        Require ($hash -cmatch '^[0-9a-f]{64}$') ('Invalid model provenance hash: ' + $name)
        Require ((Hash-File (Join-Path $model $name)) -ceq $hash) ('Packaged model does not match provenance: ' + $name)
        Require (@($configured.result.plan.request.resource_ids) -ccontains ($resourcePrefixes[$name] + $hash)) ('Plan asset binding does not match packaged model: ' + $name)
    }
    [void]$checks.Add('plan-assets-and-linked-archive-match-provenance')
    [void](Plan-Check 'disabled-plan' 'Disabled' $true $false $null)
    [void](Plan-Check 'wrong-profile' (Configured $model 'unsupported-preview-model' $null) $false $false 'RECOGNITION_UNAVAILABLE')
    [void](Plan-Check 'missing-model-part' (Configured $missing $bundleId $null) $false $false 'RECOGNITION_UNAVAILABLE')
    [void](Plan-Check 'corrupt-model-part' (Configured $corrupt $bundleId $null) $false $false 'RECOGNITION_UNAVAILABLE')
    [void](Plan-Check 'zero-job-budget' (Configured $model $bundleId @{max_inference_calls = 0}) $false $false 'PLANNING_RESOURCE_LIMIT')
    [void](Plan-Check 'ner-runtime-limit' (Configured $model $bundleId @{ner_input_bytes = 1}) $false $true 'PLANNING_RESOURCE_LIMIT')
    $apply = Invoke-Cli 'apply --json' '{}'
    $applyValue = ConvertFrom-Json -InputObject $apply.stdout
    Require ($apply.code -eq 3 -and $applyValue.error.code -eq 'UNSUPPORTED_CAPABILITY' -and $applyValue.result.executable -ceq $false) 'Unsupported apply was not blocked'
    [void]$checks.Add('physical-apply-blocked')

    Assert-Unchanged $script:TestRoot $script:AllBefore 'Owned package/source/target/preferences final check'
    Assert-Unchanged $PackageRoot $inputBefore 'Original input package final check'
    Verify-Manifest $copy
    Verify-Manifest $PackageRoot
    [void]$checks.Add('package-source-target-permissions-content-mtimes-unchanged')
    $report = [ordered]@{
        result = 'PASS'
        executable_sha256 = Hash-File (Join-Path $PackageRoot 'singles-sorter.exe')
        sha256sums_sha256 = Hash-File (Join-Path $PackageRoot 'SHA256SUMS.txt')
        powershell = $PSVersionTable.PSVersion.ToString()
        windows = [Environment]::OSVersion.VersionString
        child_path = 'System32 only; developer tools removed'
        python_runtime_required = $false
        physical_apply = $false
        graceful_ctrl_c_verified = $false
        checks = @($checks.ToArray())
        utc = [DateTime]::UtcNow.ToString('o')
    }
    $json = ConvertTo-Json -InputObject $report -Depth 8
    [IO.File]::WriteAllText($ReportPath, $json + "`n", $Utf8)
    Write-Output $json
} finally {
    if ($null -ne $temporary -and [IO.Directory]::Exists($temporary)) {
        Remove-Item -LiteralPath $temporary -Recurse -Force
    }
}
