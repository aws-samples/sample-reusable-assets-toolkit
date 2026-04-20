import type { Component } from 'solid-js';
import { createResource, createSignal, For, Show } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { useAuth } from 'oidc-provider-solid';
import { Header } from '@/components/Header';
import { SearchInput } from '@/components/SearchInput';
import { listRepos } from '@/lib/rat-api';

const Landing: Component = () => {
  const { isAuthenticated } = useAuth();
  const navigate = useNavigate();
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
    navigate(`/search?q=${encodeURIComponent(q)}`);
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header />

      <main class="flex flex-1 items-start justify-center px-6 py-24">
        <div class="w-full max-w-2xl space-y-12">
          <SearchInput
            value={query()}
            onInput={setQuery}
            onSubmit={submit}
            autofocus
            class="text-base"
          />

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
                              <A
                                href={`/repo?id=${encodeURIComponent(repo.repo_id)}`}
                                class="font-bold hover:underline"
                              >
                                {repo.repo_id}
                              </A>
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
      {() => <li class="h-6 animate-pulse rounded bg-gray-100" />}
    </For>
  </ul>
);

export default Landing;
