import hashlib
import os
from collections.abc import Generator
from typing import Any

import boto3
import httpx
from botocore.auth import SigV4Auth
from botocore.awsrequest import AWSRequest
from mcp.client.streamable_http import streamablehttp_client
from strands import Agent
from strands.models import BedrockModel
from strands.tools.mcp.mcp_client import MCPClient

MODEL_ID = "global.anthropic.claude-sonnet-4-6"
BEDROCK_REGION = "us-east-1"
SYSTEM_PROMPT = """You are an assistant that helps developers discover and reuse internal code assets.

Given a user's requirement, you:
- Find suitable repositories that match what they want to build (`search_repos`, `list_repos`).
- Locate concrete code snippets, functions, or examples relevant to the task (`search`).
- Fetch and explain full file contents when deeper context is needed (`file_get`).
- Summarize implementations and provide adapted code samples the user can apply.

When using the rat tools, the `query` argument MUST be in English — translate non-English user input before calling.
Always cite the repo and file path of any snippet or file you reference."""


class SigV4HTTPXAuth(httpx.Auth):
    """HTTPX Auth class that signs requests with AWS SigV4 for bedrock-agentcore."""

    def __init__(self, credentials: Any, region: str):
        self.credentials = credentials
        self.region = region
        self.signer = SigV4Auth(credentials, "bedrock-agentcore", region)

    def auth_flow(
        self, request: httpx.Request
    ) -> Generator[httpx.Request, httpx.Response, None]:
        headers = dict(request.headers)
        headers.pop("connection", None)
        headers["x-amz-content-sha256"] = hashlib.sha256(
            request.content if request.content else b""
        ).hexdigest()

        aws_request = AWSRequest(
            method=request.method,
            url=str(request.url),
            data=request.content,
            headers=headers,
        )
        self.signer.add_auth(aws_request)

        request.headers.clear()
        request.headers.update(dict(aws_request.headers))

        yield request


def _build_mcp_client() -> MCPClient | None:
    gateway_url = os.environ.get("MCP_GATEWAY_URL")
    if not gateway_url:
        return None

    region = os.environ.get("AWS_REGION", BEDROCK_REGION)
    credentials = boto3.Session().get_credentials()

    return MCPClient(
        lambda: streamablehttp_client(
            gateway_url,
            auth=SigV4HTTPXAuth(credentials, region),
            timeout=120,
            terminate_on_close=False,
        )
    )


model = BedrockModel(model_id=MODEL_ID, region_name=BEDROCK_REGION)

mcp_client = _build_mcp_client()
if mcp_client is not None:
    mcp_client.__enter__()
    tools = mcp_client.list_tools_sync()
else:
    tools = []

agent = Agent(model=model, system_prompt=SYSTEM_PROMPT, tools=tools)
