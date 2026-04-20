import type { Component } from 'solid-js';
import {
  createEffect,
  createResource,
  createSignal,
  For,
  on,
  Show,
} from 'solid-js';
import { A, useSearchParams } from '@solidjs/router';
import { Header } from '@/components/Header';
import { SearchInput } from '@/components/SearchInput';
import {
  search,
  searchRepos,
  type RepoSearchResult,
  type SearchResult,
} from '@/lib/rat-api';
import { highlight } from '@/lib/highlight';
import { parseDescription } from '@/lib/description';

type Data = {
  repos: RepoSearchResult[];
  snippets: SearchResult[];
};

const Search: Component = () => {
  const [params, setParams] = useSearchParams<{ q?: string }>();
  const [query, setQuery] = createSignal(params.q ?? '');

  // Sync URL → input (back/forward). Only tracks params.q — reading query()
  // inside a plain createEffect would make typing snap back to the URL value.
  createEffect(
    on(
      () => params.q ?? '',
      (q) => setQuery(q),
    ),
  );

  const [data] = createResource<Data | null, string>(
    () => params.q?.trim() || undefined,
    async (q) => {
      const [repoRes, snippetRes] = await Promise.all([
        searchRepos(q, 3),
        search(q, { limit: 30 }),
      ]);
      return {
        repos: repoRes.results,
        snippets: snippetRes.results,
      };
    },
  );

  const submit = () => {
    const q = query().trim();
    if (!q) return;
    setParams({ q });
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header>
        <SearchInput
          value={query()}
          onInput={setQuery}
          onSubmit={submit}
          placeholder="Search repos, code, docs…"
        />
      </Header>

      <main class="mx-auto w-full max-w-4xl flex-1 px-6 py-8">
        <Show
          when={params.q?.trim()}
          fallback={
            <p class="font-mono text-sm text-gray-400">
              Enter a query above to search.
            </p>
          }
        >
          <Show when={!data.loading} fallback={<LoadingStripe />}>
            <Show
              when={!data.error}
              fallback={
                <p class="font-mono text-sm text-red-600">
                  Search failed: {data.error?.message ?? ''}
                </p>
              }
            >
              <Results data={data() ?? null} />
            </Show>
          </Show>
        </Show>
      </main>
    </div>
  );
};

const Results: Component<{ data: Data | null }> = (props) => {
  const isEmpty = () => {
    const d = props.data;
    return !d || (d.repos.length === 0 && d.snippets.length === 0);
  };

  return (
    <Show
      when={!isEmpty()}
      fallback={<p class="font-mono text-sm text-gray-400">No matches.</p>}
    >
      <div class="space-y-10">
        <Show when={(props.data?.repos.length ?? 0) > 0}>
          <ReposSection repos={props.data!.repos} />
        </Show>
        <Show when={(props.data?.snippets.length ?? 0) > 0}>
          <SnippetsSection snippets={props.data!.snippets} />
        </Show>
      </div>
    </Show>
  );
};


const fileHref = (r: SearchResult): string => {
  let url = `/file?repo=${encodeURIComponent(r.repo_id)}&path=${encodeURIComponent(r.source_path)}`;
  if (r.start_line != null && r.end_line != null) {
    url += `&start=${r.start_line}&end=${r.end_line}`;
  }
  return url;
};

const BAR_CELLS = 8;

const relevanceBar = (relative: number): string => {
  const filled = Math.round(relative * BAR_CELLS);
  return '█'.repeat(filled) + '░'.repeat(BAR_CELLS - filled);
};

const ReposSection: Component<{ repos: RepoSearchResult[] }> = (props) => {
  // Min-max normalise so that tiny absolute score differences span the full
  // 0-1 range visually. All-equal scores fall back to 1 (all full bars).
  const range = () => {
    const scores = props.repos.map((r) => r.score);
    const top = Math.max(...scores);
    const bottom = Math.min(...scores);
    return { top, bottom, spread: top - bottom };
  };
  const relativeOf = (score: number) => {
    const { bottom, spread } = range();
    return spread > 0 ? (score - bottom) / spread : 1;
  };

  return (
    <section class="space-y-3">
      <h2 class="text-xs uppercase tracking-wider text-gray-500">
        repositories · {props.repos.length}
      </h2>
      <ul class="divide-y divide-gray-100 font-mono text-sm">
        <For each={props.repos}>
          {(repo) => {
            const relative = () => relativeOf(repo.score);
            return (
              <li class="py-2.5">
                <div class="flex items-center justify-between gap-4">
                  <div class="flex min-w-0 items-center gap-3">
                    <span
                      class="select-none font-mono text-xs tracking-tighter text-gray-700"
                      aria-hidden="true"
                    >
                      {relevanceBar(relative())}
                    </span>
                    <A
                      href={`/repo?id=${encodeURIComponent(repo.repo_id)}`}
                      class="truncate font-bold hover:underline"
                    >
                      {repo.repo_id}
                    </A>
                  </div>
                  <span class="flex-none text-xs text-gray-400">
                    {repo.snippet_count} snippets
                  </span>
                </div>
                <Show when={repo.description}>
                  <p class="mt-1 text-sm text-gray-500">
                    {repo.description}
                  </p>
                </Show>
              </li>
            );
          }}
        </For>
      </ul>
    </section>
  );
};

const SnippetsSection: Component<{ snippets: SearchResult[] }> = (props) => (
  <section class="space-y-3">
    <h2 class="text-xs uppercase tracking-wider text-gray-500">
      snippets · {props.snippets.length}
    </h2>
    <div class="space-y-4">
      <For each={props.snippets}>{(r) => <ResultCard result={r} />}</For>
    </div>
  </section>
);

const LoadingStripe: Component = () => (
  <div class="h-0.5 w-full animate-pulse bg-gray-200" />
);

const ResultCard: Component<{ result: SearchResult }> = (props) => {
  const isCode = () => props.result.source_type === 'code';
  const parsed = () => parseDescription(props.result.description);

  return (
    <article class="rounded border border-gray-200">
      {/* Header */}
      <div class="flex items-center justify-between gap-4 border-b border-gray-200 px-3 py-2 font-mono text-xs">
        <div class="flex min-w-0 items-center gap-2">
          <span class="flex-none font-bold">{props.result.repo_id}</span>
          <A
            href={fileHref(props.result)}
            class="truncate text-gray-500 hover:text-gray-900 hover:underline"
          >
            {props.result.source_path}
          </A>
        </div>
        <div class="flex flex-none items-center gap-3 text-gray-400">
          <Show when={props.result.language}>
            <span>{props.result.language}</span>
          </Show>
          <Show when={props.result.start_line !== null}>
            <span>
              L{props.result.start_line}–{props.result.end_line}
            </span>
          </Show>
        </div>
      </div>

      {/* Description */}
      <Show when={parsed().summary}>
        <p class="border-b border-gray-100 px-3 py-2 text-xs italic text-gray-500">
          {parsed().summary}
        </p>
      </Show>

      {/* Keyword tags */}
      <Show when={parsed().keywords.length > 0}>
        <div class="flex flex-wrap gap-1 border-b border-gray-100 px-3 py-2">
          <For each={parsed().keywords}>
            {(kw) => (
              <span class="rounded border border-gray-200 px-1.5 py-0.5 font-mono text-[10px] text-gray-600">
                {kw}
              </span>
            )}
          </For>
        </div>
      </Show>

      {/* Body */}
      <Show
        when={isCode()}
        fallback={
          <div class="border-l-2 border-gray-300 px-3 py-2 text-sm text-gray-700">
            <div class="whitespace-pre-wrap">{props.result.content}</div>
          </div>
        }
      >
        <pre class="overflow-auto px-3 py-2 font-mono text-xs leading-relaxed text-gray-900">
          <code
            innerHTML={highlight(
              props.result.content,
              props.result.language,
            )}
          />
        </pre>
      </Show>
    </article>
  );
};

export default Search;
