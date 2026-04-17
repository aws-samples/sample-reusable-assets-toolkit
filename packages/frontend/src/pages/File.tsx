import type { Component } from 'solid-js';
import { createResource, onMount, Show } from 'solid-js';
import { useSearchParams } from '@solidjs/router';
import { Header } from '@/components/Header';
import { getFile, type FileRow } from '@/lib/rat-api';
import { highlight } from '@/lib/highlight';

const FilePage: Component = () => {
  const [params] = useSearchParams<{
    repo?: string;
    path?: string;
    start?: string;
    end?: string;
  }>();

  const [file] = createResource(
    () => {
      const repo = params.repo;
      const path = params.path;
      return repo && path ? { repo, path } : undefined;
    },
    async ({ repo, path }) => (await getFile(repo, path)).file,
  );

  const highlightRange = (): [number, number] | null => {
    const s = Number(params.start);
    const e = Number(params.end);
    return Number.isFinite(s) && Number.isFinite(e) && s > 0 && e >= s
      ? [s, e]
      : null;
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header />

      <main class="mx-auto w-full max-w-5xl flex-1 px-6 py-8">
        <Show
          when={params.repo && params.path}
          fallback={
            <p class="font-mono text-sm text-gray-400">
              Missing repo or path parameter.
            </p>
          }
        >
          <div class="mb-4 flex min-w-0 items-center gap-2 font-mono text-xs">
            <span class="flex-none font-bold">{params.repo}</span>
            <span class="text-gray-400">/</span>
            <span class="truncate text-gray-600">{params.path}</span>
          </div>

          <Show when={!file.loading} fallback={<LoadingStripe />}>
            <Show
              when={!file.error}
              fallback={
                <p class="font-mono text-sm text-red-600">
                  Failed to load: {file.error?.message ?? ''}
                </p>
              }
            >
              <Show
                when={file()}
                fallback={
                  <p class="font-mono text-sm text-gray-400">File not found.</p>
                }
              >
                <FileView
                  file={file()!}
                  highlightRange={highlightRange()}
                />
              </Show>
            </Show>
          </Show>
        </Show>
      </main>
    </div>
  );
};

const FileView: Component<{
  file: FileRow;
  highlightRange: [number, number] | null;
}> = (props) => {
  let articleRef: HTMLElement | undefined;

  onMount(() => {
    if (!props.highlightRange) return;
    // Wait for innerHTML to flush, then scroll the first highlighted line
    // into view.
    requestAnimationFrame(() => {
      const el = articleRef?.querySelector('[data-highlight-start]');
      el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    });
  });

  const codeHtml = () => {
    const raw = highlight(props.file.content, props.file.language);
    const lines = raw.split('\n');
    const range = props.highlightRange;
    return lines
      .map((line, i) => {
        const lineNum = i + 1;
        const isHighlighted =
          range && lineNum >= range[0] && lineNum <= range[1];
        const classes = isHighlighted ? 'block px-3 bg-yellow-50' : 'block px-3';
        const dataAttr =
          range && lineNum === range[0] ? ' data-highlight-start' : '';
        // Preserve empty lines
        const content = line.length === 0 ? ' ' : line;
        return `<span class="${classes}"${dataAttr}>${content}</span>`;
      })
      .join('');
  };

  return (
    <article ref={articleRef} class="rounded border border-gray-200">
      <div class="flex items-center justify-between gap-4 border-b border-gray-200 px-3 py-2 font-mono text-xs">
        <span class="truncate text-gray-500">{props.file.source_path}</span>
        <Show when={props.file.language}>
          <span class="flex-none text-gray-400">{props.file.language}</span>
        </Show>
      </div>
      <pre class="overflow-auto py-2 font-mono text-xs leading-relaxed text-gray-900">
        <code class="block" innerHTML={codeHtml()} />
      </pre>
    </article>
  );
};

const LoadingStripe: Component = () => (
  <div class="h-0.5 w-full animate-pulse bg-gray-200" />
);

export default FilePage;
