// Setup the nav bar
fetch('https://dubsdot.cslabs.clarkson.edu/cosi-nav.json')
    .then(res => res.json())
    .then(json => {
        let links = json.links;
        let linkList = document.querySelector('cosi-nav');
        links.forEach(link => {
            let newLink = document.createElement('li');
            newLink.innerHTML = `<a href="${link.url}">${link.name}</a>`;
            linkList.appendChild(newLink);
        });
    });

var websocket;
var authenticated = false;

var ordering = {}
ordering["forum topic"] = 1
ordering["lightning talk"] = 2
ordering["project update"] = 3
ordering["announcement"] = 4
ordering["after meeting slot"] = 5

// Ask to hide an entry
function hide(id) {
    let event = {
        "event": "Hide",
        "id": id,
    };

    websocket.send(JSON.stringify(event));
}

// Ask to create an entry
function create() {
    // Get values
    let name = document.getElementById("name").value;
    let type = document.getElementById("type").value;
    let desc = document.getElementById("desc").value;

    // Check for errors
    if (!name || !type || !desc) {
        return;
    }

    // Create event
    let event = {
        "event": "Create",
        "name": name,
        "talk_type": type,
        "desc": desc,
    };

    // Send it
    websocket.send(JSON.stringify(event));
}

// Register a websocket connection
fetch("/register")
    .then(function (response) {
        return response.json();
    })
    .then(function (result) {
        authenticated = result.authenticated;
        websocket = new WebSocket(result.url);

        websocket.onmessage = function (event) {
            let json = JSON.parse(event.data);

            if (json.event == "Show") {
                var table = document.getElementById('table');
                var rows = document.getElementById('tb').children;

                // Insert the new data into the correct location in the table
                let i = 0
                for (i = 0; i < rows.length-1; i++) {
                    // Order by talk type then by id

                    let order = ordering[rows[i].children[2].innerText];
                    let id = rows[i].children[0].innerText;

                    console.log(ordering[json.talk_type], order);
                    if (ordering[json.talk_type] < order) {
                        break;
                    }
                }

                // Building a new event object using _javascript_
                var row = table.insertRow(i+1);
                row.setAttribute("class", "event");

                var c0 = row.insertCell(0);
                c0.setAttribute("style", "display: none;");
                c0.innerHTML = json.id;

                var c1 = row.insertCell(1);
                c1.setAttribute("class", "name");
                c1.innerHTML = json.name;

                var c2 = row.insertCell(2);
                c2.setAttribute("class", "type");
                c2.innerHTML = json.talk_type;

                var c3 = row.insertCell(3);
                c3.setAttribute("class", "desc");
                c3.innerHTML = json.desc;

                var c4 = row.insertCell(4);
                c4.setAttribute("class", "actions");
                c4.innerHTML = '<button onclick="hide(' + json.id + ')"> x </button>';

            } else if (json.event == "Hide") {
                // Remove the row with matching id
                var rows = document.getElementById('tb').children;

                for (i = 0; i < rows.length-1; i++) {
                    if (json.id == rows[i].children[0].innerHTML) {
                        rows[i].remove();
                        break;
                    }
                }
            }
        }

        window.onbeforeunload = function() {
            websocket.onclose = function () {}; // disable onclose handler first
            websocket.close();
        };
    })
    .catch(function (error) {
        console.log("Error: " + error);
    });

