$ErrorActionPreference = "Stop"

$targets = @(
    "crates/api",
    "crates/core",
    "crates/auth",
    "crates/ai",
    "crates/extensions"
)

$pattern = "sqlx::|PgPool|PgPoolOptions|DatabaseConnection|DatabaseTransaction|ActiveModel|Entity::find|Statement::"
$matches = & rg -n --glob "*.rs" $pattern $targets
if ($LASTEXITCODE -eq 0) {
    Write-Error "Database boundary violation detected:`n$($matches -join [Environment]::NewLine)"
}
if ($LASTEXITCODE -gt 1) {
    throw "rg failed with exit code $LASTEXITCODE"
}

Write-Host "Database boundary check passed."
