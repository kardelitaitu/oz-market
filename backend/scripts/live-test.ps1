param(
    [string]$DatabaseUrl = "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable",
    [string]$Bind = "127.0.0.1:3000",
    [switch]$SkipBuild,
    [switch]$SkipCleanup
)

$BaseUrl = "http://$Bind"
$Pass = 0
$Fail = 0

function Write-Step($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Result($ok, $msg) {
    if ($ok) { $script:Pass++; Write-Host "  PASS  $msg" -ForegroundColor Green }
    else { $script:Fail++; Write-Host "  FAIL  $msg" -ForegroundColor Red }
}

function Invoke-Api($method, $path, $body, $claims, $expectedStatus) {
    $headers = @{ "Content-Type" = "application/json" }
    if ($claims) { $headers["x-marketplace-claims"] = ($claims | ConvertTo-Json -Compress) }
    $params = @{ Uri = "$BaseUrl$path"; Method = $method; Headers = $headers }
    if ($body) { $params["Body"] = ($body | ConvertTo-Json -Depth 10 -Compress) }
    try {
        $resp = Invoke-RestMethod @params -StatusCodeVariable statusCode
        if ($statusCode -eq $expectedStatus) { return $resp, $statusCode, $null }
        else { return $resp, $statusCode, "expected $expectedStatus got $statusCode" }
    } catch {
        try {
            $code = [int]$_.Exception.Response.StatusCode
            if ($code -eq $expectedStatus) { return $null, $code, $null }
            return $null, $code, "expected $expectedStatus got ${code}: $($_.Exception.Message)"
        } catch { return $null, -1, "exception: $($_.Exception.Message)" }
    }
}

# ── Claims fixtures ──
$Seller = @{
    sub = "seller-1"; roles = @("seller_listing_writer", "seller_negotiator", "seller_contact_reveal_approver")
    scopes = @("listing:create", "listing:read", "listing:search", "negotiation:create", "negotiation:read",
               "negotiation:offer:submit", "negotiation:reveal:request", "reveal:approve")
    seller_account_id = "seller-1"; buyer_agent_id = $null; hardware_id = $null; exp = 1900000000
}
$Buyer = @{
    sub = "buyer-1"; roles = @("buyer_negotiator")
    scopes = @("negotiation:create", "negotiation:read", "negotiation:offer:submit", "negotiation:reveal:request")
    seller_account_id = $null; buyer_agent_id = "buyer-1"; hardware_id = $null; exp = 1900000000
}
$Admin = @{
    sub = "admin-1"; roles = @("admin"); scopes = @("*")
    seller_account_id = $null; buyer_agent_id = $null; hardware_id = $null; exp = 1900000000
}

# ── Request fixtures ──
function New-ListingBody($suffix) {
    return @{
        idempotency_key = "create-live-$suffix"
        listing = @{
            schema_version = "1.0"; owner_id = "seller-1"
            listing_type = "product"; category = "laptop"
            title = "Live Test Laptop $suffix"; condition = "new"
            price = @{ currency = "USD"; amount = 999.99 }
            location = @{ country_code = "US"; country_name = "United States"; city = "New York" }
            picture_urls = @("https://example.com/laptop.jpg")
            description = "Live testing laptop listing $suffix"
            attributes = $null
        }
    }
}

# ══════════════════════════════════════════════
# SETUP
# ══════════════════════════════════════════════
Write-Step "PREREQUISITES"

Write-Host "  DATABASE_URL=$DatabaseUrl"
Write-Host "  BIND=$Bind"

Write-Step "BUILD"
if (-not $SkipBuild) {
    Write-Host "  Building oz-market-server (release)..."
    cargo build --release --package oz-market-server 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Host "  Build failed"; exit 1 }
    Write-Host "  Build OK"
} else { Write-Host "  Skipping build" }

$ServerBin = Join-Path (Get-Item $PSScriptRoot).Parent.Parent.FullName "backend" "target" "release" "oz-market-server.exe"
if (-not (Test-Path $ServerBin)) { Write-Host "  Binary not found at $ServerBin"; exit 1 }

Write-Step "START SERVER"
$serverEnv = @{ DATABASE_URL = $DatabaseUrl; MARKETPLACE_BIND = $Bind; RUST_LOG = "info" }
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $ServerBin
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
foreach ($e in $serverEnv.GetEnumerator()) { $psi.EnvironmentVariables[$e.Key] = $e.Value }
$proc = New-Object System.Diagnostics.Process
$proc.StartInfo = $psi
$proc.Start() | Out-Null
Write-Host "  Server PID: $($proc.Id)"

Write-Step "WAIT FOR HEALTH"
$healthy = $false
for ($i = 0; $i -lt 30; $i++) {
    try { $r = Invoke-RestMethod "$BaseUrl/health" -TimeoutSec 2; if ($r.status -eq "ok") { $healthy = $true; break } }
    catch { Start-Sleep -Seconds 1 }
}
if (-not $healthy) { Write-Host "  Server failed to start"; Stop-Process $proc.Id; exit 1 }
Write-Host "  Health OK"

# ══════════════════════════════════════════════
# TESTS
# ══════════════════════════════════════════════
Write-Step "1. HEALTH & METADATA"

$resp, $code, $err = Invoke-Api GET "/health" -expectedStatus 200
Write-Result ($code -eq 200) "GET /health => $code"

$resp, $code, $err = Invoke-Api GET "/api-docs/openapi.json" -expectedStatus 200
Write-Result ($code -eq 200) "GET /api-docs/openapi.json => $code"

# ─────────────────────────────────────────
Write-Step "2. CREATE LISTING (SELLER)"

$suffix = [DateTimeOffset]::Now.ToUnixTimeMilliseconds()
$createBody = New-ListingBody $suffix
$resp, $code, $err = Invoke-Api POST "/v1/listings" $createBody $Seller 201
Write-Result ($code -eq 201) "POST /v1/listings => $code"
if ($code -eq 201 -and $resp.listing_id) { $ListingId = $resp.listing_id; Write-Host "  listing_id=$ListingId" }
else { $ListingId = "lst_live_$suffix" }

$resp, $code, $err = Invoke-Api POST "/v1/listings" $createBody $Seller 200
Write-Result ($code -eq 200) "POST /v1/listings (idempotent replay) => $code"

# ─────────────────────────────────────────
Write-Step "3. GET LISTING (PUBLIC)"

$resp, $code, $err = Invoke-Api GET "/v1/listings/$ListingId" -expectedStatus 200
Write-Result ($code -eq 200) "GET /v1/listings/{id} => $code"

$resp, $code, $err = Invoke-Api GET "/v1/listings/LST_DOES_NOT_EXIST" -expectedStatus 404
Write-Result ($code -eq 404) "GET /v1/listings/NONEXISTENT => $code"

# ─────────────────────────────────────────
Write-Step "4. SEARCH (PUBLIC)"

$resp, $code, $err = Invoke-Api GET "/v1/listings/search?q=laptop&limit=5" -expectedStatus 200
Write-Result ($code -eq 200) "GET /v1/listings/search?q=laptop => $code"

# ─────────────────────────────────────────
Write-Step "5. OPEN NEGOTIATION (BUYER)"

$negBody = @{
    idempotency_key = "open-live-$suffix"; listing_id = $ListingId
    buyer_agent_id = "buyer-1"; offer_currency = "USD"; offer_amount = 750.00
}
$resp, $code, $err = Invoke-Api POST "/v1/negotiations" $negBody $Buyer 201
Write-Result ($code -eq 201) "POST /v1/negotiations => $code"
if ($code -eq 201 -and $resp.negotiation_id) { $NegId = $resp.negotiation_id; Write-Host "  negotiation_id=$NegId" }

$resp, $code, $err = Invoke-Api POST "/v1/negotiations" $negBody $Buyer 200
Write-Result ($code -eq 200) "POST /v1/negotiations (idempotent replay) => $code"

$badNegBody = @{ idempotency_key = "open-zero-$suffix"; listing_id = $ListingId
    buyer_agent_id = "buyer-1"; offer_currency = "USD"; offer_amount = 0 }
$resp, $code, $err = Invoke-Api POST "/v1/negotiations" $badNegBody $Buyer 400
Write-Result ($code -eq 400) "POST /v1/negotiations (zero amount => 400) => $code"

# ─────────────────────────────────────────
Write-Step "6. GET NEGOTIATION STATUS"

$resp, $code, $err = Invoke-Api GET "/v1/negotiations/$NegId" $null $Buyer 200
Write-Result ($code -eq 200) "GET /v1/negotiations/{id} => $code"

# ─────────────────────────────────────────
Write-Step "7. SUBMIT OFFER (SELLER)"

$offerBody = @{ idempotency_key = "offer-live-$suffix"; offer_currency = "USD"; offer_amount = 800.00 }
$resp, $code, $err = Invoke-Api POST "/v1/negotiations/$NegId/offers" $offerBody $Seller 200
Write-Result ($code -eq 200) "POST .../offers (seller counter) => $code"

# ─────────────────────────────────────────
Write-Step "8. ACCEPT NEGOTIATION (SELLER)"

$acceptBody = @{ idempotency_key = "accept-live-$suffix" }
$resp, $code, $err = Invoke-Api POST "/v1/negotiations/$NegId/accept" $acceptBody $Seller 200
Write-Result ($code -eq 200) "POST .../accept => $code"

# ─────────────────────────────────────────
Write-Step "9. REJECT NEGOTIATION PATH"

# Seed a second negotiation for reject test
$suffix2 = "$suffix-reject"
$ListingId2 = "lst_reject_$suffix2"
$createBody2 = New-ListingBody $suffix2
$resp, $code, $err = Invoke-Api POST "/v1/listings" $createBody2 $Seller 201
if ($code -eq 201 -and $resp.listing_id) { $ListingId2 = $resp.listing_id }

$negBody2 = @{
    idempotency_key = "open-reject-$suffix2"; listing_id = $ListingId2
    buyer_agent_id = "buyer-1"; offer_currency = "USD"; offer_amount = 500.00
}
$resp, $code, $err = Invoke-Api POST "/v1/negotiations" $negBody2 $Buyer 201
if ($code -eq 201 -and $resp.negotiation_id) { $NegId2 = $resp.negotiation_id }

$rejectBody = @{ idempotency_key = "reject-live-$suffix2" }
$resp, $code, $err = Invoke-Api POST "/v1/negotiations/$NegId2/reject" $rejectBody $Seller 200
Write-Result ($code -eq 200) "POST .../reject => $code"

# ─────────────────────────────────────────
Write-Step "10. CONTACT REVEAL FLOW"

# Seed listing, negotiation, reservation for reveal test
$suffix3 = "$suffix-reveal"
$ListingId3 = "lst_reveal_$suffix3"
$createBody3 = New-ListingBody $suffix3
$resp, $code, $err = Invoke-Api POST "/v1/listings" $createBody3 $Seller 201
if ($code -eq 201 -and $resp.listing_id) { $ListingId3 = $resp.listing_id }

$negBody3 = @{
    idempotency_key = "open-reveal-$suffix3"; listing_id = $ListingId3
    buyer_agent_id = "buyer-1"; offer_currency = "USD"; offer_amount = 600.00
}
$resp, $code, $err = Invoke-Api POST "/v1/negotiations" $negBody3 $Buyer 201
Write-Result ($code -eq 201) "POST .../negotiations (reveal setup) => $code"
if ($code -eq 201 -and $resp.negotiation_id) { $NegId3 = $resp.negotiation_id }

# Request contact reveal (buyer)
$revealBody = @{ idempotency_key = "reveal-live-$suffix3" }
$resp, $code, $err = Invoke-Api POST "/v1/negotiations/$NegId3/request-contact-reveal" $revealBody $Buyer 202
Write-Result ($code -eq 202) "POST .../request-contact-reveal => $code"
if ($code -eq 202 -and $resp.reveal_id) { $RevealId = $resp.reveal_id; Write-Host "  reveal_id=$RevealId" }

# Approve contact reveal (seller)
$resp, $code, $err = Invoke-Api POST "/v1/contact-reveals/$RevealId/approve" $null $Seller 200
Write-Result ($code -eq 200) "POST .../contact-reveals/{id}/approve => $code"

# ─────────────────────────────────────────
Write-Step "11. UNAUTHORIZED / ERROR CASES"

# No auth header
try {
    Invoke-RestMethod -Uri "$BaseUrl/v1/listings" -Method POST -ContentType "application/json" `
        -Body '{"idempotency_key":"x","listing":{"schema_version":"1.0","owner_id":"x","listing_type":"product","title":"x","description":"x","price":{"currency":"USD","amount":1},"location":{"country_code":"US","country_name":"US","city":"x"}}}' `
        -TimeoutSec 5 -StatusCodeVariable sc
    Write-Result ($sc -eq 401) "POST /v1/listings (no auth => $sc)"
} catch { Write-Result ($_.Exception.Response.StatusCode -eq 401) "POST /v1/listings (no auth => unauthorized)" }

# Wrong role (buyer tries to create listing)
$resp, $code, $err = Invoke-Api POST "/v1/listings" $createBody $Buyer 403
Write-Result ($code -eq 403) "POST /v1/listings (buyer => 403) => $code"

# Rate limit — rapid requests
Write-Host "  Testing rate limit (sending rapid requests)..."
$limited = $false
for ($i = 0; $i -lt 25; $i++) {
    $body = @{ idempotency_key = "rl-live-$suffix-$i"; listing_id = $ListingId3
        buyer_agent_id = "buyer-1"; offer_currency = "USD"; offer_amount = 100.00 }
    $resp, $code, $err = Invoke-Api POST "/v1/negotiations" $body $Buyer 429
    if ($code -eq 429) { $limited = $true; break }
}
Write-Result ($limited) "Rate limiting triggers 429"

# ══════════════════════════════════════════════
# RESULTS
# ══════════════════════════════════════════════
Write-Step "RESULTS"
Write-Host "  $Pass passed, $Fail failed" -ForegroundColor $(if ($Fail -eq 0) { "Green" } else { "Red" })

# ══════════════════════════════════════════════
# CLEANUP
# ══════════════════════════════════════════════
if (-not $SkipCleanup) {
    Write-Step "CLEANUP"
    Write-Host "  Stopping server (PID $($proc.Id))..."
    try { Stop-Process $proc.Id -Force -ErrorAction SilentlyContinue } catch {}
    Write-Host "  Done"
}

exit $Fail
