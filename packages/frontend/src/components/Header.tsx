import type { Component, JSX } from 'solid-js';
import { Show } from 'solid-js';
import { A } from '@solidjs/router';
import { useAuth } from '@drskur/oidc-provider-solid';

export const Header: Component<{ children?: JSX.Element }> = (props) => {
  const { user, isAuthenticated, login, logout } = useAuth();

  return (
    <header class="sticky top-0 z-10 flex items-center gap-6 border-b border-gray-200 bg-white px-6 py-3">
      <A
        href="/"
        class="flex flex-none items-center gap-1.5 font-mono text-sm font-bold"
      >
        <span class="text-base grayscale" aria-hidden="true">
          🐿️
        </span>
        rat
      </A>

      <div class="flex-1">{props.children}</div>

      <div class="flex flex-none items-center gap-4 text-xs">
        <Show
          when={isAuthenticated()}
          fallback={
            <button
              onClick={login}
              class="font-mono rounded border border-gray-900 px-3 py-1 hover:bg-gray-900 hover:text-white"
            >
              Sign in
            </button>
          }
        >
          <span class="font-mono text-gray-500">
            {(user()?.profile.email as string | undefined) ?? ''}
          </span>
          <button
            onClick={logout}
            class="font-mono text-gray-500 hover:text-gray-900"
          >
            Sign out
          </button>
        </Show>
      </div>
    </header>
  );
};
