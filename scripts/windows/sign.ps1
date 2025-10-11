param(
    [Parameter(Mandatory = $true)][string]$InputPath,
    [Parameter(Mandatory = $true)][string]$CertificatePath,
    [Parameter(Mandatory = $false)][string]$CertificatePassword,
    [Parameter(Mandatory = $false)][string]$TimestampUrl = "http://timestamp.digicert.com"
)

if (-not (Test-Path $InputPath)) {
    Write-Error "Input artifact '$InputPath' was not found."
}

if (-not (Test-Path $CertificatePath)) {
    Write-Error "Signing certificate '$CertificatePath' was not found."
}

$arguments = @(
    "sign",
    "/fd", "SHA256",
    "/tr", $TimestampUrl,
    "/td", "SHA256",
    "/f", $CertificatePath
)

if ($CertificatePassword) {
    $arguments += @("/p", $CertificatePassword)
}

$arguments += $InputPath

Write-Host "Running: signtool $arguments"
& signtool @arguments
if ($LASTEXITCODE -ne 0) {
    throw "signtool failed with exit code $LASTEXITCODE"
}
