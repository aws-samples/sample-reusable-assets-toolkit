import {
  CfnOutput,
  CfnResource,
  Duration,
  RemovalPolicy,
  Stack,
  StackProps,
} from 'aws-cdk-lib';
import {
  Gateway,
  GatewayAuthorizer,
  MCPProtocolVersion,
  McpGatewaySearchType,
  McpProtocolConfiguration,
  ToolSchema,
} from '@aws-cdk/aws-bedrock-agentcore-alpha';
import { CfnStage } from 'aws-cdk-lib/aws-apigatewayv2';
import * as cognito from 'aws-cdk-lib/aws-cognito';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import { LogGroup, RetentionDays } from 'aws-cdk-lib/aws-logs';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS, suppressRules } from ':idp-code/common-constructs';
import { McpAuthProxy } from '@drskur/dcr-proxy';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export interface McpStackProps extends StackProps {
  readonly userPool: cognito.IUserPool;
  readonly userPoolDomain: cognito.UserPoolDomain;
}

export class McpStack extends Stack {
  public readonly iamGateway: Gateway;
  public readonly jwtGateway: Gateway;
  public readonly mcpFunction: RustFunction;

  constructor(scope: Construct, id: string, props: McpStackProps) {
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

    const toolSchema = ToolSchema.fromLocalAsset(
      '../rat/crates/rat-mcp/schema.json',
    );

    const protocolConfig = new McpProtocolConfiguration({
      instructions:
        'Use this gateway to search the rat reusable asset store for code snippets, repositories, and files.',
      searchType: McpGatewaySearchType.SEMANTIC,
      supportedVersions: [
        MCPProtocolVersion.MCP_2025_03_26,
        MCPProtocolVersion.MCP_2025_06_18,
      ],
    });

    // ─── Confidential App Client (DCR proxy) ──────────────────────────
    // Claude Code: requires `--callback-port 33418` (exact match in Cognito).
    // Kiro CLI/IDE: requires `oauth.redirectUri: "http://127.0.0.1:49153"`
    //   in mcp.json — Kiro picks a random ephemeral port otherwise.
    // Claude Desktop / claude.ai: uses hosted callback URL.
    const proxyClient = props.userPool.addClient('McpProxyClient', {
      generateSecret: true,
      oAuth: {
        flows: { authorizationCodeGrant: true },
        scopes: [
          cognito.OAuthScope.OPENID,
          cognito.OAuthScope.PROFILE,
          cognito.OAuthScope.EMAIL,
          cognito.OAuthScope.PHONE,
        ],
        callbackUrls: [
          'http://localhost:33418/callback',
          'https://claude.ai/api/mcp/auth_callback',
          'http://127.0.0.1:49153',
        ],
      },
      preventUserExistenceErrors: true,
      supportedIdentityProviders: [
        cognito.UserPoolClientIdentityProvider.COGNITO,
      ],
    });

    // ─── IAM Gateway (internal: AgentCore Runtime) ────────────────────
    this.iamGateway = new Gateway(this, 'IamGateway', {
      gatewayName: 'idp-code-mcp-gateway-iam',
      description: 'IDP Code MCP Gateway (IAM, internal Runtime)',
      authorizerConfiguration: GatewayAuthorizer.usingAwsIam(),
      protocolConfiguration: protocolConfig,
    });

    const iamTarget = this.iamGateway.addLambdaTarget('RatTargetIam', {
      gatewayTargetName: 'rat',
      description:
        'Search and retrieve assets from the rat reusable asset store (snippets, repositories, files).',
      lambdaFunction: this.mcpFunction,
      toolSchema,
    });
    this.mcpFunction.grantInvoke(this.iamGateway.role);
    iamTarget.node.addDependency(this.iamGateway.role);

    // ─── JWT Gateway (external: MCP clients via DCR proxy) ────────────
    this.jwtGateway = new Gateway(this, 'JwtGateway', {
      gatewayName: 'idp-code-mcp-gateway-jwt',
      description: 'IDP Code MCP Gateway (JWT, external MCP clients)',
      authorizerConfiguration: GatewayAuthorizer.usingCognito({
        userPool: props.userPool,
        allowedClients: [proxyClient],
      }),
      protocolConfiguration: protocolConfig,
    });

    const jwtTarget = this.jwtGateway.addLambdaTarget('RatTargetJwt', {
      gatewayTargetName: 'rat',
      description:
        'Search and retrieve assets from the rat reusable asset store (snippets, repositories, files).',
      lambdaFunction: this.mcpFunction,
      toolSchema,
    });
    this.mcpFunction.grantInvoke(this.jwtGateway.role);
    jwtTarget.node.addDependency(this.jwtGateway.role);

    // ─── DCR Proxy ────────────────────────────────────────────────────
    const proxy = new McpAuthProxy(this, 'AuthProxy', {
      userPool: props.userPool,
      userPoolClient: proxyClient,
      cognitoDomain: props.userPoolDomain,
      upstreamUrl: this.jwtGateway.gatewayUrl ?? '',
      allowedRedirectPatterns: [
        /^http:\/\/localhost:\d+\/callback$/,
        /^http:\/\/127\.0\.0\.1:\d+\/?$/,
        /^https:\/\/claude\.ai\/api\/mcp\/auth_callback$/,
      ],
    });

    // ─── API Gateway access logging ───────────────────────────────────
    const accessLogGroup = new LogGroup(this, 'AuthProxyAccessLogs', {
      retention: RetentionDays.ONE_MONTH,
      removalPolicy: RemovalPolicy.DESTROY,
    });
    const cfnStage = proxy.httpApi.defaultStage?.node
      .defaultChild as CfnStage;
    cfnStage.accessLogSettings = {
      destinationArn: accessLogGroup.logGroupArn,
      format: JSON.stringify({
        requestId: '$context.requestId',
        ip: '$context.identity.sourceIp',
        requestTime: '$context.requestTime',
        httpMethod: '$context.httpMethod',
        routeKey: '$context.routeKey',
        status: '$context.status',
        protocol: '$context.protocol',
        responseLength: '$context.responseLength',
      }),
    };

    // ─── Checkov suppressions for AuthProxy (third-party construct) ───
    suppressRules(
      Stack.of(this),
      ['CKV_AWS_158'],
      'AuthProxy LogGroups: no sensitive data, CloudWatch default encryption sufficient',
      (c) =>
        CfnResource.isCfnResource(c) &&
        c.cfnResourceType === 'AWS::Logs::LogGroup' &&
        c.node.path.includes('AuthProxy'),
    );
    suppressRules(
      Stack.of(this),
      ['CKV_AWS_149'],
      'AuthProxy Cognito client secret, KMS CMK encryption to be addressed later',
      (c) =>
        CfnResource.isCfnResource(c) &&
        c.cfnResourceType === 'AWS::SecretsManager::Secret' &&
        c.node.path.includes('AuthProxy'),
    );

    // ─── CfnOutputs ───────────────────────────────────────────────────
    new CfnOutput(this, 'IamGatewayUrl', {
      value: this.iamGateway.gatewayUrl ?? '',
    });

    new CfnOutput(this, 'JwtGatewayUrl', {
      value: this.jwtGateway.gatewayUrl ?? '',
    });

    new CfnOutput(this, 'AuthProxyUrl', {
      value: proxy.proxyUrl,
    });

    new CfnOutput(this, 'McpEndpoint', {
      value: proxy.mcpUrl,
    });

    new CfnOutput(this, 'AuthProxyMetadataUrl', {
      value: proxy.metadataUrl,
    });
  }
}
