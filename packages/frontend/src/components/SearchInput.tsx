// SPDX-License-Identifier: MIT

import type { Component } from 'solid-js';
import { Show } from 'solid-js';
import { cn } from '@/lib/cn';

export type SearchMode = 'keyword' | 'ai';

export const SearchInput: Component<{
  value: string;
  onInput: (v: string) => void;
  onSubmit: () => void;
  mode?: SearchMode;
  onModeChange?: (mode: SearchMode) => void;
  placeholder?: string;
  autofocus?: boolean;
  class?: string;
}> = (props) => {
  return (
    <div
      class={cn(
        'flex items-center rounded border border-gray-300 bg-white focus-within:border-gray-900',
        props.class,
      )}
    >
      <Show when={props.onModeChange}>
        <div class="flex flex-none items-center gap-0.5 border-r border-gray-200 p-1">
          <ModeButton
            label="keyword"
            active={props.mode !== 'ai'}
            onClick={() => props.onModeChange?.('keyword')}
          />
          <ModeButton
            label="ai"
            active={props.mode === 'ai'}
            onClick={() => props.onModeChange?.('ai')}
          />
        </div>
      </Show>
      <input
        type="text"
        value={props.value}
        onInput={(e) => props.onInput(e.currentTarget.value)}
        onKeyDown={(e) => e.key === 'Enter' && props.onSubmit()}
        placeholder={props.placeholder ?? 'Search repos, code, docs…'}
        autofocus={props.autofocus}
        class="flex-1 bg-transparent px-4 py-2.5 font-mono text-sm outline-none placeholder:text-gray-400"
      />
    </div>
  );
};

const ModeButton: Component<{
  label: string;
  active: boolean;
  onClick: () => void;
}> = (props) => (
  <button
    type="button"
    onClick={props.onClick}
    class={cn(
      'rounded px-2 py-1 font-mono text-[10px] uppercase tracking-wider transition-colors',
      props.active
        ? 'bg-gray-900 text-white'
        : 'text-gray-500 hover:text-gray-900',
    )}
  >
    {props.label}
  </button>
);
