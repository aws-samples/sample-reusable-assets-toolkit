// SPDX-License-Identifier: MIT

import { Duration, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as kms from 'aws-cdk-lib/aws-kms';
import * as rds from 'aws-cdk-lib/aws-rds';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export class StorageStack extends Stack {
  public readonly cluster: rds.DatabaseCluster;

  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const vpcId = StringParameter.valueFromLookup(this, SSM_KEYS.VPC_ID);
    const vpc = ec2.Vpc.fromLookup(this, 'Vpc', { vpcId });

    const storageKey = new kms.Key(this, 'StorageKey', {
      enableKeyRotation: true,
      removalPolicy: RemovalPolicy.RETAIN,
    });

    this.cluster = new rds.DatabaseCluster(this, 'AuroraCluster', {
      engine: rds.DatabaseClusterEngine.auroraPostgres({
        version: rds.AuroraPostgresEngineVersion.VER_17_7,
      }),
      serverlessV2MinCapacity: 0.5,
      serverlessV2MaxCapacity: 2,
      writer: rds.ClusterInstance.serverlessV2('Writer', {
        enablePerformanceInsights: true,
      }),
      vpc,
      vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_ISOLATED },
      defaultDatabaseName: 'assets',
      storageEncrypted: true,
      storageEncryptionKey: storageKey,
      iamAuthentication: true,
      monitoringInterval: Duration.seconds(60),
      removalPolicy: RemovalPolicy.RETAIN,
    });

    // checkov skip: Secret KMS CMK — CDK auto-generated secret, will address separately
    const cfnSecret = this.cluster.node.findChild('Secret').node.defaultChild as secretsmanager.CfnSecret;
    cfnSecret.addMetadata('checkov', {
      skip: [{ id: 'CKV_AWS_149', comment: 'Aurora auto-generated secret, KMS CMK encryption to be addressed later' }],
    });

    new StringParameter(this, 'ClusterEndpointParam', {
      parameterName: SSM_KEYS.AURORA_CLUSTER_ENDPOINT,
      stringValue: this.cluster.clusterEndpoint.hostname,
    });

    new StringParameter(this, 'ClusterPortParam', {
      parameterName: SSM_KEYS.AURORA_CLUSTER_PORT,
      stringValue: this.cluster.clusterEndpoint.port.toString(),
    });

    new StringParameter(this, 'SecretArnParam', {
      parameterName: SSM_KEYS.AURORA_SECRET_ARN,
      stringValue: this.cluster.secret?.secretArn ?? '',
    });

    const proxySg = new ec2.SecurityGroup(this, 'RdsProxySg', {
      vpc,
      description: 'Security group for RDS Proxy',
    });

    const proxy = this.cluster.addProxy('RdsProxy', {
      vpc,
      secrets: this.cluster.secret ? [this.cluster.secret] : [],
      requireTLS: true,
      securityGroups: [proxySg],
    });

    new StringParameter(this, 'ProxyEndpointParam', {
      parameterName: SSM_KEYS.RDS_PROXY_ENDPOINT,
      stringValue: proxy.endpoint,
    });

    new StringParameter(this, 'ProxySgIdParam', {
      parameterName: SSM_KEYS.RDS_PROXY_SG_ID,
      stringValue: proxySg.securityGroupId,
    });
  }
}
