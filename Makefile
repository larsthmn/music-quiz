# Could be done better I guess, but whatever
# Copys spotify.json to the backend and frontend dirs and changes the configured port to either 3000 or 80

clean:
	powershell "if (Test-Path frontend/src/spotify.json) { Remove-Item frontend/src/spotify.json }"
	powershell "if (Test-Path backend/spotify.json) { Remove-Item backend/spotify.json }"
	powershell "if (Test-Path backend/target/release/rust-backend.exe) { Remove-Item backend/target/release/rust-backend.exe }"
	powershell "if (Test-Path frontend/build) { Remove-Item frontend/build -Recurse }"
	powershell "if (Test-Path release) { Remove-Item release -Recurse }"
	
.PHONY: release	debug

release: clean 
	powershell "((Get-Content -path shared/spotify_template.json -Raw) -replace '\[PORT\]','80') | Set-Content -Path frontend/src/spotify.json"
	cargo build --release --manifest-path=backend/Cargo.toml
	npm run build --prefix frontend
	powershell "mkdir release"
	powershell "mkdir release/public"
	powershell "((Get-Content -path shared/spotify_template.json -Raw) -replace '\[PORT\]','80') | Set-Content -Path release/spotify.json"
	powershell "copy backend/target/release/rust-backend.exe release/rust-backend.exe"
	powershell "copy backend/rocket.toml release/rocket.toml"
	powershell "Copy-Item -Path "frontend/build/*" -Destination "release/public" -Recurse"
	
debug: clean
	powershell "((Get-Content -path shared/spotify_template.json -Raw) -replace '\[PORT\]','3000') | Set-Content -Path backend/spotify.json"
	powershell "((Get-Content -path shared/spotify_template.json -Raw) -replace '\[PORT\]','3000') | Set-Content -Path frontend/src/spotify.json"