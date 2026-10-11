# Pure snapshot-comparison tests; Windows CI runs this in Windows PowerShell 5.1.
# Loading only reviewed helper definitions does not run the application smoke.
Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$Utf8 = New-Object System.Text.UTF8Encoding($false, $true)
$checks = New-Object 'System.Collections.Generic.List[string]'
$scriptFile = Join-Path $PSScriptRoot 'smoke.ps1'
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($scriptFile, [ref]$tokens, [ref]$parseErrors)
if (@($parseErrors).Count -ne 0) { throw ('Smoke script parse failure: ' + ($parseErrors | Out-String)) }
$names = @('Require', 'Hash-File', 'Snapshot', 'Read-SnapshotRows', 'Write-RawSnapshotEvidence', 'Snapshot-Differences', 'Assert-SnapshotEqual')
$functions = @($ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $names -contains $node.Name }, $false))
if ($functions.Count -ne $names.Count) { throw 'Expected snapshot helper definitions not found' }
foreach ($function in $functions) { . ([scriptblock]::Create($function.Extent.Text)) }
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('snapshot-unit-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($temporary)
$ReportPath = Join-Path $temporary 'failure.json'
$count = 0
function Require-Test([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}
function Test-CachedMetadataRefresh {
    $root = Join-Path $temporary 'cached metadata fixture'
    [void][IO.Directory]::CreateDirectory($root)
    $directory = Join-Path $root 'owned directory'
    [void][IO.Directory]::CreateDirectory($directory)
    # Deliberately old metadata ONLY on this owned unit fixture makes the cache
    # regression deterministic. Production smoke never sets any timestamp.
    $oldTime = [DateTime]::new(2020, 1, 1, 0, 0, 0, [DateTimeKind]::Utc)
    $newTime = $oldTime.AddDays(1)
    [IO.Directory]::SetLastWriteTimeUtc($directory, $oldTime)
    $script:CachedEntry = Get-Item -LiteralPath $directory -Force
    $cachedTicks = $script:CachedEntry.LastWriteTimeUtc.Ticks
    [IO.Directory]::SetLastWriteTimeUtc($directory, $newTime)
    Require-Test ($script:CachedEntry.LastWriteTimeUtc.Ticks -eq $cachedTicks) 'Regression fixture did not retain cached metadata'
    function Get-ChildItem {
        param([string]$LiteralPath, [switch]$Recurse, [switch]$Force)
        $script:CachedEntry
    }
    $rows = @(Read-SnapshotRows (Snapshot $root))
    $observed = @($rows | Where-Object { $_.path -eq 'owned directory' })
    Require-Test ($observed.Count -eq 1 -and $observed[0].modified_utc_ticks -ceq $newTime.Ticks.ToString()) 'Snapshot used stale enumerated directory metadata'
    $script:count += 1
    [IO.Directory]::Delete($directory)
    $failed = $false
    try { [void](Snapshot $root) } catch {
        Require-Test ($_.Exception.Message.Contains('Snapshot entry disappeared')) 'Unexpected missing-entry failure'
        $failed = $true
    }
    Require-Test $failed 'Disappeared cached entry produced a sentinel snapshot'
    $script:count += 1
}
function Test-LiveSetupSnapshot {
    $root = Join-Path $temporary 'live setup fixture'
    [void][IO.Directory]::CreateDirectory($root)
    $cwd = Join-Path $root 'unrelated cwd'
    [void][IO.Directory]::CreateDirectory($cwd)
    [IO.File]::WriteAllText((Join-Path $cwd 'preferences-sentinel.txt'), 'owned unchanged fixture', $Utf8)
    $source = Join-Path $root 'original'
    [void][IO.Directory]::CreateDirectory($source)
    [IO.File]::WriteAllBytes((Join-Path $source 'weights.bin'), [byte[]]@(0, 1, 2))
    [IO.File]::WriteAllText((Join-Path $source 'manifest.txt'), 'owned fixture', $Utf8)
    $missing = Join-Path $root 'missing-original'
    Copy-Item -LiteralPath $source -Destination $missing -Recurse
    Remove-Item -LiteralPath (Join-Path $missing 'weights.bin')
    # All synchronous setup calls have returned. Freeze once, then immediately
    # compare with fresh retrieval; no sleep, retry or baseline replacement.
    $baseline = Snapshot $root
    Assert-SnapshotEqual $baseline (Snapshot $root) 'owned-live-setup-stability'
    $script:count += 1
}
function Baseline {
    return ConvertTo-Json -InputObject @(
        [ordered]@{ path = ''; directory = $true; attributes = 16; length = '0'; modified_utc_ticks = '1'; permissions_sddl = 'owned-root-acl'; sha256 = $null },
        [ordered]@{ path = 'owned.wav'; directory = $false; attributes = 32; length = '3'; modified_utc_ticks = '2'; permissions_sddl = 'owned-file-acl'; sha256 = ('a' * 64) }
    ) -Depth 5 -Compress
}
function Expect-Failure([string]$Before, [string]$After, [string]$Field, [bool]$OrderingOnly) {
    $failed = $false
    try { Assert-SnapshotEqual $Before $After 'owned-unit-fixture' }
    catch {
        Require-Test ($_.Exception.Message.Contains('exact filesystem snapshot changed')) 'Unexpected failure while reporting snapshot'
        $failed = $true
    }
    Require-Test $failed 'Changed snapshot incorrectly accepted'
    $report = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($ReportPath, $Utf8))
    Require-Test ($report.result -eq 'FAIL') 'Diagnostic must never be a passing smoke receipt'
    Require-Test ($report.snapshot_order_or_serialization_changed -eq $OrderingOnly) 'Wrong diagnostic difference classification'
    if (-not $OrderingOnly) { Require-Test (@($report.changes | Where-Object { $_.field -eq $Field }).Count -gt 0) ('Missing exact changed field: ' + $Field) }
    $script:count += 1
}
try {
    # ACL integration is exercised on Windows. Linux's PowerShell lacks Get-Acl;
    # use a declared fixture-only stand-in there for metadata-cache unit coverage.
    if ($env:OS -ne 'Windows_NT') {
        function Get-Acl { param([string]$LiteralPath) [pscustomobject]@{ Sddl = 'owned-linux-unit-acl-placeholder' } }
    }
    Test-CachedMetadataRefresh
    Test-LiveSetupSnapshot
    $modes = @('native')
    if ($PSVersionTable.PSVersion.Major -ge 7) { $modes += 'legacy-array-output-simulation' }
    foreach ($mode in $modes) {
        if ($mode -eq 'legacy-array-output-simulation') {
            function ConvertFrom-Json {
                param([string]$InputObject)
                Microsoft.PowerShell.Utility\ConvertFrom-Json -InputObject $InputObject -NoEnumerate
            }
        }
        if ([IO.File]::Exists($ReportPath)) { Remove-Item -LiteralPath $ReportPath }
    $before = Baseline
    Require-Test (@(Read-SnapshotRows '[]').Count -eq 0) 'Empty snapshot became one null/nested row'
    $count += 1
    $parsedRows = @(Read-SnapshotRows $before)
    Require-Test ($parsedRows.Count -eq 2 -and $parsedRows[0] -is [pscustomobject] -and $parsedRows[1] -is [pscustomobject] -and $parsedRows[1].path -eq 'owned.wav') 'Multiple snapshot rows were not enumerated'
    $count += 1
    $single = ConvertTo-Json -InputObject @($parsedRows[0]) -Depth 5 -Compress
    Require-Test (@(Read-SnapshotRows $single).Count -eq 1) 'Single snapshot row lost array shape'
    $count += 1
    $nestedRejected = $false
    try { [void]@(Read-SnapshotRows ('[' + $before + ']')) } catch { $nestedRejected = $true }
    Require-Test $nestedRejected 'Nested snapshot arrays accepted'
    $count += 1
    Assert-SnapshotEqual $before $before 'unchanged'
    Require-Test (-not [IO.File]::Exists($ReportPath)) 'Unchanged snapshot emitted failure evidence'
    $count += 1
    foreach ($field in @('path', 'directory', 'attributes', 'length', 'modified_utc_ticks', 'permissions_sddl', 'sha256')) {
        $rows = @(Read-SnapshotRows $before)
        switch ($field) {
            'path' { $rows[1].path = 'OWNED.wav' }
            'directory' { $rows[1].directory = $true }
            'attributes' { $rows[1].attributes = 33 }
            'length' { $rows[1].length = '4' }
            'modified_utc_ticks' { $rows[1].modified_utc_ticks = '3' }
            'permissions_sddl' { $rows[1].permissions_sddl = 'changed-owned-acl' }
            'sha256' { $rows[1].sha256 = ('b' * 64) }
        }
        Expect-Failure $before (ConvertTo-Json -InputObject $rows -Depth 5 -Compress) $field $false
    }
    $rows = @(Read-SnapshotRows $before)
    Expect-Failure $before (ConvertTo-Json -InputObject @($rows[0]) -Depth 5 -Compress) 'entry' $false
    Expect-Failure (ConvertTo-Json -InputObject @($rows[0]) -Depth 5 -Compress) $before 'entry' $false
    Expect-Failure $before (ConvertTo-Json -InputObject @($rows[1], $rows[0]) -Depth 5 -Compress) '' $true
    Expect-Failure $before (ConvertTo-Json -InputObject $rows -Depth 5) '' $true
    $failed = $false
    try { Assert-SnapshotEqual $before '[ invalid JSON ]' 'parse-failure-evidence' } catch { $failed = $true }
    Require-Test $failed 'Malformed changed snapshot accepted'
    $raw = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($ReportPath, $Utf8))
    Require-Test ($raw.result -eq 'FAIL' -and $raw.before_snapshot -ceq $before -and $raw.after_snapshot -ceq '[ invalid JSON ]') 'Parser failure hid raw snapshots'
    $count += 1
    $failed = $false
    try { Assert-SnapshotEqual @('a', 'b') 'a b' 'type-failure-evidence' } catch { $failed = $true }
    Require-Test $failed 'Unexpected snapshot return type was coerced into acceptance'
    $raw = ConvertFrom-Json -InputObject ([IO.File]::ReadAllText($ReportPath, $Utf8))
    Require-Test ($raw.result -eq 'FAIL' -and @($raw.before_snapshot).Count -eq 2) 'Type failure hid original value'
    $count += 1
        if ($mode -eq 'legacy-array-output-simulation') { Remove-Item Function:\ConvertFrom-Json }
    }
    Write-Output ('PASS: ' + $count + ' exact snapshot diagnostic tests across ' + $modes.Count + ' JSON output modes; no fields ignored or rebased')
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}
