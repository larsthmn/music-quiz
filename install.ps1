# Copy spotify config files to backend and frontend 

Write-Host "Copy spotify configs..." -ForegroundColor white -BackgroundColor blue
if (Test-Path frontend/src/spotify_devel.json) { Remove-Item frontend/src/spotify_devel.json }
if (Test-Path backend/spotify_devel.json) { Remove-Item backend/spotify_devel.json }
copy "shared/spotify_devel.json" "frontend/src/spotify_devel.json"
copy "shared/spotify_devel.json" "backend/spotify_devel.json"

Write-Host "Run cargo test to build shared objects" -ForegroundColor white -BackgroundColor blue
cargo test --manifest-path=backend/Cargo.toml

Write-Host "Prepared everything for development" -ForegroundColor white -BackgroundColor DarkGreen
pause