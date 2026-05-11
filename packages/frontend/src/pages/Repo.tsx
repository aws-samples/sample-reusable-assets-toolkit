// SPDX-License-Identifier: MIT

import type { Component } from 'solid-js';
import { createResource, createSignal, For, onMount, Show } from 'solid-js';
import { A, useSearchParams } from '@solidjs/router';
import { Header } from '@/components/Header';
import {
  getRepo,
  listFiles,
  type FileListRow,
  type RepoRow,
} from '@/lib/rat-api';
import { cn } from '@/lib/cn';

const RepoPage: Component = () => {
  const [params] = useSearchParams<{ id?: string; path?: string }>();

  const [data] = createResource(
    () => params.id,
    async (id) => {
      const [repoRes, filesRes] = await Promise.all([
        getRepo(id),
        listFiles(id),
      ]);
      return { repo: repoRes.repo, files: filesRes.files };
    },
  );

  // Folder paths to pre-expand — all ancestors of params.path.
  const expandedPaths = (): Set<string> => {
    const p = params.path;
    if (!p) return new Set();
    const parts = p.split('/');
    parts.pop(); // strip the file segment
    const acc = new Set<string>();
    for (let i = 1; i <= parts.length; i++) {
      acc.add(parts.slice(0, i).join('/'));
    }
    return acc;
  };

  return (
    <div class="min-h-screen flex flex-col bg-white text-gray-900">
      <Header />

      <main class="mx-auto w-full max-w-4xl flex-1 px-6 py-8">
        <Show
          when={params.id}
          fallback={
            <p class="font-mono text-sm text-gray-400">
              Missing repo id parameter.
            </p>
          }
        >
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
                when={data()?.repo}
                fallback={
                  <p class="font-mono text-sm text-gray-400">
                    Repository not found.
                  </p>
                }
              >
                <RepoDetail
                  repo={data()!.repo!}
                  files={data()!.files}
                  expandedPaths={expandedPaths()}
                  highlightPath={params.path ?? null}
                />
              </Show>
            </Show>
          </Show>
        </Show>
      </main>
    </div>
  );
};

const RepoDetail: Component<{
  repo: RepoRow;
  files: FileListRow[];
  expandedPaths: Set<string>;
  highlightPath: string | null;
}> = (props) => (
  <div class="space-y-8">
    <div>
      <h1 class="font-mono text-xl font-bold">{props.repo.repo_id}</h1>
      <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs text-gray-500">
        <span>branch {props.repo.branch}</span>
        <Show when={props.repo.indexed_commit_id}>
          <span>· commit {props.repo.indexed_commit_id!.slice(0, 8)}</span>
        </Show>
        <span>· {props.repo.file_count} files</span>
        <span>· {props.repo.snippet_count} snippets</span>
      </div>
    </div>

    <Show when={props.repo.description}>
      <p class="text-sm text-gray-700">{props.repo.description}</p>
    </Show>

    <section class="space-y-3">
      <h2 class="text-xs uppercase tracking-wider text-gray-500">
        files · {props.files.length}
      </h2>
      <Show
        when={props.files.length > 0}
        fallback={
          <p class="font-mono text-sm text-gray-400">
            No files indexed in this repository.
          </p>
        }
      >
        <FileTree
          repoId={props.repo.repo_id}
          files={props.files}
          expandedPaths={props.expandedPaths}
          highlightPath={props.highlightPath}
        />
      </Show>
    </section>
  </div>
);

// ── File tree ────────────────────────────────────────────────────────

type TreeNode = {
  name: string;
  path: string;
  children: TreeNode[];
  file?: FileListRow;
};

const buildTree = (files: FileListRow[]): TreeNode => {
  const root: TreeNode = { name: '', path: '', children: [] };
  for (const file of files) {
    const segments = file.source_path.split('/');
    let current = root;
    segments.forEach((segment, i) => {
      const isLeaf = i === segments.length - 1;
      const path = segments.slice(0, i + 1).join('/');
      let node = current.children.find((c) => c.name === segment);
      if (!node) {
        node = { name: segment, path, children: [] };
        if (isLeaf) node.file = file;
        current.children.push(node);
      }
      current = node;
    });
  }
  const sort = (node: TreeNode) => {
    node.children.sort((a, b) => {
      const aDir = !a.file;
      const bDir = !b.file;
      if (aDir !== bDir) return aDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    node.children.forEach(sort);
  };
  sort(root);
  return root;
};

const FileTree: Component<{
  repoId: string;
  files: FileListRow[];
  expandedPaths: Set<string>;
  highlightPath: string | null;
}> = (props) => {
  const root = () => buildTree(props.files);
  return (
    <ul class="font-mono text-sm">
      <For each={root().children}>
        {(node) => (
          <TreeNodeView
            node={node}
            repoId={props.repoId}
            depth={0}
            expandedPaths={props.expandedPaths}
            highlightPath={props.highlightPath}
          />
        )}
      </For>
    </ul>
  );
};

const TreeNodeView: Component<{
  node: TreeNode;
  repoId: string;
  depth: number;
  expandedPaths: Set<string>;
  highlightPath: string | null;
}> = (props) => {
  const isFolder = () => !props.node.file;
  const [expanded, setExpanded] = createSignal(
    isFolder() && props.expandedPaths.has(props.node.path),
  );
  const indent = () => ({ 'padding-left': `${props.depth * 1.25}rem` });
  const isHighlighted = () =>
    !!props.node.file &&
    props.highlightPath === props.node.file.source_path;

  let highlightRef: HTMLAnchorElement | undefined;
  onMount(() => {
    if (!isHighlighted()) return;
    requestAnimationFrame(() => {
      highlightRef?.scrollIntoView({ block: 'center' });
    });
  });

  return (
    <li>
      <Show
        when={isFolder()}
        fallback={
          <A
            ref={highlightRef}
            href={`/file?repo=${encodeURIComponent(props.repoId)}&path=${encodeURIComponent(props.node.file!.source_path)}`}
            class={cn(
              'flex items-center justify-between gap-4 py-0.5 hover:bg-gray-50',
              isHighlighted() && 'bg-yellow-50',
            )}
            style={indent()}
          >
            <span class="flex min-w-0 items-center gap-2 text-gray-700">
              <span class="w-3 flex-none select-none text-transparent">▸</span>
              <span class="truncate">{props.node.name}</span>
            </span>
            <Show when={props.node.file!.language}>
              <span class="flex-none pr-2 text-xs text-gray-400">
                {props.node.file!.language}
              </span>
            </Show>
          </A>
        }
      >
        <button
          type="button"
          onClick={() => setExpanded(!expanded())}
          class="flex w-full items-center gap-2 py-0.5 text-left text-gray-700 hover:bg-gray-50"
          style={indent()}
        >
          <span class="w-3 flex-none select-none text-gray-400">
            {expanded() ? '▾' : '▸'}
          </span>
          <span class="truncate font-semibold">{props.node.name}/</span>
        </button>
        <Show when={expanded()}>
          <ul>
            <For each={props.node.children}>
              {(child) => (
                <TreeNodeView
                  node={child}
                  repoId={props.repoId}
                  depth={props.depth + 1}
                  expandedPaths={props.expandedPaths}
                  highlightPath={props.highlightPath}
                />
              )}
            </For>
          </ul>
        </Show>
      </Show>
    </li>
  );
};

const LoadingStripe: Component = () => (
  <div class="h-0.5 w-full animate-pulse bg-gray-200" />
);

export default RepoPage;
