$ErrorActionPreference = "Stop"
$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"

Push-Location $PSScriptRoot
try {
    flet build windows --product "Singles Sorter" --no-rich-output
    if ($LASTEXITCODE -ne 0) {
        throw "Flet Windows build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
