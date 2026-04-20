import type { Component } from 'solid-js';
import {
  createEffect,
  createResource,
  createSignal,
  For,
  on,
  Show,
} from 'solid-js';
import { A, useNavigate, useSearchParams } from '@solidjs/router';
import { Header } from '@/components/Header';
import {
  getFile,
  listSnippetsByFile,
  type FileRow,
  type SnippetRow,
} from '@/lib/rat-api';
import { highlight } from '@/lib/highlight';
import { parseDescription } from '@/lib/description';
import { cn } from '@/lib/cn';

type Range = [number, number];

const FilePage: Component = () => {
  const [params] = useSearchParams<{
    repo?: string;
    path?: string;
    start?: string;
    end?: string;
  }>();
  const navigate = useNavigate();

  const [data] = createResource(
    () => {
      const repo = params.repo;
      const path = params.path;
      return repo && path ? { repo, path } : undefined;
    },
    async ({ repo, path }) => {
      const [fileRes, snippetsRes] = await Promise.all([
        getFile(repo, path),
        listSnippetsByFile(repo, path),
      ]);
      return { file: fileRes.file, snippets: snippetsRes.snippets };
    },
  );

  const snippets = (): SnippetRow[] => data()?.snippets ?? [];

  const [hoveredChunkIdx, setHoveredChunkIdx] = createSignal<number | null>(
    null,
  );

  const chunkIdxForLine = (lineNum: number): number =>
    snippets().findIndex(
      (s) =>
        s.start_line != null &&
        s.end_line != null &&
        lineNum >= s.start_line &&
        lineNum <= s.end_line,
    );

  const handleLineHover = (lineNum: number | null): void => {
    if (lineNum === null) {
      setHoveredChunkIdx(null);
      return;
    }
    const idx = chunkIdxForLine(lineNum);
    setHoveredChunkIdx(idx >= 0 ? idx : null);
  };

  const handleLineClick = (lineNum: number): void => {
    const idx = chunkIdxForLine(lineNum);
    if (idx < 0) return;
    const s = snippets()[idx];
    if (s.start_line == null || s.end_line == null) return;
    const p = new URLSearchParams({
      repo: params.repo ?? '',
      path: params.path ?? '',
      start: String(s.start_line),
      end: String(s.end_line),
    });
    navigate(`/file?${p.toString()}`, { replace: true, scroll: false });
  };

  const urlRange = (): Range | null => {
    const s = Number(params.start);
    const e = Number(params.end);
    return Number.isFinite(s) && Number.isFinite(e) && s > 0 && e >= s
      ? [s, e]
      : null;
  };

  const hoveredRange = (): Range | null => {
    const idx = hoveredChunkIdx();
    if (idx === null) return null;
    const s = snippets()[idx];
    return s && s.start_line != null && s.end_line != null
      ? [s.start_line, s.end_line]
      : null;
  };

  const visualRange = (): Range | null => hoveredRange() ?? urlRange();

  // The chunk the panel should show — selected via URL range.
  const selectedChunk = (): SnippetRow | null => {
    const r = urlRange();
    if (!r) return null;
    return (
      snippets().find(
        (s) => s.start_line === r[0] && s.end_line === r[1],
      ) ?? null
    );
  };

  const closePanel = () => {
    const p = new URLSearchParams({
      repo: params.repo ?? '',
      path: params.path ?? '',
    });
    navigate(`/file?${p.toString()}`, { replace: true, scroll: false });
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header />

      <main class="mx-auto w-full max-w-6xl flex flex-1 gap-6 px-6 py-8">
        <div class="min-w-0 flex-1">
          <Show
            when={params.repo && params.path}
            fallback={
              <p class="font-mono text-sm text-gray-400">
                Missing repo or path parameter.
              </p>
            }
          >
            <div class="mb-4 flex min-w-0 items-center gap-2 font-mono text-xs">
              <A
                href={`/repo?id=${encodeURIComponent(params.repo ?? '')}&path=${encodeURIComponent(params.path ?? '')}`}
                class="flex-none font-bold hover:underline"
              >
                {params.repo}
              </A>
              <span class="text-gray-400">/</span>
              <span class="truncate text-gray-600">{params.path}</span>
            </div>

            <Show when={!data.loading} fallback={<LoadingStripe />}>
              <Show
                when={!data.error}
                fallback={
                  <p class="font-mono text-sm text-red-600">
                    Failed to load: {data.error?.message ?? ''}
                  </p>
                }
              >
                <Show
                  when={data()?.file}
                  fallback={
                    <p class="font-mono text-sm text-gray-400">
                      File not found.
                    </p>
                  }
                >
                  <FileView
                    file={data()!.file!}
                    highlightRange={visualRange()}
                    scrollRange={urlRange()}
                    onLineHover={handleLineHover}
                    onLineClick={handleLineClick}
                    isLineClickable={(n) => chunkIdxForLine(n) >= 0}
                  />
                </Show>
              </Show>
            </Show>
          </Show>
        </div>

        <Show when={selectedChunk()}>
          <ChunkPanel snippet={selectedChunk()!} onClose={closePanel} />
        </Show>
      </main>
    </div>
  );
};

const ChunkPanel: Component<{
  snippet: SnippetRow;
  onClose: () => void;
}> = (props) => {
  const parsed = () => parseDescription(props.snippet.description);

  return (
    <aside class="sticky top-20 max-h-[calc(100vh-6rem)] w-80 flex-none self-start overflow-auto rounded border border-gray-200">
      <div class="flex items-center justify-between gap-3 border-b border-gray-200 px-3 py-2 font-mono text-xs">
        <Show
          when={
            props.snippet.start_line != null &&
            props.snippet.end_line != null
          }
          fallback={<span class="text-gray-400">chunk</span>}
        >
          <span class="font-bold">
            L{props.snippet.start_line}–{props.snippet.end_line}
          </span>
        </Show>
        <button
          type="button"
          onClick={props.onClose}
          class="text-gray-400 hover:text-gray-900"
          aria-label="Close"
        >
          ×
        </button>
      </div>

      <div class="space-y-4 px-3 py-3">
        <Show when={props.snippet.symbol_name}>
          <div class="font-mono text-sm font-semibold">
            {props.snippet.symbol_name}
          </div>
        </Show>

        <Show when={parsed().summary}>
          <p class="text-sm italic text-gray-600">{parsed().summary}</p>
        </Show>

        <Show when={parsed().keywords.length > 0}>
          <section class="space-y-1">
            <h3 class="text-[10px] uppercase tracking-wider text-gray-500">
              keywords
            </h3>
            <div class="flex flex-wrap gap-1">
              <For each={parsed().keywords}>
                {(kw) => (
                  <span class="rounded border border-gray-200 px-1.5 py-0.5 font-mono text-[10px] text-gray-600">
                    {kw}
                  </span>
                )}
              </For>
            </div>
          </section>
        </Show>
      </div>
    </aside>
  );
};

const FileView: Component<{
  file: FileRow;
  highlightRange: Range | null;
  scrollRange: Range | null;
  onLineHover: (lineNum: number | null) => void;
  onLineClick: (lineNum: number) => void;
  isLineClickable: (lineNum: number) => boolean;
}> = (props) => {
  let articleRef: HTMLElement | undefined;

  // Scroll only when arriving at a new file (e.g. from search results), not
  // on subsequent in-file chunk selections.
  createEffect(
    on(
      () => props.file.id,
      () => {
        if (!props.scrollRange) return;
        requestAnimationFrame(() => {
          const el = articleRef?.querySelector('[data-highlight-start]');
          el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
        });
      },
    ),
  );

  const lines = () =>
    highlight(props.file.content, props.file.language).split('\n');

  const inRange = (lineNum: number): boolean => {
    const r = props.highlightRange;
    return !!r && lineNum >= r[0] && lineNum <= r[1];
  };

  const isStart = (lineNum: number): boolean => {
    const r = props.highlightRange;
    return !!r && lineNum === r[0];
  };

  return (
    <article ref={articleRef} class="rounded border border-gray-200">
      <div class="flex items-center justify-between gap-4 border-b border-gray-200 px-3 py-2 font-mono text-xs">
        <span class="truncate text-gray-500">{props.file.source_path}</span>
        <Show when={props.file.language}>
          <span class="flex-none text-gray-400">{props.file.language}</span>
        </Show>
      </div>
      <pre
        class="overflow-auto py-2 font-mono text-xs leading-relaxed text-gray-900"
        onMouseLeave={() => props.onLineHover(null)}
      >
        <code class="block">
          <For each={lines()}>
            {(line, i) => {
              const lineNum = i() + 1;
              const clickable = props.isLineClickable(lineNum);
              return (
                <span
                  class={cn(
                    'block px-3',
                    clickable && 'cursor-pointer',
                    inRange(lineNum) && 'bg-yellow-50',
                  )}
                  data-highlight-start={isStart(lineNum) ? '' : undefined}
                  onMouseEnter={() => props.onLineHover(lineNum)}
                  onClick={() => props.onLineClick(lineNum)}
                  innerHTML={line.length === 0 ? ' ' : line}
                />
              );
            }}
          </For>
        </code>
      </pre>
    </article>
  );
};

const LoadingStripe: Component = () => (
  <div class="h-0.5 w-full animate-pulse bg-gray-200" />
);

export default FilePage;
