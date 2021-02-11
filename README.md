# rust_talks

## Endpoints

| Request | Endpoint           | Desc                                    |
| :------ | :----------------- | :-------------------------------------- |
| GET     | /health            | Indicates whether the service is up     |
| POST    | /register          | Registers a new client for live updates |
| POST    | /authenticate      | authenticates a client                  |
| GET     | /ws                | Websocket endpoint                      |