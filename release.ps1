# Build everything as release build and copy to release folder

echo "Removing old release..."
if (Test-Path release) { Remove-Item release -Recurse }

echo "Copy spotify.json..."
copy "shared/spotify.json" "frontend/src/spotify.json"

echo "Build backend..."
cargo build --release --manifest-path=backend/Cargo.toml

echo "Build frontend..."
npm run build --prefix frontend

echo "Copy files..."
mkdir release
mkdir release/files
copy "shared/spotify.json" "release/spotify.json"
copy backend/target/release/music-quiz.exe release/music-quiz.exe
Copy-Item -Path "frontend/build/*" -Destination "release/files" -Recurse

echo "Finished"
pause
