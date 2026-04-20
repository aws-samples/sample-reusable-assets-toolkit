from strands import Agent
from strands.models import BedrockModel

MODEL_ID = "global.anthropic.claude-sonnet-4-6"
BEDROCK_REGION = "us-east-1"
SYSTEM_PROMPT = "You are a helpful assistant."

model = BedrockModel(model_id=MODEL_ID, region_name=BEDROCK_REGION)
agent = Agent(model=model, system_prompt=SYSTEM_PROMPT)
