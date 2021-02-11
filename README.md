![](talks-preview.png)

# rust_talks

rust_talks (Or more commonly known as just Talks) is an app to manage talks at COSI meetings.

It allows people to submit talks that they are planning on giving at upcoming meetngs. As well, it includes an export functionality to aggregate active talks to be put into a markdown format which is used for our meeting minutes.


## Build
First ensure that you've [installed rust](https://www.rust-lang.org/tools/install) then run the following
```
git clone git@github.com:COSI-Lab/rust-talks.git

cd rust-talks

cargo build --release

./target/release/rust_talks
```

## Endpoints

| Request | Endpoint           | Desc                                    |
| :------ | :----------------- | :-------------------------------------- |
| GET     | /                  | The talks homepage                      |
| GET     | /health            | Indicates whether the service is up     |
| POST    | /register          | Registers a new client for live updates |
| POST    | /authenticate      | authenticates a client                  |
| GET     | /talks             | Returns the currently visible talks     |
| GET     | /ws                | Websocket endpoint                      |
| GET     | /static/*          | Serves static files                     |

## Todos

* Replace the "Next Meeting TBD" with the next meeting
* Parse to meeting minutes