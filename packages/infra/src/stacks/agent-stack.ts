import { Stack, StackProps } from 'aws-cdk-lib';
import {
  AgentRuntimeArtifact,
  Gateway,
  ProtocolType,
  Runtime,
} from '@aws-cdk/aws-bedrock-agentcore-alpha';
import { Platform } from 'aws-cdk-lib/aws-ecr-assets';
import * as iam from 'aws-cdk-lib/aws-iam';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export interface AgentStackProps extends StackProps {
  readonly mcpGateway: Gateway;
}

export class AgentStack extends Stack {
  public readonly runtime: Runtime;

  constructor(scope: Construct, id: string, props: AgentStackProps) {
    super(scope, id, props);

    // ─── AgentCore Runtime ────────────────────────────────────────────
    const artifact = AgentRuntimeArtifact.fromAsset('../agent', {
      platform: Platform.LINUX_ARM64,
    });

    this.runtime = new Runtime(this, 'Runtime', {
      runtimeName: 'idp_code_agent',
      protocolConfiguration: ProtocolType.HTTP,
      agentRuntimeArtifact: artifact,
      environmentVariables: {
        ...(props.mcpGateway.gatewayUrl && {
          MCP_GATEWAY_URL: props.mcpGateway.gatewayUrl,
        }),
      },
    });

    props.mcpGateway.grantInvoke(this.runtime.role);

    this.runtime.role.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: [
          'bedrock:InvokeModel',
          'bedrock:InvokeModelWithResponseStream',
        ],
        resources: ['*'],
      }),
    );

    new StringParameter(this, 'AgentRuntimeArnParam', {
      parameterName: SSM_KEYS.AGENT_RUNTIME_ARN,
      stringValue: this.runtime.agentRuntimeArn,
    });
  }
}
