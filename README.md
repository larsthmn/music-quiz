# Music Quiz

## Development

Execute `copy-shared.bat` to copy shared files to frontend and backend directories. \

Change to the `web` directory and start the frontend with
```
npm start
```
This runs the app in the development mode.
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.
The page will reload if you make edits.
You will also see any lint errors in the console.

Compile and start the rust server in debug build. This starts the backend on port 8000 to handle all API calls.

While developing, routes are first handles by React and forwarded to its configured proxy (localhost:8000 and thereby
to the Rust backend) if React doesn't know the route.
In release build, all routes are served by the Rust application. The rust app has a wildcard serving all routes with
the index.html file with a lower rank than all other routes.

## Deployment

Compile the rust app as release build with 
```
cargo build --release
```
and copy the executable to the destination folder. 
Then go to `web` folder and type 
```
npm run build
```
to build the frontend app for production with to the `build` folder. \
Copy its contents (index.html, folder 'static', manifest.json etc.) to a folder called `public` in the destination 
folder.
Start the `rust-backend.exe` in some terminal and connect to the IP of the host computer with your clients.

### 