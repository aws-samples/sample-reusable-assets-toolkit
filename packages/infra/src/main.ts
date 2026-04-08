import { ApplicationStack } from './stacks/application-stack.js';
import { NetworkStack } from './stacks/network-stack.js';
import { StorageStack } from './stacks/storage-stack.js';
import { App } from ':idp-code/common-constructs';

const app = new App();

const env = {
  account: process.env.CDK_DEFAULT_ACCOUNT,
  region: process.env.CDK_DEFAULT_REGION,
};

const network = new NetworkStack(app, 'IDP-CODE-NETWORK', {
  env,
  crossRegionReferences: true,
});

const storage = new StorageStack(app, 'IDP-CODE-STORAGE', {
  env,
  crossRegionReferences: true,
});
storage.addDependency(network);

const application = new ApplicationStack(app, 'IDP-CODE-APPLICATION', {
  env,
  crossRegionReferences: true,
});
application.addDependency(storage);

app.synth();
