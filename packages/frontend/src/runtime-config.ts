// SPDX-License-Identifier: MIT

export type RuntimeConfig = {
  cognito: {
    region: string;
    userPoolId: string;
    userPoolClientId: string;
    identityPoolId: string;
  };
  api: { functionArn: string };
  agent: { runtimeArn: string };
};

export async function loadRuntimeConfig(): Promise<RuntimeConfig> {
  const res = await fetch('/runtime-config.json');
  if (!res.ok) {
    throw new Error(`runtime-config.json not reachable (HTTP ${res.status})`);
  }
  return res.json();
}
