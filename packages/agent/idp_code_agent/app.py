from typing import TypedDict

from bedrock_agentcore.runtime import BedrockAgentCoreApp

from idp_code_agent.agent import agent


class Payload(TypedDict):
    message: str


app = BedrockAgentCoreApp()


@app.entrypoint
async def invoke(payload: Payload):
    stream = agent.stream_async(payload["message"])
    async for event in stream:
        if "data" in event:
            yield {"type": "text", "content": event["data"]}
        if event.get("complete"):
            yield {"type": "complete"}


if __name__ == "__main__":
    app.run()
