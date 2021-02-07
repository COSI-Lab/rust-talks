# rust_talks

## Endpoints

| Request | Endpoint           | Desc                                    |
| :------ | :----------------- | :-------------------------------------- |
| GET     | /index.html        | returns talks home page                 |
| GET     | /health            | Indicates whether the service is up     |
| POST    | /register          | Registers a new client for live updates |
| DELETE  | /register/{client} | Unregisters a client                    |
| POST    | /publish           | Broadcasts events to all clients        |
| GET     | /ws                | Websocket endpoint                      |