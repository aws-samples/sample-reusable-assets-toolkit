export const SSM_KEYS = {
  VPC_ID: '/idp-code/vpc/id',
  AURORA_CLUSTER_ENDPOINT: '/idp-code/aurora/cluster-endpoint',
  AURORA_CLUSTER_PORT: '/idp-code/aurora/cluster-port',
  AURORA_SECRET_ARN: '/idp-code/aurora/secret-arn',
  RDS_PROXY_ENDPOINT: '/idp-code/aurora/proxy-endpoint',
  RDS_PROXY_SG_ID: '/idp-code/aurora/proxy-sg-id',
  INGEST_QUEUE_URL: '/idp-code/ingest/queue-url',
  COGNITO_DOMAIN: '/idp-code/cognito/domain',
  COGNITO_APP_CLIENT_ID: '/idp-code/cognito/app-client-id',
  COGNITO_IDENTITY_POOL_ID: '/idp-code/cognito/identity-pool-id',
  COGNITO_USER_POOL_ID: '/idp-code/cognito/user-pool-id',
} as const;
