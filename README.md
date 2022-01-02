# socketeer

A websocket manager that is designed to emit events via a internal REST api.

Clients are watched and checked for lagging and fall behind nature allowing reasonable limits in order to recover otherwise terminating the connection.
Rooms have an automatic timeout of 10 minutes being IDLE i.e no connections on the room's targgetted websocket handlers. 

WS Path: `/ws/v0/gateway`
REST Path: `/api/v0/emit`

## Payloads

You can send any event via the bellow payload to the `/api/v0/emit`:

```json
{
  "room_id": "123e4567-e89b-12d3-a456-426655440000",
  "type": "HELLO",
  "data": {
    "some": "payload"
  }
}
```

## Inbuilt event types

socketeer produces two default event types `PING`, `CLOSE`.
`PING` is designed to perform a socket wakeup / heartbeat every 30 seconds.
`CLOSE` signals to the client that the conenction will be terminated.

