import aiohttp
import asyncio


async def test_connect():
    session = aiohttp.ClientSession()

    params = {
        "room_id": "882c3d7d-ef12-4281-9f76-503e55f60d0a",
        "token": ""
    }

    ws = await session.ws_connect("ws://127.0.0.1:8800/ws/v0/gateway", params=params)

    await ws.close()


if __name__ == "__main__":
    asyncio.run(test_connect())

