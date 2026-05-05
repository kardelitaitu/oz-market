param(
    [Parameter(Mandatory = $true)]
    [string]$BaseSpec,

    [Parameter(Mandatory = $true)]
    [string]$RevisionSpec
)

$output = & oasdiff breaking $BaseSpec $RevisionSpec -o ERR 2>&1
$exitCode = $LASTEXITCODE
$text = ($output | Out-String)

if ($text -match 'No changes detected' -or $text -match 'No breaking changes to report') {
    exit 0
}

if ($output) {
    $output
}

exit $exitCode
