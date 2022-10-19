# Build everything as release build and copy to release folder

Write-Host "Removing old release..." -ForegroundColor white -BackgroundColor blue
if (Test-Path release) { Remove-Item release -Recurse }

Write-Host "Copy spotify.json..." -ForegroundColor white -BackgroundColor blue
copy "shared/spotify.json" "frontend/src/spotify.json"

Write-Host "Build backend..." -ForegroundColor white -BackgroundColor blue
cargo build --release --manifest-path=backend/Cargo.toml

Write-Host "Build frontend..." -ForegroundColor white -BackgroundColor blue
npm run build --prefix frontend

Write-Host "Copy files..." -ForegroundColor white -BackgroundColor blue
mkdir release
mkdir release/files
copy "shared/spotify.json" "release/spotify.json"
copy backend/target/release/music-quiz.exe release/music-quiz.exe
Copy-Item -Path "frontend/build/*" -Destination "release/files" -Recurse
New-Item "release/start.bat" -ItemType File -Value "music-quiz.exe"

Write-Host "Finished!" -ForegroundColor white -BackgroundColor DarkGreen
pause
