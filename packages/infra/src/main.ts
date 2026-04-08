import { ApplicationStage } from './stages/application-stage.js';
import { App } from ':idp-code/common-constructs';

const app = new App();

// Use this to deploy your own sandbox environment (assumes your CLI credentials)
new ApplicationStage(app, 'idp-code-infra-sandbox', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
});

app.synth();
