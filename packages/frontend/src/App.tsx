import type { Component } from 'solid-js';
import { Show } from 'solid-js';
import { useAuth } from 'oidc-provider-solid';
import Landing from '@/pages/Landing';

const App: Component = () => {
  const { isLoading } = useAuth();

  return (
    <Show
      when={!isLoading()}
      fallback={
        <div class="min-h-screen flex items-center justify-center">
          <p class="font-mono text-xs text-gray-500">Loading…</p>
        </div>
      }
    >
      <Landing />
    </Show>
  );
};

export default App;
