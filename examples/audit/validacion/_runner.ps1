param(
    [Parameter(Mandatory=$true)][string]$Name,
    [Parameter(Mandatory=$true)][string]$Args
)
$ErrorActionPreference = "Continue"
$logDir = "C:\Users\Frask\Documents\cls\examples\audit\validacion\_logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$clx = "C:\Users\Frask\Documents\cls\target\debug\clx.exe"
$outFile = Join-Path $logDir "$Name.out.txt"
$errFile = Join-Path $logDir "$Name.err.txt"
$exit = 0
$out = ""
$err = ""
try {
    $out = & $clx $Args 2> $errFile
    $exit = $LASTEXITCODE
} catch {
    $exit = -1
    $err = $_.Exception.Message
}
if ($err -eq "") { $err = (Get-Content $errFile -Raw -ErrorAction SilentlyContinue) }
$out | Set-Content -Encoding UTF8 $outFile
if ($err -and $err -ne "") { $err | Set-Content -Encoding UTF8 $errFile }
"=== $Name (exit=$exit) ==="
"--- STDOUT ---"
$out
"--- STDERR ---"
$err
""
