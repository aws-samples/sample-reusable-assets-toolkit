// SPDX-License-Identifier: MIT

import {
  BedrockAgentCoreClient,
  InvokeAgentRuntimeCommand,
} from '@aws-sdk/client-bedrock-agentcore';
import { CognitoIdentityClient } from '@aws-sdk/client-cognito-identity';
import { fromCognitoIdentityPool } from '@aws-sdk/credential-providers';
import { UserManager } from 'oidc-client-ts';

import type { RuntimeConfig } from '@/runtime-config';

export type AgentEvent =
  | { type: 'text'; content: string }
  | { type: 'tool_use'; tool_use_id: string; name: string }
  | { type: 'complete' };

let client: BedrockAgentCoreClient | null = null;
let runtimeArn: string | null = null;

export function configureAgentApi(
  rc: RuntimeConfig,
  userManager: UserManager,
): void {
  const region = rc.cognito.region;
  const provider = `cognito-idp.${region}.amazonaws.com/${rc.cognito.userPoolId}`;

  client = new BedrockAgentCoreClient({
    region,
    credentials: fromCognitoIdentityPool({
      client: new CognitoIdentityClient({ region }),
      identityPoolId: rc.cognito.identityPoolId,
      logins: {
        [provider]: async () => {
          const user = await userManager.getUser();
          if (!user || user.expired) {
            const renewed = await userManager.signinSilent();
            if (!renewed?.id_token) {
              throw new Error('No Cognito ID token available — sign in first.');
            }
            return renewed.id_token;
          }
          return user.id_token!;
        },
      },
    }),
  });
  runtimeArn = rc.agent.runtimeArn;
}

export async function* invokeAgentStream(
  message: string,
  opts?: { sessionId?: string; signal?: AbortSignal },
): AsyncGenerator<AgentEvent> {
  if (!client || !runtimeArn) {
    throw new Error(
      'agent-api not configured. Call configureAgentApi() first.',
    );
  }

  const sessionId = opts?.sessionId ?? sessionIdFallback();

  const result = await client.send(
    new InvokeAgentRuntimeCommand({
      agentRuntimeArn: runtimeArn,
      runtimeSessionId: sessionId,
      qualifier: 'DEFAULT',
      payload: new TextEncoder().encode(JSON.stringify({ message })),
    }),
    { abortSignal: opts?.signal },
  );

  const stream = result.response?.transformToWebStream?.();
  if (!stream) throw new Error('agent runtime returned no stream');

  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    // SSE message boundary — double newline
    const parts = buffer.split('\n\n');
    buffer = parts.pop() ?? '';
    for (const part of parts) {
      const event = parseSseEvent(part);
      if (event) yield event;
    }
  }

  const tail = parseSseEvent(buffer);
  if (tail) yield tail;
}

function parseSseEvent(raw: string): AgentEvent | null {
  const data = raw
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l.startsWith('data:'))
    .map((l) => l.slice(5).trim())
    .join('');
  if (!data) return null;
  return JSON.parse(data) as AgentEvent;
}

// runtimeSessionId must be 33–100 chars. crypto.randomUUID is 36.
function sessionIdFallback(): string {
  return crypto.randomUUID() + crypto.randomUUID().slice(0, 4);
}
