from typing import TypedDict

from bedrock_agentcore.runtime import BedrockAgentCoreApp

from idp_code_agent.agent import agent


class Payload(TypedDict):
    message: str


app = BedrockAgentCoreApp()


@app.entrypoint
async def invoke(payload: Payload):
    stream = agent.stream_async(payload["message"])
    yielded_tool_use_ids: set[str] = set()
    async for event in stream:
        if "data" in event:
            yield {"type": "text", "content": event["data"]}
        if "current_tool_use" in event:
            tool_use = event["current_tool_use"]
            tool_use_id = tool_use.get("toolUseId")
            if (
                tool_use_id
                and tool_use_id not in yielded_tool_use_ids
                and tool_use.get("name")
            ):
                yielded_tool_use_ids.add(tool_use_id)
                yield {
                    "type": "tool_use",
                    "tool_use_id": tool_use_id,
                    "name": tool_use["name"],
                }
        if event.get("complete"):
            yield {"type": "complete"}


if __name__ == "__main__":
    app.run()
