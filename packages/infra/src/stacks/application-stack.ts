import { CfnResource, Duration, Stack, StackProps } from 'aws-cdk-lib';
import * as ec2 from 'aws-cdk-lib/aws-ec2';
import * as lambda from 'aws-cdk-lib/aws-lambda';
import * as sqs from 'aws-cdk-lib/aws-sqs';
import * as eventsources from 'aws-cdk-lib/aws-lambda-event-sources';
import * as secretsmanager from 'aws-cdk-lib/aws-secretsmanager';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { RustFunction } from 'cargo-lambda-cdk';
import { Construct } from 'constructs';

export class ApplicationStack extends Stack {
  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    const vpcId = StringParameter.valueFromLookup(this, SSM_KEYS.VPC_ID);
    const vpc = ec2.Vpc.fromLookup(this, 'Vpc', { vpcId });

    const proxyEndpoint = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.RDS_PROXY_ENDPOINT,
    );
    const secretArn = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.AURORA_SECRET_ARN,
    );
    const dbSecret = secretsmanager.Secret.fromSecretCompleteArn(
      this,
      'DbSecret',
      secretArn,
    );

    const proxySgId = StringParameter.valueForStringParameter(
      this,
      SSM_KEYS.RDS_PROXY_SG_ID,
    );
    const proxySg = ec2.SecurityGroup.fromSecurityGroupId(
      this,
      'ProxySg',
      proxySgId,
    );

    // Dead letter queue
    const dlq = new sqs.Queue(this, 'IngestDlq', {
      retentionPeriod: Duration.days(14),
      encryption: sqs.QueueEncryption.SQS_MANAGED,
    });

    // Ingest queue
    const queue = new sqs.Queue(this, 'IngestQueue', {
      visibilityTimeout: Duration.minutes(10),
      encryption: sqs.QueueEncryption.SQS_MANAGED,
      deadLetterQueue: {
        queue: dlq,
        maxReceiveCount: 3,
      },
    });

    // checkov skip: SQS-managed SSE enabled, KMS CMK not required
    (dlq.node.defaultChild as CfnResource).addMetadata('checkov', {
      skip: [{ id: 'CKV_AWS_27', comment: 'Using SQS-managed SSE' }],
    });
    (queue.node.defaultChild as CfnResource).addMetadata('checkov', {
      skip: [{ id: 'CKV_AWS_27', comment: 'Using SQS-managed SSE' }],
    });

    // Consumer Lambda (Rust via cargo-lambda)
    const consumer = new RustFunction(this, 'IngestConsumer', {
      manifestPath: '../rat/Cargo.toml',
      binaryName: 'bootstrap',
      architecture: lambda.Architecture.ARM_64,
      bundling: {
        cargoLambdaFlags: ['-p', 'rat-lambda'],
      },
      memorySize: 512,
      timeout: Duration.minutes(5),
      vpc,
      vpcSubnets: { subnetType: ec2.SubnetType.PRIVATE_WITH_EGRESS },
      environment: {
        RDS_PROXY_ENDPOINT: proxyEndpoint,
        DB_SECRET_ARN: secretArn,
      },
    });

    // checkov skip for IngestConsumer Lambda
    (consumer.node.defaultChild as CfnResource).addMetadata('checkov', {
      skip: [
        { id: 'CKV_AWS_115', comment: 'Concurrency managed by SQS event source batch size' },
        { id: 'CKV_AWS_116', comment: 'DLQ configured on source SQS queue, not on Lambda' },
        { id: 'CKV_AWS_173', comment: 'Environment variables contain only endpoints and ARNs, not sensitive data' },
      ],
    });

    // Allow Lambda to access RDS Proxy
    proxySg.connections.allowFrom(consumer, ec2.Port.tcp(5432));

    // SQS -> Lambda trigger
    consumer.addEventSource(
      new eventsources.SqsEventSource(queue, {
        batchSize: 10,
        maxBatchingWindow: Duration.seconds(30),
      }),
    );

    // Grant DB secret read
    dbSecret.grantRead(consumer);

    // SSM parameters
    new StringParameter(this, 'IngestQueueUrlParam', {
      parameterName: SSM_KEYS.INGEST_QUEUE_URL,
      stringValue: queue.queueUrl,
    });
  }
}
