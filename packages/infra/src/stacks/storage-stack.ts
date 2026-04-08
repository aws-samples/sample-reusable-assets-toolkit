import { RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as rds from 'aws-cdk-lib/aws-rds';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export class StorageStack extends Stack {
  public readonly cluster: rds.DatabaseCluster;

  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const vpcId = StringParameter.valueFromLookup(this, SSM_KEYS.VPC_ID);
    const vpc = ec2.Vpc.fromLookup(this, 'Vpc', { vpcId });

    this.cluster = new rds.DatabaseCluster(this, 'AuroraCluster', {
      engine: rds.DatabaseClusterEngine.auroraPostgres({
        version: rds.AuroraPostgresEngineVersion.VER_17_7,
      }),
      serverlessV2MinCapacity: 0.5,
      serverlessV2MaxCapacity: 2,
      writer: rds.ClusterInstance.serverlessV2('Writer'),
      vpc,
      vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_ISOLATED },
      defaultDatabaseName: 'assets',
      removalPolicy: RemovalPolicy.RETAIN,
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

    const proxy = this.cluster.addProxy('RdsProxy', {
      vpc,
      secrets: this.cluster.secret ? [this.cluster.secret] : [],
      requireTLS: true,
    });

    new StringParameter(this, 'ProxyEndpointParam', {
      parameterName: SSM_KEYS.RDS_PROXY_ENDPOINT,
      stringValue: proxy.endpoint,
    });
  }
}
