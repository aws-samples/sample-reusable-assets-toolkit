FROM public.ecr.aws/docker/library/python:3.12-slim

COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

WORKDIR /app

COPY pyproject.toml README.md ./
COPY idp_code_agent ./idp_code_agent

RUN uv pip install --system --no-cache .

EXPOSE 8080

CMD ["python", "-m", "idp_code_agent.app"]
