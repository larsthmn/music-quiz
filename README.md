# Music Quiz

## General

This app is developed on Windows and powershell scripts are used.
It can be deployed on GNU/Linux, but you probably need to rewrite the scripts with bash commands or execute the stuff in it manually.
Hasn't been tested yet though.

## Development

Put all spotify credentials into `shared/spotify.json` and `shared/spotify_devel.json`. 

Use the powershell script `install.ps1` to set up everything for development.

Then compile and start the rust server in debug build with the options
```
-p 8000 -a 127.0.0.1 -s spotify_devel.json
```
This starts the backend locally on port 8000 to handle all API calls.
The redirect URL has port 3000 when developing and 80 in production, thereby different spotify configs are needed. 

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
The rust app has a fallback route that serves non-API-routes with the index.html file.

## Deployment

Run the powershell script
```
release.ps1
```
This compiles the frontend and backend and copies all files to the release folder. 
Take the whole release folder for distribution.
Start the `music-quiz.exe` in some terminal and connect to the IP of the host computer with your clients.
