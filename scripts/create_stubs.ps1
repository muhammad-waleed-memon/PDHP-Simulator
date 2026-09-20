$base = "d:\Dr Fariya Memon\backend\src"
$mods = @("users","patients","doctors","facilities","encounters","conditions","allergies","medications","prescriptions","laboratories","consent","audit","clinical_alerts","fhir")
$files = @("mod.rs","routes.rs","service.rs","model.rs","dto.rs")

foreach ($mod in $mods) {
    $dir = Join-Path $base $mod
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    foreach ($f in $files) {
        $path = Join-Path $dir $f
        $content = "// $mod - $f - placeholder (implemented in later phases)"
        Set-Content -Path $path -Value $content -Encoding UTF8
    }
}
Write-Host "Done: all module stubs created"
