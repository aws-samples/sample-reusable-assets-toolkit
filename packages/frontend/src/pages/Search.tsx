// SPDX-License-Identifier: MIT

import type { Component } from 'solid-js';
import {
  createEffect,
  createResource,
  createSignal,
  For,
  on,
  onCleanup,
  Show,
} from 'solid-js';
import { A, useSearchParams } from '@solidjs/router';
import { Header } from '@/components/Header';
import { SearchInput, type SearchMode } from '@/components/SearchInput';
import {
  search,
  searchRepos,
  type RepoSearchResult,
  type SearchResult,
} from '@/lib/rat-api';
import { invokeAgentStream } from '@/lib/agent-api';
import { highlight } from '@/lib/highlight';
import { renderMarkdown } from '@/lib/markdown';
import { parseDescription } from '@/lib/description';

type Data = {
  repos: RepoSearchResult[];
  snippets: SearchResult[];
};

const Search: Component = () => {
  const [params, setParams] = useSearchParams<{ q?: string; mode?: string }>();
  const [query, setQuery] = createSignal(params.q ?? '');
  const mode = (): SearchMode => (params.mode === 'ai' ? 'ai' : 'keyword');
  const [draftMode, setDraftMode] = createSignal<SearchMode>(mode());

  // Sync URL → input (back/forward). Only tracks params.q / params.mode —
  // reading the draft signals inside a plain createEffect would make typing
  // or mode clicks snap back to the URL value.
  createEffect(
    on(
      () => params.q ?? '',
      (q) => setQuery(q),
    ),
  );
  createEffect(
    on(
      () => params.mode ?? '',
      () => setDraftMode(mode()),
    ),
  );

  const [data] = createResource<Data | null, string>(
    () => (mode() === 'keyword' && params.q?.trim()) || undefined,
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
    setParams({ q, mode: draftMode() === 'ai' ? 'ai' : undefined });
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header>
        <SearchInput
          value={query()}
          onInput={setQuery}
          onSubmit={submit}
          mode={draftMode()}
          onModeChange={setDraftMode}
          placeholder={
            draftMode() === 'ai'
              ? 'Ask anything about your code…'
              : 'Search repos, code, docs…'
          }
        />
      </Header>

      <main class="mx-auto w-full max-w-4xl flex-1 px-6 py-8">
        <Show
          when={params.q?.trim()}
          fallback={
            <p class="font-mono text-sm text-gray-400">
              {mode() === 'ai'
                ? 'Ask a question above to start.'
                : 'Enter a query above to search.'}
            </p>
          }
        >
          <Show
            when={mode() === 'ai'}
            fallback={
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
            }
          >
            <AiAnswer query={params.q!.trim()} />
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

const AiAnswer: Component<{ query: string }> = (props) => {
  const [text, setText] = createSignal('');
  const [tools, setTools] = createSignal<string[]>([]);
  const [done, setDone] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  createEffect(
    on(
      () => props.query,
      (q) => {
        setText('');
        setTools([]);
        setDone(false);
        setError(null);
        if (!q) return;
        const ctrl = new AbortController();
        (async () => {
          try {
            for await (const event of invokeAgentStream(q, {
              signal: ctrl.signal,
            })) {
              if (event.type === 'text')
                setText((t) => t + event.content);
              else if (event.type === 'tool_use')
                setTools((t) =>
                  t.includes(event.name) ? t : [...t, event.name],
                );
              else if (event.type === 'complete') setDone(true);
            }
            setDone(true);
          } catch (e) {
            if (!ctrl.signal.aborted) setError((e as Error).message);
          }
        })();
        onCleanup(() => ctrl.abort());
      },
    ),
  );

  return (
    <article class="rounded border border-gray-200">
      <div class="flex items-center justify-between gap-4 border-b border-gray-200 px-3 py-2 font-mono text-xs text-gray-500">
        <span class="uppercase tracking-wider">ai answer</span>
        <Show when={!done() && !error()}>
          <span class="flex items-center gap-1.5">
            <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-gray-900" />
            thinking
          </span>
        </Show>
      </div>

      <Show when={tools().length > 0}>
        <div class="flex flex-wrap gap-1 border-b border-gray-100 px-3 py-2">
          <For each={tools()}>
            {(t) => (
              <span class="rounded border border-gray-200 px-1.5 py-0.5 font-mono text-[10px] text-gray-600">
                {t}
              </span>
            )}
          </For>
        </div>
      </Show>

      <div
        class={
          'px-3 py-3 text-sm leading-relaxed text-gray-900 ' +
          '[&_p]:my-2 [&_p:first-child]:mt-0 [&_p:last-child]:mb-0 ' +
          '[&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5 ' +
          '[&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 ' +
          '[&_li]:my-0.5 ' +
          '[&_h1]:mt-3 [&_h1]:mb-2 [&_h1]:text-lg [&_h1]:font-bold ' +
          '[&_h2]:mt-3 [&_h2]:mb-2 [&_h2]:text-base [&_h2]:font-bold ' +
          '[&_h3]:mt-2 [&_h3]:mb-1 [&_h3]:text-sm [&_h3]:font-bold ' +
          '[&_a]:text-gray-900 [&_a]:underline ' +
          '[&_code]:rounded [&_code]:bg-gray-100 [&_code]:px-1 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-xs ' +
          '[&_pre]:my-2 [&_pre_code]:bg-transparent [&_pre_code]:p-0 ' +
          '[&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-gray-300 [&_blockquote]:pl-3 [&_blockquote]:text-gray-600 ' +
          '[&_hr]:my-3 [&_hr]:border-gray-200 ' +
          '[&_table]:my-2 [&_th]:border-b [&_th]:border-gray-200 [&_th]:px-2 [&_th]:py-1 [&_th]:text-left [&_td]:border-b [&_td]:border-gray-100 [&_td]:px-2 [&_td]:py-1'
        }
      >
        <Show
          when={!error()}
          fallback={
            <span class="text-red-600">Error: {error()}</span>
          }
        >
          <Show
            when={text()}
            fallback={
              <Show when={!done()}>
                <span class="text-gray-400">…</span>
              </Show>
            }
          >
            <div innerHTML={renderMarkdown(text())} />
          </Show>
        </Show>
      </div>
    </article>
  );
};

export default Search;
