function poll() {
    console.log("Hello from JS");
    setTimeout(poll, 1000);
    fetch("/get_state").then(response => response.json()).then(data => console.log(data));
}

poll();
