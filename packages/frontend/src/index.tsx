/* @refresh reload */
import './index.css';
import { render } from 'solid-js/web';
import 'solid-devtools';
import { AuthProvider, useAuth } from '@drskur/oidc-provider-solid';
import type { JSX } from 'solid-js';
import type { UserManagerSettings } from 'oidc-client-ts';

import App from './App';
import { loadRuntimeConfig, type RuntimeConfig } from './runtime-config';
import { configureRatApi } from '@/lib/rat-api';
import { configureAgentApi } from '@/lib/agent-api';

const config = await loadRuntimeConfig();

const authConfig: UserManagerSettings = {
  authority: `https://cognito-idp.${config.cognito.region}.amazonaws.com/${config.cognito.userPoolId}`,
  client_id: config.cognito.userPoolClientId,
  redirect_uri: `${window.location.origin}/callback`,
  post_logout_redirect_uri: `${window.location.origin}/logout`,
  silent_redirect_uri: `${window.location.origin}/silent-callback.html`,
  response_type: 'code',
  scope: 'openid profile email',
  automaticSilentRenew: true,
  loadUserInfo: false,
};

const ApiBootstrap = (props: { rc: RuntimeConfig; children: JSX.Element }) => {
  const { userManager } = useAuth();
  configureRatApi(props.rc, userManager);
  configureAgentApi(props.rc, userManager);
  return <>{props.children}</>;
};

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
    'Root element not found. Did you forget to add it to your index.html? Or maybe the id attribute got misspelled?',
  );
}

render(
  () => (
    <AuthProvider config={authConfig}>
      <ApiBootstrap rc={config}>
        <App />
      </ApiBootstrap>
    </AuthProvider>
  ),
  root!,
);
