/* @refresh reload */
import './index.css';
import { render } from 'solid-js/web';
import 'solid-devtools';
import { AuthProvider } from 'oidc-provider-solid';
import type { UserManagerSettings } from 'oidc-client-ts';

import App from './App';
import { loadRuntimeConfig } from './runtime-config';
import { configureRatApi } from '@/lib/rat-api';

const config = await loadRuntimeConfig();

const authConfig: UserManagerSettings = {
  authority: `https://cognito-idp.${config.cognito.region}.amazonaws.com/${config.cognito.userPoolId}`,
  client_id: config.cognito.userPoolClientId,
  redirect_uri: `${window.location.origin}/callback`,
  post_logout_redirect_uri: `${window.location.origin}/logout`,
  response_type: 'code',
  scope: 'openid profile email',
};

configureRatApi(config, authConfig);

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
    'Root element not found. Did you forget to add it to your index.html? Or maybe the id attribute got misspelled?',
  );
}

render(
  () => (
    <AuthProvider config={authConfig}>
      <App />
    </AuthProvider>
  ),
  root!,
);
