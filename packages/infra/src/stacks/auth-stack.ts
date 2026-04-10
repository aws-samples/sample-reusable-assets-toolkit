import { RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import * as cognito from 'aws-cdk-lib/aws-cognito';
import * as iam from 'aws-cdk-lib/aws-iam';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import {
  IdentityPool,
  UserPoolAuthenticationProvider,
} from 'aws-cdk-lib/aws-cognito-identitypool';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export class AuthStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    // --- Cognito User Pool ---
    const domainPrefix = `idp-code-${this.account}`;
    const relyingPartyDomain = `${domainPrefix}.auth.${this.region}.amazoncognito.com`;

    const userPool = new cognito.UserPool(this, 'UserPool', {
      selfSignUpEnabled: false,
      signInAliases: { email: true },
      autoVerify: { email: true },
      featurePlan: cognito.FeaturePlan.ESSENTIALS,
      signInPolicy: {
        allowedFirstAuthFactors: { password: true, passkey: true },
      },
      passkeyRelyingPartyId: relyingPartyDomain,
      passwordPolicy: {
        minLength: 8,
        requireUppercase: true,
        requireDigits: true,
        requireSymbols: true,
      },
      accountRecovery: cognito.AccountRecovery.EMAIL_ONLY,
      removalPolicy: RemovalPolicy.RETAIN,
    });

    // Hosted UI domain (managed login)
    userPool.addDomain('Domain', {
      cognitoDomain: { domainPrefix },
      managedLoginVersion: cognito.ManagedLoginVersion.NEWER_MANAGED_LOGIN,
    });

    // Public app client (PKCE, no secret)
    const appClient = userPool.addClient('CliClient', {
      generateSecret: false,
      authFlows: {
        custom: true,
      },
      oAuth: {
        flows: { authorizationCodeGrant: true },
        scopes: [
          cognito.OAuthScope.OPENID,
          cognito.OAuthScope.PROFILE,
          cognito.OAuthScope.EMAIL,
        ],
        callbackUrls: ['http://localhost:9876/callback'],
        logoutUrls: ['http://localhost:9876/logout'],
      },
      preventUserExistenceErrors: true,
      supportedIdentityProviders: [
        cognito.UserPoolClientIdentityProvider.COGNITO,
      ],
    });

    // Managed Login branding (Cognito default style)
    new cognito.CfnManagedLoginBranding(this, 'ManagedLoginBranding', {
      userPoolId: userPool.userPoolId,
      clientId: appClient.userPoolClientId,
      useCognitoProvidedValues: true,
    });

    // --- Cognito Identity Pool ---
    const identityPool = new IdentityPool(this, 'IdentityPool', {
      allowUnauthenticatedIdentities: false,
      authenticationProviders: {
        userPools: [
          new UserPoolAuthenticationProvider({
            userPool,
            userPoolClient: appClient,
          }),
        ],
      },
    });

    // Grant authenticated role: sqs:SendMessage on IngestQueue
    identityPool.authenticatedRole.addToPrincipalPolicy(
      new iam.PolicyStatement({
        actions: ['sqs:SendMessage'],
        resources: [`arn:aws:sqs:${this.region}:${this.account}:*Ingest*`],
      }),
    );

    // --- SSM Parameters ---
    new StringParameter(this, 'CognitoDomainParam', {
      parameterName: SSM_KEYS.COGNITO_DOMAIN,
      stringValue: relyingPartyDomain,
    });

    new StringParameter(this, 'CognitoAppClientIdParam', {
      parameterName: SSM_KEYS.COGNITO_APP_CLIENT_ID,
      stringValue: appClient.userPoolClientId,
    });

    new StringParameter(this, 'CognitoIdentityPoolIdParam', {
      parameterName: SSM_KEYS.COGNITO_IDENTITY_POOL_ID,
      stringValue: identityPool.identityPoolId,
    });

    new StringParameter(this, 'CognitoUserPoolIdParam', {
      parameterName: SSM_KEYS.COGNITO_USER_POOL_ID,
      stringValue: userPool.userPoolId,
    });
  }
}
