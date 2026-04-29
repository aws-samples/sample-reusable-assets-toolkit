import { UserManager } from 'oidc-client-ts';

new UserManager({} as never).signinSilentCallback().catch((err) => {
  console.error('signinSilentCallback failed:', err);
});
