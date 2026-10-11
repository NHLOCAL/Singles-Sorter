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
$names = @('Snapshot-Differences', 'Assert-SnapshotEqual')
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
    $before = Baseline
    Assert-SnapshotEqual $before $before 'unchanged'
    Require-Test (-not [IO.File]::Exists($ReportPath)) 'Unchanged snapshot emitted failure evidence'
    $count += 1
    foreach ($field in @('path', 'directory', 'attributes', 'length', 'modified_utc_ticks', 'permissions_sddl', 'sha256')) {
        $rows = @(ConvertFrom-Json -InputObject $before)
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
    $rows = @(ConvertFrom-Json -InputObject $before)
    Expect-Failure $before (ConvertTo-Json -InputObject @($rows[0]) -Depth 5 -Compress) 'entry' $false
    Expect-Failure (ConvertTo-Json -InputObject @($rows[0]) -Depth 5 -Compress) $before 'entry' $false
    Expect-Failure $before (ConvertTo-Json -InputObject @($rows[1], $rows[0]) -Depth 5 -Compress) '' $true
    Expect-Failure $before (ConvertTo-Json -InputObject $rows -Depth 5) '' $true
    Write-Output ('PASS: ' + $count + ' exact snapshot diagnostic tests; no fields ignored or rebased')
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}
