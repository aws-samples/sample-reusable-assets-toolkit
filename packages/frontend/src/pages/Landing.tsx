import type { Component } from 'solid-js';
import { createResource, createSignal, For, Show } from 'solid-js';
import { useAuth } from 'oidc-provider-solid';
import { listRepos } from '@/lib/rat-api';

const Landing: Component = () => {
  const { user, isAuthenticated, login, logout } = useAuth();
  const [query, setQuery] = createSignal('');

  const [repos] = createResource(
    () => isAuthenticated(),
    async (signedIn) => {
      if (!signedIn) return [];
      const { repos } = await listRepos();
      return repos;
    },
  );

  const submit = () => {
    const q = query().trim();
    if (!q) return;
    // TODO: navigate to /search?q=... once router is added
    console.log('search', { q });
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      {/* Header */}
      <header class="flex items-center justify-between border-b border-gray-200 px-6 py-3">
        <div class="flex items-center gap-1.5 font-mono text-sm font-bold">
          <span class="text-base grayscale" aria-hidden="true">🐀</span>
          rat
        </div>
        <div class="flex items-center gap-4 text-xs">
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

      {/* Main */}
      <main class="flex flex-1 items-start justify-center px-6 py-24">
        <div class="w-full max-w-2xl space-y-12">
          {/* Search */}
          <div class="flex items-center rounded border border-gray-300 bg-white focus-within:border-gray-900">
            <input
              type="text"
              value={query()}
              onInput={(e) => setQuery(e.currentTarget.value)}
              onKeyDown={(e) => e.key === 'Enter' && submit()}
              placeholder="Search repos, code, docs…"
              autofocus
              class="flex-1 bg-transparent px-4 py-3 font-mono text-sm outline-none placeholder:text-gray-400"
            />
          </div>

          {/* Browse */}
          <section class="space-y-3">
            <h2 class="text-xs uppercase tracking-wider text-gray-500">
              browse
            </h2>

            <Show
              when={isAuthenticated()}
              fallback={
                <p class="font-mono text-sm text-gray-400">
                  Sign in to browse repositories.
                </p>
              }
            >
              <Show when={!repos.loading} fallback={<RepoListSkeleton />}>
                <Show
                  when={!repos.error}
                  fallback={
                    <p class="font-mono text-sm text-red-600">
                      Failed to load repositories: {repos.error?.message ?? ''}
                    </p>
                  }
                >
                  <Show
                    when={repos()!.length > 0}
                    fallback={
                      <p class="font-mono text-sm text-gray-400">
                        No repositories indexed yet.
                      </p>
                    }
                  >
                    <ul class="divide-y divide-gray-100 font-mono text-sm">
                      <For each={repos()}>
                        {(repo) => (
                          <li class="py-3">
                            <div class="flex items-center justify-between gap-4">
                              <span class="font-bold">{repo.repo_id}</span>
                              <span class="text-xs text-gray-400">
                                {repo.snippet_count} snippets
                              </span>
                            </div>
                            <Show when={repo.description}>
                              <p class="mt-1 line-clamp-2 text-sm text-gray-500">
                                {repo.description}
                              </p>
                            </Show>
                          </li>
                        )}
                      </For>
                    </ul>
                  </Show>
                </Show>
              </Show>
            </Show>
          </section>
        </div>
      </main>
    </div>
  );
};

const RepoListSkeleton: Component = () => (
  <ul class="space-y-2">
    <For each={[0, 1, 2]}>
      {() => (
        <li class="h-6 animate-pulse rounded bg-gray-100" />
      )}
    </For>
  </ul>
);

export default Landing;
