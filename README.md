# Music Quiz

## Dependencies

This app is developed on Windows, so we stick with the GNU make port and powershell commands in the Makefile. 
Install `make` with
```
choco install make
```
To use it under Linux, you probably need to rewrite the Makefile with bash commands or execute the stuff in it manually.

## Development

Put all spotify credentials into shared/spotify.json. 
Leave `[PORT]` as it is since it's replaced automatically when copying the file for debug or release.
Use 
```
make debug
```
to copy the spotify config to the frontend and backend folders. 
This has only to be done once at the setup and when the original spotify.json is changed.

First, run `cargo test` to generate the TypeScript API objects into the `shared` directory.

Compile and start the rust server in debug build with the option `-p 8000 -a 127.0.0.1`.
This starts the backend locally on port 8000 to handle all API calls.

Change to the `frontend` directory and start the frontend with
```
npm start
```
This runs the app in the development mode.
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.
The page will reload if you make edits.
You will also see any lint errors in the console.

While developing, routes are first handles by React and forwarded to its configured proxy (localhost:8000 and thereby
to the Rust backend) if React doesn't know the route.
In release build, all routes are served by the Rust application via port 80. 
The rust app has a wildcard serving all routes with the index.html file with a lower rank than all other routes.

## Deployment

Run
```
make release
```
This copies the spotify.json with port 80 to the folders, compiles the frontend and backend and copies all files to
the release folder. 
Take the whole release folder for distribution.
Start the `rust-backend.exe` in some terminal and connect to the IP of the host computer with your clients.
