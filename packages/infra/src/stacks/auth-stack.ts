// SPDX-License-Identifier: MIT

import { CfnOutput, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
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
  public readonly authenticatedRole: iam.IRole;
  public readonly userPool: cognito.UserPool;
  public readonly userPoolDomain: cognito.UserPoolDomain;
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    // --- Cognito User Pool ---
    const domainPrefix = `idp-code-${this.account}`;
    const relyingPartyDomain = `${domainPrefix}.auth.${this.region}.amazoncognito.com`;

    const userPool = new cognito.UserPool(this, 'UserPool', {
      selfSignUpEnabled: false,
      signInAliases: { email: true },
      autoVerify: { email: true },
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
    const userPoolDomain = userPool.addDomain('Domain', {
      cognitoDomain: { domainPrefix },
      managedLoginVersion: cognito.ManagedLoginVersion.NEWER_MANAGED_LOGIN,
    });
    this.userPoolDomain = userPoolDomain;

    // Public app client (PKCE, no secret)
    const appClient = userPool.addClient('CliClient', {
      generateSecret: false,
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

    this.authenticatedRole = identityPool.authenticatedRole;
    this.userPool = userPool;

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

    // --- CfnOutputs ---
    new CfnOutput(this, 'CognitoDomain', {
      value: relyingPartyDomain,
    });

    new CfnOutput(this, 'CognitoAppClientId', {
      value: appClient.userPoolClientId,
    });

    new CfnOutput(this, 'CognitoIdentityPoolId', {
      value: identityPool.identityPoolId,
    });

    new CfnOutput(this, 'CognitoUserPoolId', {
      value: userPool.userPoolId,
    });
  }
}
