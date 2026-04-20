import { AgentStack } from './stacks/agent-stack.js';
import { ApplicationStack } from './stacks/application-stack.js';
import { AuthStack } from './stacks/auth-stack.js';
import { FrontendStack } from './stacks/frontend-stack.js';
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

const auth = new AuthStack(app, 'IDP-CODE-AUTH', {
  env,
  crossRegionReferences: true,
});

const application = new ApplicationStack(app, 'IDP-CODE-APPLICATION', {
  env,
  crossRegionReferences: true,
  authenticatedRole: auth.authenticatedRole,
});
application.addDependency(auth);
application.addDependency(storage);

const agent = new AgentStack(app, 'IDP-CODE-AGENT', {
  env,
  crossRegionReferences: true,
});

const frontend = new FrontendStack(app, 'IDP-CODE-FRONTEND', {
  env,
  crossRegionReferences: true,
  userPool: auth.userPool,
});
frontend.addDependency(auth);
frontend.addDependency(application);
frontend.addDependency(agent);

app.synth();
