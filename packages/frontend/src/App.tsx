// SPDX-License-Identifier: MIT

import type { Component } from 'solid-js';
import { Show } from 'solid-js';
import { Route, Router } from '@solidjs/router';
import { useAuth } from '@drskur/oidc-provider-solid';
import Landing from '@/pages/Landing';
import SearchPage from '@/pages/Search';
import FilePage from '@/pages/File';
import RepoPage from '@/pages/Repo';

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
      <Router>
        <Route path="/" component={Landing} />
        <Route path="/search" component={SearchPage} />
        <Route path="/file" component={FilePage} />
        <Route path="/repo" component={RepoPage} />
        <Route path="*" component={Landing} />
      </Router>
    </Show>
  );
};

export default App;
