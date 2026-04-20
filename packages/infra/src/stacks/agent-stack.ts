import { Duration, Stack, StackProps } from 'aws-cdk-lib';
import {
  AgentRuntimeArtifact,
  Gateway,
  GatewayAuthorizer,
  MCPProtocolVersion,
  McpGatewaySearchType,
  McpProtocolConfiguration,
  ProtocolType,
  Runtime,
  ToolSchema,
} from '@aws-cdk/aws-bedrock-agentcore-alpha';
import { Platform } from 'aws-cdk-lib/aws-ecr-assets';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export class AgentStack extends Stack {
  public readonly runtime: Runtime;
  public readonly gateway: Gateway;
  public readonly mcpFunction: RustFunction;

  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const apiFunctionArn = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.API_FUNCTION_ARN,
    );

    // ─── rat-mcp Lambda ───────────────────────────────────────────────
    this.mcpFunction = new RustFunction(this, 'McpFunction', {
      manifestPath: '../rat/Cargo.toml',
      binaryName: 'bootstrap',
      architecture: lambda.Architecture.ARM_64,
      bundling: {
        cargoLambdaFlags: ['-p', 'rat-mcp'],
      },
      memorySize: 256,
      timeout: Duration.seconds(60),
      environment: {
        API_FUNCTION_ARN: apiFunctionArn,
      },
    });

    this.mcpFunction.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ['lambda:InvokeFunction'],
        resources: [apiFunctionArn],
      }),
    );

    // ─── AgentCore MCP Gateway ────────────────────────────────────────
    this.gateway = new Gateway(this, 'McpGateway', {
      gatewayName: 'idp-code-mcp-gateway',
      description: 'IDP Code MCP Gateway (rat reusable asset store)',
      authorizerConfiguration: GatewayAuthorizer.usingAwsIam(),
      protocolConfiguration: new McpProtocolConfiguration({
        instructions:
          'Use this gateway to search the rat reusable asset store for code snippets, repositories, and files.',
        searchType: McpGatewaySearchType.SEMANTIC,
        supportedVersions: [
          MCPProtocolVersion.MCP_2025_03_26,
          MCPProtocolVersion.MCP_2025_06_18,
        ],
      }),
    });

    const ratTarget = this.gateway.addLambdaTarget('RatTarget', {
      gatewayTargetName: 'rat',
      description:
        'Search and retrieve assets from the rat reusable asset store (snippets, repositories, files).',
      lambdaFunction: this.mcpFunction,
      toolSchema: ToolSchema.fromLocalAsset(
        '../rat/crates/rat-mcp/schema.json',
      ),
    });
    this.mcpFunction.grantInvoke(this.gateway.role);
    ratTarget.node.addDependency(this.gateway.role);

    // ─── AgentCore Runtime ────────────────────────────────────────────
    const artifact = AgentRuntimeArtifact.fromAsset('../agent', {
      platform: Platform.LINUX_ARM64,
    });

    this.runtime = new Runtime(this, 'Runtime', {
      runtimeName: 'idp_code_agent',
      protocolConfiguration: ProtocolType.HTTP,
      agentRuntimeArtifact: artifact,
      environmentVariables: {
        ...(this.gateway.gatewayUrl && {
          MCP_GATEWAY_URL: this.gateway.gatewayUrl,
        }),
      },
    });

    this.gateway.grantInvoke(this.runtime.role);

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
