// SPDX-License-Identifier: MIT

import { CfnResource, Stack, StackProps } from 'aws-cdk-lib';
import {
  Vpc,
  SubnetType,
  IpAddresses,
  FlowLogDestination,
  FlowLogTrafficType,
} from 'aws-cdk-lib/aws-ec2';
import * as logs from 'aws-cdk-lib/aws-logs';
import { StringParameter } from 'aws-cdk-lib/aws-ssm';
import { SSM_KEYS } from ':idp-code/common-constructs';
import { Construct } from 'constructs';

export class NetworkStack extends Stack {
  public readonly vpc: Vpc;

  constructor(scope: Construct, id: string, props?: StackProps) {
    super(scope, id, props);

    this.vpc = new Vpc(this, 'Vpc', {
      ipAddresses: IpAddresses.cidr('10.0.0.0/16'),
      maxAzs: 2,
      natGateways: 1,
      subnetConfiguration: [
        {
          name: 'Public',
          subnetType: SubnetType.PUBLIC,
          cidrMask: 24,
        },
        {
          name: 'Private',
          subnetType: SubnetType.PRIVATE_WITH_EGRESS,
          cidrMask: 24,
        },
        {
          name: 'Isolated',
          subnetType: SubnetType.PRIVATE_ISOLATED,
          cidrMask: 24,
        },
      ],
    });

    const flowLogGroup = new logs.LogGroup(this, 'FlowLogGroup', {
      retention: logs.RetentionDays.TWO_YEARS,
    });

    // checkov skip: KMS CMK not required for VPC flow logs
    (flowLogGroup.node.defaultChild as CfnResource).addMetadata('checkov', {
      skip: [
        {
          id: 'CKV_AWS_158',
          comment:
            'VPC flow logs do not contain sensitive data, CloudWatch default encryption is sufficient',
        },
      ],
    });

    this.vpc.addFlowLog('FlowLog', {
      destination: FlowLogDestination.toCloudWatchLogs(flowLogGroup),
      trafficType: FlowLogTrafficType.REJECT,
    });

    // checkov skip: CDK auto-generated custom resource Lambda, not user-controllable
    const handler = this.node
      .tryFindChild('Custom::VpcRestrictDefaultSGCustomResourceProvider')
      ?.node.tryFindChild('Handler') as CfnResource | undefined;
    handler?.addMetadata('checkov', {
      skip: [
        {
          id: 'CKV_AWS_115',
          comment: 'CDK auto-generated custom resource Lambda',
        },
        {
          id: 'CKV_AWS_116',
          comment: 'CDK auto-generated custom resource Lambda',
        },
        {
          id: 'CKV_AWS_117',
          comment: 'CDK auto-generated custom resource Lambda',
        },
      ],
    });

    new StringParameter(this, 'VpcIdParam', {
      parameterName: SSM_KEYS.VPC_ID,
      stringValue: this.vpc.vpcId,
    });
  }
}
