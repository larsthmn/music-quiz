# Copy spotify config files to backend and frontend 

echo "Copy spotify configs..."
if (Test-Path frontend/src/spotify_devel.json) { Remove-Item frontend/src/spotify_devel.json }
if (Test-Path backend/spotify_devel.json) { Remove-Item backend/spotify_devel.json }
copy "shared/spotify_devel.json" "frontend/src/spotify_devel.json"
copy "shared/spotify_devel.json" "backend/spotify_devel.json"

echo "Run cargo test to build shared objects"
cargo test --manifest-path=backend/Cargo.toml

echo "Prepared everything for development"
pause