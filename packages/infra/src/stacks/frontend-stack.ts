// SPDX-License-Identifier: MIT

import {
  CfnResource,
  Duration,
  RemovalPolicy,
  Stack,
  StackProps,
} from 'aws-cdk-lib';
import * as cloudfront from 'aws-cdk-lib/aws-cloudfront';
import * as origins from 'aws-cdk-lib/aws-cloudfront-origins';
import * as cognito from 'aws-cdk-lib/aws-cognito';
import {
  IdentityPool,
  UserPoolAuthenticationProvider,
} from 'aws-cdk-lib/aws-cognito-identitypool';
import * as iam from 'aws-cdk-lib/aws-iam';
import * as s3 from 'aws-cdk-lib/aws-s3';
import * as s3deploy from 'aws-cdk-lib/aws-s3-deployment';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS, suppressRules } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export interface FrontendStackProps extends StackProps {
  userPool: cognito.IUserPool;
}

export class FrontendStack extends Stack {
  public readonly distribution: cloudfront.Distribution;
  public readonly authenticatedRole: iam.IRole;

  constructor(scope: Construct, id: string, props: FrontendStackProps) {
    super(scope, id, props);

    // ─── Access Log Bucket ──────────────────────────────────────────────
    const logBucket = new s3.Bucket(this, 'LogBucket', {
      encryption: s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      enforceSSL: true,
      objectOwnership: s3.ObjectOwnership.BUCKET_OWNER_PREFERRED,
      versioned: true,
      removalPolicy: RemovalPolicy.RETAIN,
      lifecycleRules: [{ expiration: Duration.days(365) }],
    });

    (logBucket.node.defaultChild as CfnResource).addMetadata('checkov', {
      skip: [
        {
          id: 'CKV_AWS_18',
          comment: 'This bucket is itself the access log target',
        },
      ],
    });

    // ─── Asset Bucket ───────────────────────────────────────────────────
    const assetBucket = new s3.Bucket(this, 'AssetBucket', {
      encryption: s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      enforceSSL: true,
      versioned: true,
      removalPolicy: RemovalPolicy.RETAIN,
      serverAccessLogsBucket: logBucket,
      serverAccessLogsPrefix: 'asset-bucket/',
    });

    // ─── CloudFront Distribution ────────────────────────────────────────
    this.distribution = new cloudfront.Distribution(this, 'Distribution', {
      defaultBehavior: {
        origin: origins.S3BucketOrigin.withOriginAccessControl(assetBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
        compress: true,
      },
      defaultRootObject: 'index.html',
      // SPA fallback: any unknown path returns index.html so client-side router can handle it
      errorResponses: [
        {
          httpStatus: 403,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: Duration.minutes(5),
        },
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: '/index.html',
          ttl: Duration.minutes(5),
        },
      ],
      minimumProtocolVersion: cloudfront.SecurityPolicyProtocol.TLS_V1_3_2025,
      enableLogging: true,
      logBucket,
      logFilePrefix: 'cloudfront/',
    });

    (this.distribution.node.defaultChild as CfnResource).addMetadata(
      'checkov',
      {
        skip: [
          {
            id: 'CKV_AWS_68',
            comment: 'WAF not required for internal IDP frontend',
          },
          {
            id: 'CKV_AWS_305',
            comment: 'defaultRootObject is set to index.html',
          },
          {
            id: 'CKV_AWS_310',
            comment:
              'SPA fallback via errorResponses, no secondary origin needed',
          },
          {
            id: 'CKV_AWS_174',
            comment:
              'Using default CloudFront certificate (*.cloudfront.net); MinimumProtocolVersion only applies when a custom ACM certificate is attached',
          },
        ],
      },
    );

    // ─── Cognito Web Client (dedicated to this frontend) ───────────────
    // Use `new UserPoolClient` instead of `userPool.addClient` so the client
    // resource lives in this stack, not in the UserPool's stack — otherwise
    // AuthStack ends up referencing the Distribution and we get a stack cycle.
    const cfUrl = `https://${this.distribution.distributionDomainName}`;
    const webClient = new cognito.UserPoolClient(this, 'WebClient', {
      userPool: props.userPool,
      generateSecret: false,
      oAuth: {
        flows: { authorizationCodeGrant: true },
        scopes: [
          cognito.OAuthScope.OPENID,
          cognito.OAuthScope.PROFILE,
          cognito.OAuthScope.EMAIL,
        ],
        callbackUrls: [`${cfUrl}/callback`, 'http://localhost:3000/callback'],
        logoutUrls: [`${cfUrl}/logout`, 'http://localhost:3000/logout'],
      },
      preventUserExistenceErrors: true,
      supportedIdentityProviders: [
        cognito.UserPoolClientIdentityProvider.COGNITO,
      ],
    });

    // Managed Login branding (Cognito default style)
    new cognito.CfnManagedLoginBranding(this, 'ManagedLoginBranding', {
      userPoolId: props.userPool.userPoolId,
      clientId: webClient.userPoolClientId,
      useCognitoProvidedValues: true,
    });

    // ─── Web Identity Pool ─────────────────────────────────────────────
    const webIdentityPool = new IdentityPool(this, 'WebIdentityPool', {
      allowUnauthenticatedIdentities: false,
      authenticationProviders: {
        userPools: [
          new UserPoolAuthenticationProvider({
            userPool: props.userPool,
            userPoolClient: webClient,
          }),
        ],
      },
    });
    this.authenticatedRole = webIdentityPool.authenticatedRole;

    // Grant the web role the same set of application permissions the CLI role
    // has. Application resources are referenced via SSM dynamic refs so this
    // stack stays dependent only on AuthStack + ApplicationStack.
    const apiFunctionArn = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.API_FUNCTION_ARN,
    );
    const migrationFunctionArn = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.MIGRATION_FUNCTION_ARN,
    );
    const agentRuntimeArn = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.AGENT_RUNTIME_ARN,
    );

    this.authenticatedRole.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: ['sqs:SendMessage', 'sqs:GetQueueAttributes'],
        resources: [`arn:aws:sqs:${this.region}:${this.account}:*Ingest*`],
      }),
    );
    this.authenticatedRole.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: ['lambda:InvokeFunction'],
        resources: [apiFunctionArn, migrationFunctionArn],
      }),
    );
    this.authenticatedRole.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: ['bedrock-agentcore:InvokeAgentRuntime'],
        resources: [agentRuntimeArn, `${agentRuntimeArn}/*`],
      }),
    );
    this.authenticatedRole.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: ['ssm:GetParameter'],
        resources: [
          `arn:aws:ssm:${this.region}:${this.account}:parameter/idp-code/*`,
        ],
      }),
    );

    // ─── Bucket Deployment (SPA assets + runtime-config.json) ──────────
    new s3deploy.BucketDeployment(this, 'Deployment', {
      sources: [
        s3deploy.Source.asset('../frontend/dist'),
        s3deploy.Source.jsonData('runtime-config.json', {
          cognito: {
            region: this.region,
            userPoolId: props.userPool.userPoolId,
            userPoolClientId: webClient.userPoolClientId,
            identityPoolId: webIdentityPool.identityPoolId,
          },
          api: {
            functionArn: apiFunctionArn,
          },
          agent: {
            runtimeArn: agentRuntimeArn,
          },
        }),
      ],
      destinationBucket: assetBucket,
      distribution: this.distribution,
      distributionPaths: ['/*'],
    });

    // CDK-generated BucketDeployment Lambda & IAM policy — not user-controllable.
    suppressRules(
      Stack.of(this),
      ['CKV_AWS_111'],
      'CDK-auto-generated BucketDeployment IAM policy uses wildcard to deploy arbitrary assets',
      (c) =>
        CfnResource.isCfnResource(c) &&
        c.cfnResourceType === 'AWS::IAM::Policy' &&
        c.node.path.includes('/Custom::CDKBucketDeployment'),
    );
    suppressRules(
      Stack.of(this),
      ['CKV_AWS_115', 'CKV_AWS_116', 'CKV_AWS_117', 'CKV_AWS_173'],
      'CDK-auto-generated BucketDeployment Lambda (one-shot deploy helper, not user-controllable)',
      (c) =>
        CfnResource.isCfnResource(c) &&
        c.cfnResourceType === 'AWS::Lambda::Function' &&
        c.node.path.includes('/Custom::CDKBucketDeployment'),
    );

    // ─── SSM Parameters ─────────────────────────────────────────────────
    new StringParameter(this, 'DistributionIdParam', {
      parameterName: SSM_KEYS.FRONTEND_DISTRIBUTION_ID,
      stringValue: this.distribution.distributionId,
    });

    new StringParameter(this, 'DistributionDomainParam', {
      parameterName: SSM_KEYS.FRONTEND_DISTRIBUTION_DOMAIN,
      stringValue: this.distribution.distributionDomainName,
    });

    new StringParameter(this, 'WebClientIdParam', {
      parameterName: SSM_KEYS.COGNITO_WEB_CLIENT_ID,
      stringValue: webClient.userPoolClientId,
    });

    new StringParameter(this, 'WebIdentityPoolIdParam', {
      parameterName: SSM_KEYS.COGNITO_WEB_IDENTITY_POOL_ID,
      stringValue: webIdentityPool.identityPoolId,
    });
  }
}
